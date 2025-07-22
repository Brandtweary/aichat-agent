//! Agent definition builders for programmatically creating AIChat agents
//! 
//! This module provides [`AgentDefinitionBuilder`] for creating custom AI agents with
//! specialized instructions, tools, and behaviors without manually writing YAML files.
//!
//! ## What are Agents?
//!
//! Agents in AIChat are specialized AI assistants that combine:
//! - **Instructions** - System prompts that define behavior and personality
//! - **Tools** - Functions the agent can call to perform actions
//! - **Documents** - RAG sources for knowledge augmentation
//! - **Variables** - Dynamic parameters for customization
//!
//! ## Builder Pattern
//!
//! [`AgentDefinitionBuilder`] provides a fluent API for agent creation:
//! - Set instructions and personality
//! - Add conversation starters
//! - Configure agent-specific functions
//! - Enable dynamic instructions
//!
//! ## Examples
//!
//! ### Basic agent
//! ```no_run
//! # use aichat_agent::{AgentDefinitionBuilder, Result};
//! # use std::path::Path;
//! # let config_dir = Path::new("/tmp");
//! let agent = AgentDefinitionBuilder::new("helpful-assistant")
//!     .description("A general-purpose AI assistant")
//!     .instructions("You are a helpful, friendly AI assistant.")
//!     .add_starter("How can I help you today?")
//!     .save_to(config_dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Specialized agent with tools
//! ```no_run
//! # use aichat_agent::{AgentDefinitionBuilder, Result};
//! # use std::path::Path;
//! # let config_dir = Path::new("/tmp");
//! let agent = AgentDefinitionBuilder::new("research-assistant")
//!     .description("An AI that helps with research tasks")
//!     .version("1.0.0")
//!     .instructions(r#"You are a research assistant that helps users
//! find, analyze, and summarize information. Always cite sources
//! and think critically about the information you provide."#)
//!     .add_starter("What topic would you like to research?")
//!     .add_starter("I can help you find academic papers on any subject.")
//!     .add_document("knowledge-base.pdf")
//!     .save_to(config_dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## File Structure
//!
//! Agents are saved to `{config_dir}/functions/agents/{agent-name}/` with:
//! - `index.yaml` - Agent definition
//! - `functions.json` - Agent-specific functions (if any)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// An agent definition that can be saved to index.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub dynamic_instructions: bool,
    #[serde(default)]
    pub variables: Vec<AgentVariable>,
    #[serde(default)]
    pub conversation_starters: Vec<String>,
    #[serde(default)]
    pub documents: Vec<String>,
}

/// A variable that can be used in agent templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVariable {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Builder for creating agent definitions
pub struct AgentDefinitionBuilder {
    definition: AgentDefinition,
}

