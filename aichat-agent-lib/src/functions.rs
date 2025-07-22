//! Native Rust function integration for AIChat
//! 
//! This module provides a system for registering native Rust functions as LLM-callable tools,
//! allowing direct integration of custom logic without external scripts or subprocesses.
//!
//! ## Overview
//!
//! AIChat normally executes functions as external scripts. This module provides:
//! - [`FunctionRegistry`] - Register and manage native Rust functions
//! - [`FunctionsBuilder`] - Fluent API for setting up functions in a config
//! - Automatic generation of wrapper scripts for AIChat compatibility
//!
//! ## Function Signatures
//!
//! Functions must have the signature: `Fn(Value) -> Result<Value>`
//! - Input: JSON value containing function arguments
//! - Output: JSON value with function results
//!
//! ## Examples
//!
//! ### Basic function registration
//! ```no_run
//! # use aichat_agent::{FunctionRegistry, Result};
//! # use serde_json::json;
//! let mut registry = FunctionRegistry::new();
//! 
//! registry.register("add", "Add two numbers", |args| {
//!     let a = args["a"].as_f64().unwrap_or(0.0);
//!     let b = args["b"].as_f64().unwrap_or(0.0);
//!     Ok(json!({ "sum": a + b }))
//! });
//! ```
//!
//! ### Using the builder pattern
//! ```no_run
//! # use aichat_agent::{FunctionsBuilder, Result};
//! # use serde_json::json;
//! # use std::path::Path;
//! # let config_dir = Path::new("/tmp");
//! let functions = FunctionsBuilder::new(config_dir)
//!     .register("greet", "Say hello", |args| {
//!         let name = args["name"].as_str().unwrap_or("World");
//!         Ok(json!({ "message": format!("Hello, {name}!") }))
//!     })
//!     .build()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Implementation Note
//!
//! Currently, this module creates placeholder wrapper scripts that AIChat can discover.
//! Full native function execution requires IPC or another mechanism to bridge between
//! AIChat's subprocess model and our in-process functions.

use crate::{function::{FunctionDeclaration, JsonSchema}, Functions};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// A native Rust function that can be called by the LLM
pub type NativeFunction = Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// Registry for native Rust functions
/// 
/// This allows you to register Rust closures as LLM-callable functions,
/// creating a bridge between AIChat's file-based function system and
/// native Rust code.
pub struct FunctionRegistry {
    functions: HashMap<String, NativeFunction>,
    declarations: Vec<FunctionDeclaration>,
}

impl FunctionRegistry {
    /// Create a new function registry
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::FunctionRegistry;
    /// 
    /// let mut registry = FunctionRegistry::new();
    /// assert_eq!(registry.declarations().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),  
            declarations: Vec::new(),
        }
    }
    
    /// Register a native Rust function
    /// 
    /// # Example
    /// ```no_run
    /// use serde_json::json;
    /// use aichat_agent::FunctionRegistry;
    /// let mut registry = FunctionRegistry::new();
    /// registry.register("hello", "Say hello", |args| {
    ///     let name = args["name"].as_str().unwrap_or("World");
    ///     Ok(json!({ "message": format!("Hello, {name}!") }))
    /// });
    /// ```
    pub fn register<F>(&mut self, name: &str, description: &str, func: F) -> &mut Self 
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        // Store the function implementation
        self.functions.insert(name.to_string(), Arc::new(func));
        
        // Create a function declaration that AIChat understands
        // For now, we'll use a simple object schema that accepts any properties
        let declaration = FunctionDeclaration {
            name: name.to_string(),
            description: description.to_string(),
            parameters: JsonSchema {
                type_value: Some("object".to_string()),
                description: None,
                properties: None,
                items: None,
                any_of: None,
                enum_value: None,
                default: None,
                required: None,
            },
            agent: false,
        };
        self.declarations.push(declaration);
        
        self
    }
    
    /// Register a function with full declaration
    pub fn register_with_declaration<F>(
        &mut self, 
        declaration: FunctionDeclaration,
        func: F,
    ) -> &mut Self
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        self.functions.insert(declaration.name.clone(), Arc::new(func));
        self.declarations.push(declaration);
        self
    }
    
    /// Install functions to the config directory
    /// 
    /// This automatically installs functions to the correct location: config_dir/functions/
    /// Creates the functions.json file that AIChat expects and wrapper scripts.
    /// 
    /// # Example
    /// ```no_run
    /// use aichat_agent::FunctionRegistry;
    /// use serde_json::json;
    /// use std::path::Path;
    /// 
    /// let config_dir = Path::new("/tmp/config");
    /// let mut registry = FunctionRegistry::new();
    /// registry.register("greet", "Say hello", |args| {
    ///     let name = args["name"].as_str().unwrap_or("World");
    ///     Ok(json!({ "message": format!("Hello, {name}!") }))
    /// });
    /// registry.install(config_dir)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn install(&self, config_dir: &Path) -> Result<()> {
        let functions_dir = config_dir.join("functions");
        self.install_to_functions_dir(&functions_dir)
    }
    
    /// Internal method to install to a specific functions directory (for testing)
    fn install_to_functions_dir(&self, functions_dir: &Path) -> Result<()> {
        // Ensure directories exist
        fs::create_dir_all(functions_dir)?;
        let bin_dir = functions_dir.join("bin");
        fs::create_dir_all(&bin_dir)?;
        
        // Write functions.json
        let functions_file = functions_dir.join("functions.json");
        let declarations_json = serde_json::to_string_pretty(&self.declarations)?;
        fs::write(&functions_file, declarations_json)
            .context("Failed to write functions.json")?;
        
        // Create wrapper executables for each function
        for (name, _) in &self.functions {
            self.create_wrapper_executable(&bin_dir, name)?;
        }
        
        Ok(())
    }
    
    /// Create a wrapper executable that calls back into our Rust function
    fn create_wrapper_executable(&self, bin_dir: &Path, name: &str) -> Result<()> {
        // For now, we'll create a simple shell script that calls our binary
        // In a real implementation, this would use IPC or a more sophisticated
        // mechanism to call back into the running Rust process
        
        let wrapper_path = bin_dir.join(name);
        
        #[cfg(unix)]
        {
            let script = format!(
                r#"#!/bin/bash
# Native function wrapper for {name}
# This is a placeholder - in production, this would call back
# into the running Rust process via IPC or similar mechanism

echo '{{"error": "Native function execution not yet implemented"}}'
"#,
                name = name
            );
            
            fs::write(&wrapper_path, script)?;
            
            // Make executable
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wrapper_path, perms)?;
        }
        
        #[cfg(windows)]
        {
            let script = format!(
                r#"@echo off
REM Native function wrapper for {name}
REM This is a placeholder - in production, this would call back
REM into the running Rust process via IPC or similar mechanism

echo {{"error": "Native function execution not yet implemented"}}
"#,
                name = name
            );
            
            let wrapper_path = wrapper_path.with_extension("bat");
            fs::write(&wrapper_path, script)?;
        }
        
        Ok(())
    }
    
    /// Get the function declarations
    pub fn declarations(&self) -> &[FunctionDeclaration] {
        &self.declarations
    }
    
    /// Execute a function by name
    pub fn execute(&self, name: &str, args: Value) -> Result<Value> {
        match self.functions.get(name) {
            Some(func) => func(args),
            None => anyhow::bail!("Function '{}' not found", name),
        }
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for setting up functions in a config directory
pub struct FunctionsBuilder<'a> {
    registry: FunctionRegistry,
    config_dir: &'a Path,
}