impl AgentDefinitionBuilder {
    /// Create a new agent definition builder
    /// 
    /// # Example
    /// ```no_run
    /// use aichat_agent::AgentDefinitionBuilder;
    /// use std::path::Path;
    /// 
    /// let config_dir = Path::new("/tmp/config");
    /// let agent = AgentDefinitionBuilder::new("my-assistant")
    ///     .description("A helpful assistant")
    ///     .instructions("You are a helpful AI assistant.")
    ///     .add_starter("How can I help you today?")
    ///     .save_to(config_dir)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            definition: AgentDefinition {
                name: name.into(),
                description: String::new(),
                version: "0.1.0".to_string(),
                instructions: String::new(),
                dynamic_instructions: false,
                variables: Vec::new(),
                conversation_starters: Vec::new(),
                documents: Vec::new(),
            },
        }
    }
    
    /// Set the agent description
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::AgentDefinitionBuilder;
    /// 
    /// let agent = AgentDefinitionBuilder::new("my-agent")
    ///     .description("A helpful AI assistant")
    ///     .build();
    /// 
    /// assert_eq!(agent.description, "A helpful AI assistant");
    /// ```
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.definition.description = desc.into();
        self
    }
    
    /// Set the agent version
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::AgentDefinitionBuilder;
    /// 
    /// let agent = AgentDefinitionBuilder::new("my-agent")
    ///     .version("1.0.0")
    ///     .build();
    /// 
    /// assert_eq!(agent.version, "1.0.0");
    /// ```
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.definition.version = version.into();
        self
    }
    
    /// Set the agent instructions (system prompt)
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::AgentDefinitionBuilder;
    /// 
    /// let agent = AgentDefinitionBuilder::new("my-agent")
    ///     .instructions("You are a helpful AI assistant.")
    ///     .build();
    /// 
    /// assert_eq!(agent.instructions, "You are a helpful AI assistant.");
    /// ```
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.definition.instructions = instructions.into();
        self
    }
    
    /// Enable dynamic instructions (allows runtime modification)
    pub fn dynamic_instructions(mut self, enabled: bool) -> Self {
        self.definition.dynamic_instructions = enabled;
        self
    }
    
    /// Add a variable that can be used in templates
    pub fn add_variable(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.definition.variables.push(AgentVariable {
            name: name.into(),
            description: description.into(),
            default: None,
        });
        self
    }
    
    /// Add a variable with a default value
    pub fn add_variable_with_default(
        mut self, 
        name: impl Into<String>, 
        description: impl Into<String>,
        default: impl Into<String>
    ) -> Self {
        self.definition.variables.push(AgentVariable {
            name: name.into(),
            description: description.into(),
            default: Some(default.into()),
        });
        self
    }
    
    /// Add a conversation starter
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::AgentDefinitionBuilder;
    /// 
    /// let agent = AgentDefinitionBuilder::new("my-agent")
    ///     .add_starter("How can I help you today?")
    ///     .add_starter("What would you like to know?")
    ///     .build();
    /// 
    /// assert_eq!(agent.conversation_starters.len(), 2);
    /// assert_eq!(agent.conversation_starters[0], "How can I help you today?");
    /// ```
    pub fn add_starter(mut self, starter: impl Into<String>) -> Self {
        self.definition.conversation_starters.push(starter.into());
        self
    }
    
    /// Add a document path for RAG
    pub fn add_document(mut self, path: impl Into<String>) -> Self {
        self.definition.documents.push(path.into());
        self
    }
    
    /// Build and return the agent definition
    pub fn build(self) -> AgentDefinition {
        self.definition
    }
    
    /// Save the agent definition to the config directory
    /// 
    /// This automatically places the agent in the correct location: config_dir/functions/agents/{name}/
    /// 
    /// This creates the agent directory structure:
    /// ```text
    /// <config_dir>/functions/agents/<agent-name>/
    ///   ├── index.yaml
    ///   └── functions.json (if functions provided)
    /// ```
    /// 
    /// # Example
    /// ```no_run
    /// use aichat_agent::AgentDefinitionBuilder;
    /// use std::path::Path;
    /// 
    /// let config_dir = Path::new("/tmp/config");
    /// let agent = AgentDefinitionBuilder::new("my-assistant")
    ///     .description("A helpful assistant")
    ///     .instructions("You are a helpful AI assistant.")
    ///     .add_starter("How can I help you today?")
    ///     .save_to(config_dir)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn save_to(self, config_dir: &Path) -> Result<AgentDefinition> {
        let agents_dir = config_dir.join("functions").join("agents");
        self.save_to_internal(&agents_dir)
    }
    
    /// Internal method to save to a specific agents directory (for testing)
    fn save_to_internal(self, agents_dir: &Path) -> Result<AgentDefinition> {
        let agent_dir = agents_dir.join(&self.definition.name);
        fs::create_dir_all(&agent_dir)
            .with_context(|| format!("Failed to create agent directory: {}", agent_dir.display()))?;
        
        // Write index.yaml
        let index_path = agent_dir.join("index.yaml");
        let yaml_content = serde_yaml::to_string(&self.definition)
            .context("Failed to serialize agent definition")?;
        fs::write(&index_path, yaml_content)
            .with_context(|| format!("Failed to write index.yaml: {}", index_path.display()))?;
        
        // Create empty functions.json if it doesn't exist
        let functions_path = agent_dir.join("functions.json");
        if !functions_path.exists() {
            fs::write(&functions_path, "[]")
                .with_context(|| format!("Failed to write functions.json: {}", functions_path.display()))?;
        }
        
        Ok(self.definition)
    }
}

/// Helper to create agent-specific functions
pub struct AgentFunctionsBuilder {
    agent_name: String,
    functions: Vec<crate::function::FunctionDeclaration>,
}

impl AgentFunctionsBuilder {
    /// Create a new functions builder for an agent
    pub fn new(agent_name: impl Into<String>) -> Self {
        Self {
            agent_name: agent_name.into(),
            functions: Vec::new(),
        }
    }
    
    /// Add a function declaration
    pub fn add_function(mut self, mut declaration: crate::function::FunctionDeclaration) -> Self {
        // Mark as agent-specific function
        declaration.agent = true;
        self.functions.push(declaration);
        self
    }
    
    /// Save the functions to the config directory
    /// This automatically places the functions in the correct location: config_dir/functions/agents/{name}/functions.json
    pub fn save_to(self, config_dir: &Path) -> Result<()> {
        let functions_path = config_dir
            .join("functions")
            .join("agents")
            .join(&self.agent_name)
            .join("functions.json");
        
        let json_content = serde_json::to_string_pretty(&self.functions)
            .context("Failed to serialize functions")?;
        fs::write(&functions_path, json_content)
            .with_context(|| format!("Failed to write functions.json: {}", functions_path.display()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_agent_definition_builder_basic() {
        let agent = AgentDefinitionBuilder::new("test-agent")
            .build();
        
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.description, "");
        assert_eq!(agent.version, "0.1.0");
        assert_eq!(agent.instructions, "");
        assert!(!agent.dynamic_instructions);
        assert!(agent.variables.is_empty());
        assert!(agent.conversation_starters.is_empty());
        assert!(agent.documents.is_empty());
    }
    
    #[test]
    fn test_agent_definition_builder_full() {
        let agent = AgentDefinitionBuilder::new("test-agent")
            .description("A test agent")
            .version("1.0.0")
            .instructions("You are a test assistant")
            .dynamic_instructions(true)
            .add_variable("api_key", "API key for external service")
            .add_variable_with_default("model", "Model to use", "gpt-4")
            .add_starter("How can I help you test?")
            .add_starter("What would you like to know?")
            .add_document("docs/manual.pdf")
            .add_document("https://example.com/guide")
            .build();
        
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.description, "A test agent");
        assert_eq!(agent.version, "1.0.0");
        assert_eq!(agent.instructions, "You are a test assistant");
        assert!(agent.dynamic_instructions);
        assert_eq!(agent.variables.len(), 2);
        assert_eq!(agent.conversation_starters.len(), 2);
        assert_eq!(agent.documents.len(), 2);
        
        // Check variables
        assert_eq!(agent.variables[0].name, "api_key");
        assert_eq!(agent.variables[0].description, "API key for external service");
        assert!(agent.variables[0].default.is_none());
        
        assert_eq!(agent.variables[1].name, "model");
        assert_eq!(agent.variables[1].description, "Model to use");
        assert_eq!(agent.variables[1].default.as_ref().unwrap(), "gpt-4");
    }
    
    #[test]
    fn test_save_agent() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        let agent = AgentDefinitionBuilder::new("test-agent")
            .description("A test agent")
            .instructions("You are a test assistant")
            .save_to(temp_dir.path())?;
        
        // Verify files were created in the correct location: functions/agents/test-agent/
        let agent_dir = temp_dir.path().join("functions").join("agents").join(&agent.name);
        assert!(agent_dir.exists());
        assert!(agent_dir.is_dir());
        assert!(agent_dir.join("index.yaml").exists());
        assert!(agent_dir.join("functions.json").exists());
        
        // Verify index.yaml content
        let yaml_content = fs::read_to_string(agent_dir.join("index.yaml"))?;
        assert!(yaml_content.contains("name: test-agent"));
        assert!(yaml_content.contains("description: A test agent"));
        assert!(yaml_content.contains("instructions: You are a test assistant"));
        
        // Verify functions.json is empty array
        let functions_content = fs::read_to_string(agent_dir.join("functions.json"))?;
        assert_eq!(functions_content.trim(), "[]");
        
        Ok(())
    }
    
    #[test]
    fn test_save_agent_complex() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        let _agent = AgentDefinitionBuilder::new("complex-agent")
            .description("A complex test agent")
            .version("2.0.0")
            .instructions("You are a complex assistant")
            .dynamic_instructions(true)
            .add_variable("key1", "First key")
            .add_variable_with_default("key2", "Second key", "default-value")
            .add_starter("Hello!")
            .add_document("test.pdf")
            .save_to(temp_dir.path())?;
        
        let yaml_content = fs::read_to_string(
            temp_dir.path().join("functions").join("agents").join("complex-agent").join("index.yaml")
        )?;
        