impl<'a> FunctionsBuilder<'a> {
    /// Create a new functions builder for a config directory
    /// 
    /// # Example
    /// ```no_run
    /// use aichat_agent::FunctionsBuilder;
    /// use serde_json::json;
    /// use std::path::Path;
    /// 
    /// let config_dir = Path::new("/tmp/config");
    /// let functions = FunctionsBuilder::new(config_dir)
    ///     .register("hello", "Say hello", |args| {
    ///         let name = args["name"].as_str().unwrap_or("World");
    ///         Ok(json!({ "message": format!("Hello, {name}!") }))
    ///     })
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(config_dir: &'a Path) -> Self {
        Self {
            registry: FunctionRegistry::new(),
            config_dir,
        }
    }
    
    pub fn register<F>(mut self, name: &str, description: &str, func: F) -> Self
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        self.registry.register(name, description, func);
        self
    }
    
    pub fn build(self) -> Result<Functions> {
        // Install the functions
        self.registry.install(self.config_dir)?;
        
        // Load them using AIChat's standard mechanism
        let functions_file = self.config_dir.join("functions").join("functions.json");
        Functions::init(&functions_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serde_json::json;
    
    #[test]
    fn test_function_registry_new() {
        let registry = FunctionRegistry::new();
        
        assert!(registry.functions.is_empty());
        assert!(registry.declarations.is_empty());
    }
    
    #[test]
    fn test_function_registry_register() {
        let mut registry = FunctionRegistry::new();
        
        registry.register("test", "Test function", |args| {
            Ok(json!({ "received": args }))
        });
        
        assert_eq!(registry.declarations.len(), 1);
        assert_eq!(registry.declarations[0].name, "test");
        assert_eq!(registry.declarations[0].description, "Test function");
        assert!(!registry.declarations[0].agent);
        
        // Check the schema
        let params = &registry.declarations[0].parameters;
        assert_eq!(params.type_value, Some("object".to_string()));
    }
    
    #[test]
    fn test_function_registry_register_multiple() {
        let mut registry = FunctionRegistry::new();
        
        registry
            .register("func1", "First function", |_| Ok(json!({"result": 1})))
            .register("func2", "Second function", |_| Ok(json!({"result": 2})))
            .register("func3", "Third function", |_| Ok(json!({"result": 3})));
        
        assert_eq!(registry.declarations.len(), 3);
        assert_eq!(registry.functions.len(), 3);
        
        // Verify all functions are registered
        let names: Vec<&str> = registry.declarations.iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, vec!["func1", "func2", "func3"]);
    }
    
    #[test]
    fn test_function_registry_execute() {
        let mut registry = FunctionRegistry::new();
        
        registry.register("add", "Add two numbers", |args| {
            let a = args["a"].as_f64().unwrap_or(0.0);
            let b = args["b"].as_f64().unwrap_or(0.0);
            Ok(json!({ "sum": a + b }))
        });
        
        let result = registry.execute("add", json!({ "a": 5, "b": 3 })).unwrap();
        assert_eq!(result["sum"], 8.0);
    }
    
    #[test]
    fn test_function_registry_execute_not_found() {
        let registry = FunctionRegistry::new();
        
        let result = registry.execute("nonexistent", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
    
    #[test]
    fn test_function_registry_with_declaration() {
        let mut registry = FunctionRegistry::new();
        
        let declaration = FunctionDeclaration {
            name: "custom".to_string(),
            description: "Custom function with full declaration".to_string(),
            parameters: JsonSchema {
                type_value: Some("object".to_string()),
                description: Some("Parameters for custom function".to_string()),
                properties: Some(indexmap::indexmap! {
                    "input".to_string() => JsonSchema {
                        type_value: Some("string".to_string()),
                        description: Some("Input string".to_string()),
                        properties: None,
                        items: None,
                        any_of: None,
                        enum_value: None,
                        default: None,
                        required: None,
                    }
                }),
                items: None,
                any_of: None,
                enum_value: None,
                default: None,
                required: Some(vec!["input".to_string()]),
            },
            agent: false,
        };
        
        registry.register_with_declaration(declaration.clone(), |args| {
            let input = args["input"].as_str().unwrap_or("");
            Ok(json!({ "output": input.to_uppercase() }))
        });
        
        assert_eq!(registry.declarations.len(), 1);
        assert_eq!(registry.declarations[0].name, "custom");
        assert_eq!(registry.declarations[0].parameters.required, Some(vec!["input".to_string()]));
        
        let result = registry.execute("custom", json!({ "input": "hello" })).unwrap();
        assert_eq!(result["output"], "HELLO");
    }
    
    #[test]
    fn test_function_registry_install() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut registry = FunctionRegistry::new();
        
        registry.register("test_func", "Test function", |_| {
            Ok(json!({ "status": "ok" }))
        });
        
        registry.install(temp_dir.path())?;
        
        // Check that functions.json was created in the correct location
        let functions_file = temp_dir.path().join("functions").join("functions.json");
        assert!(functions_file.exists());
        
        // Check that the declarations were written correctly
        let content = fs::read_to_string(&functions_file)?;
        let declarations: Vec<FunctionDeclaration> = serde_json::from_str(&content)?;
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "test_func");
        
        // Check that the bin directory was created
        let bin_dir = temp_dir.path().join("functions").join("bin");
        assert!(bin_dir.exists());
        
        // Check that wrapper executable was created
        #[cfg(unix)]
        let wrapper_path = bin_dir.join("test_func");
        #[cfg(windows)]
        let wrapper_path = bin_dir.join("test_func.bat");
        
        assert!(wrapper_path.exists());
        
        // On Unix, check that it's executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&wrapper_path)?;
            let permissions = metadata.permissions();
            assert_eq!(permissions.mode() & 0o111, 0o111); // Check execute bits
        }
        
        Ok(())
    }
    
    #[test]
    fn test_functions_builder() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        let _functions = FunctionsBuilder::new(temp_dir.path())
            .register("hello", "Say hello", |args| {
                let name = args["name"].as_str().unwrap_or("World");
                Ok(json!({ "message": format!("Hello, {name}!") }))
            })
            .register("goodbye", "Say goodbye", |args| {
                let name = args["name"].as_str().unwrap_or("World");
                Ok(json!({ "message": format!("Goodbye, {name}!") }))
            })
            .build()?;
        
        // Verify files were created in the correct location
        assert!(temp_dir.path().join("functions").join("functions.json").exists());
        assert!(temp_dir.path().join("functions").join("bin").exists());
        
        #[cfg(unix)]
        {
            assert!(temp_dir.path().join("functions").join("bin/hello").exists());
            assert!(temp_dir.path().join("functions").join("bin/goodbye").exists());
        }
        #[cfg(windows)]
        {
            assert!(temp_dir.path().join("functions").join("bin/hello.bat").exists());
            assert!(temp_dir.path().join("functions").join("bin/goodbye.bat").exists());
        }
        
        Ok(())
    }
    
    #[test]
    fn test_default_trait() {
        let registry1 = FunctionRegistry::new();
        let registry2 = FunctionRegistry::default();
        
        assert_eq!(registry1.functions.len(), registry2.functions.len());
        assert_eq!(registry1.declarations.len(), registry2.declarations.len());
    }
    
    #[test]
    fn test_function_error_handling() {
        let mut registry = FunctionRegistry::new();
        
        registry.register("error_func", "Function that errors", |_| {
            anyhow::bail!("This function always fails")
        });
        
        let result = registry.execute("error_func", json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("always fails"));
    }
    
    #[test]
    fn test_declarations_getter() {
        let mut registry = FunctionRegistry::new();
        
        registry
            .register("func1", "First", |_| Ok(json!({})))
            .register("func2", "Second", |_| Ok(json!({})));
        
        let declarations = registry.declarations();
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].name, "func1");
        assert_eq!(declarations[1].name, "func2");
    }
}