        // Verify all fields are saved
        assert!(yaml_content.contains("name: complex-agent"));
        assert!(yaml_content.contains("version: 2.0.0"));
        assert!(yaml_content.contains("dynamic_instructions: true"));
        assert!(yaml_content.contains("key1"));
        assert!(yaml_content.contains("key2"));
        assert!(yaml_content.contains("default-value"));
        assert!(yaml_content.contains("Hello!"));
        assert!(yaml_content.contains("test.pdf"));
        
        Ok(())
    }
    
    #[test]
    fn test_agent_functions_builder() {
        let builder = AgentFunctionsBuilder::new("test-agent");
        
        assert_eq!(builder.agent_name, "test-agent");
        assert!(builder.functions.is_empty());
    }
    
    #[test]
    fn test_agent_functions_builder_add_function() {
        use crate::function::FunctionDeclaration;
        
        let func = FunctionDeclaration {
            name: "test_func".to_string(),
            description: "Test function".to_string(),
            parameters: crate::function::JsonSchema {
                type_value: Some("object".to_string()),
                description: None,
                properties: Some(indexmap::IndexMap::new()),
                items: None,
                any_of: None,
                enum_value: None,
                default: None,
                required: None,
            },
            agent: false,
        };
        
        let builder = AgentFunctionsBuilder::new("test-agent")
            .add_function(func);
        
        assert_eq!(builder.functions.len(), 1);
        assert!(builder.functions[0].agent); // Should be marked as agent-specific
        assert_eq!(builder.functions[0].name, "test_func");
    }
    
    #[test]
    fn test_agent_functions_save() -> Result<()> {
        use crate::function::FunctionDeclaration;
        
        let temp_dir = TempDir::new()?;
        
        // First create the agent
        AgentDefinitionBuilder::new("test-agent")
            .save_to(temp_dir.path())?;
        
        // Now add functions
        let func = FunctionDeclaration {
            name: "test_func".to_string(),
            description: "Test function".to_string(),
            parameters: crate::function::JsonSchema {
                type_value: Some("object".to_string()),
                description: None,
                properties: Some(indexmap::IndexMap::new()),
                items: None,
                any_of: None,
                enum_value: None,
                default: None,
                required: None,
            },
            agent: false,
        };
        
        AgentFunctionsBuilder::new("test-agent")
            .add_function(func)
            .save_to(temp_dir.path())?;
        
        // Verify functions.json was updated
        let functions_path = temp_dir.path()
            .join("functions")
            .join("agents")
            .join("test-agent")
            .join("functions.json");
        
        let content = fs::read_to_string(functions_path)?;
        let functions: Vec<FunctionDeclaration> = serde_json::from_str(&content)?;
        
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "test_func");
        // Note: The agent field is marked as skip_serializing in AIChat,
        // so it won't be preserved when saving/loading from JSON
        
        Ok(())
    }
    
    #[test]
    fn test_agent_variable_serialization() {
        let var = AgentVariable {
            name: "test".to_string(),
            description: "Test variable".to_string(),
            default: Some("default".to_string()),
        };
        
        let yaml = serde_yaml::to_string(&var).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("description: Test variable"));
        assert!(yaml.contains("default: default"));
        
        // Test without default
        let var_no_default = AgentVariable {
            name: "test2".to_string(),
            description: "Test variable 2".to_string(),
            default: None,
        };
        
        let yaml2 = serde_yaml::to_string(&var_no_default).unwrap();
        assert!(!yaml2.contains("default:"));
    }
    
    #[test]
    fn test_agent_definition_serialization() {
        let agent = AgentDefinition {
            name: "test".to_string(),
            description: "Test agent".to_string(),
            version: "1.0.0".to_string(),
            instructions: "Instructions".to_string(),
            dynamic_instructions: false,
            variables: vec![],
            conversation_starters: vec![],
            documents: vec![],
        };
        
        let yaml = serde_yaml::to_string(&agent).unwrap();
        let deserialized: AgentDefinition = serde_yaml::from_str(&yaml).unwrap();
        
        assert_eq!(agent.name, deserialized.name);
        assert_eq!(agent.description, deserialized.description);
        assert_eq!(agent.version, deserialized.version);
        assert_eq!(agent.instructions, deserialized.instructions);
    }
}