//! Temporary configuration management for AIChat
//! 
//! This module provides the [`TempConfigBuilder`] for creating isolated AIChat configurations
//! that don't interfere with the user's global settings. All configurations are created in
//! temporary directories that are automatically cleaned up.
//!
//! ## Overview
//!
//! The [`TempConfigBuilder`] allows you to:
//! - Create configurations from scratch with minimal defaults
//! - Load and modify existing configuration files
//! - Set API keys for various LLM providers
//! - Configure model parameters like temperature
//! - Maintain complete isolation from user settings
//!
//! ## Examples
//!
//! ### Creating a new configuration
//! ```no_run
//! # use aichat_agent::{TempConfigBuilder, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let config = TempConfigBuilder::new()?
//!     .model("openai:gpt-4o-mini")
//!     .api_key("openai", "sk-...")
//!     .temperature(0.7)
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Loading from an existing config file
//! ```no_run
//! # use aichat_agent::{TempConfigBuilder, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let config = TempConfigBuilder::from_file("config.yaml")?
//!     .temperature(0.3)  // Override specific settings
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::{config::WorkingMode, Config, GlobalConfig};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use tempfile::TempDir;
use std::fs;
use std::env;

/// Builder for creating temporary AIChat configurations
/// 
/// This creates configs in temporary directories that are cleaned up
/// when the builder is dropped. Perfect for running isolated AIChat instances.
/// 
/// # How AIChat Config Works
/// 
/// AIChat expects a config.yaml file in the config directory. During init:
/// 1. It loads and deserializes the YAML file
/// 2. It runs setup() which:
///    - Loads environment variables
///    - Loads functions from functions/
///    - Sets up the model
///    - Configures document loaders
pub struct TempConfigBuilder {
    temp_dir: TempDir,
    config_data: serde_json::Value,
}

impl TempConfigBuilder {
    /// Create a new temporary config builder
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::TempConfigBuilder;
    /// 
    /// let config = TempConfigBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .temperature(0.7)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        
        // Start with minimal config that matches AIChat's Config struct
        // Using empty/default values that AIChat will accept
        let config_data = serde_json::json!({
            "model": "",  // Will be set by setup_model() if empty
            "save": false,
            "stream": true,
            "keybindings": "emacs",
            "function_calling": true,
            "clients": []
        });
        
        Ok(Self {
            temp_dir,
            config_data,
        })
    }
    
    /// Create a temporary config builder from an existing config file
    /// 
    /// # Example
    /// ```no_run
    /// use aichat_agent::TempConfigBuilder;
    /// 
    /// let config = TempConfigBuilder::from_file("examples/config.yaml")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        
        // Read and parse the existing config file
        let config_content = fs::read_to_string(config_path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", config_path.as_ref().display()))?;
        
        let config_data: serde_json::Value = serde_yaml::from_str(&config_content)
            .context("Failed to parse config YAML")?;
        
        Ok(Self {
            temp_dir,
            config_data,
        })
    }
    
    /// Set the API key for a specific provider
    pub fn api_key(mut self, provider: &str, key: &str) -> Self {
        // Ensure clients array exists
        if !self.config_data["clients"].is_array() {
            self.config_data["clients"] = serde_json::json!([]);
        }
        
        // Add or update the client config
        let client_config = match provider {
            "openai" => serde_json::json!({
                "type": "openai",
                "api_key": key
            }),
            "anthropic" | "claude" => serde_json::json!({
                "type": "claude", 
                "api_key": key
            }),
            "gemini" => serde_json::json!({
                "type": "gemini",
                "api_key": key
            }),
            _ => serde_json::json!({
                "type": provider,
                "api_key": key
            }),
        };
        
        self.config_data["clients"]
            .as_array_mut()
            .unwrap()
            .push(client_config);
        
        self
    }
    
    /// Set the default model
    pub fn model(mut self, model: &str) -> Self {
        self.config_data["model"] = serde_json::json!(model);
        self
    }
    
    /// Set temperature
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::TempConfigBuilder;
    /// 
    /// let config = TempConfigBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .temperature(0.3)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn temperature(mut self, temp: f64) -> Self {
        self.config_data["temperature"] = serde_json::json!(temp);
        self
    }
    
    /// Set stream mode
    pub fn stream(mut self, stream: bool) -> Self {
        self.config_data["stream"] = serde_json::json!(stream);
        self
    }
    
    /// Enable or disable function calling
    pub fn function_calling(mut self, enabled: bool) -> Self {
        self.config_data["function_calling"] = serde_json::json!(enabled);
        self
    }
    
    /// Set a custom value in the config
    pub fn set(mut self, key: &str, value: serde_json::Value) -> Self {
        self.config_data[key] = value;
        self
    }
    
    /// Get the path to the temporary config directory
    /// 
    /// # Example
    /// ```
    /// use aichat_agent::TempConfigBuilder;
    /// 
    /// let builder = TempConfigBuilder::new()?;
    /// let config_dir = builder.config_dir();
    /// assert!(config_dir.exists());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn config_dir(&self) -> &Path {
        self.temp_dir.path()
    }
    
    /// Build the GlobalConfig instance
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::TempConfigBuilder;
    /// 
    /// let config = TempConfigBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> Result<GlobalConfig> {
        // Write config.yaml
        let config_path = self.temp_dir.path().join("config.yaml");
        let config_content = serde_yaml::to_string(&self.config_data)
            .context("Failed to serialize config")?;
        fs::write(&config_path, config_content)
            .context("Failed to write config.yaml")?;
        
        // Create necessary directories that AIChat expects
        fs::create_dir_all(self.temp_dir.path().join("roles"))?;
        fs::create_dir_all(self.temp_dir.path().join("sessions"))?;
        fs::create_dir_all(self.temp_dir.path().join("rags"))?;
        fs::create_dir_all(self.temp_dir.path().join("functions"))?;
        fs::create_dir_all(self.temp_dir.path().join("functions/bin"))?;
        fs::create_dir_all(self.temp_dir.path().join("agents"))?;
        fs::create_dir_all(self.temp_dir.path().join("macros"))?;
        
        // Create empty functions.json so load_functions() doesn't fail
        let functions_file = self.temp_dir.path().join("functions/functions.json");
        fs::write(&functions_file, "[]")?;
        
        // Set environment variable to use our temp directory
        let config_dir_env = crate::utils::get_env_name("config_dir");
        env::set_var(&config_dir_env, self.temp_dir.path());
        
        // Initialize config using AIChat's standard init
        // This will load the config.yaml we just wrote and run setup()
        let config = Config::init(WorkingMode::Repl, false).await?;
        let global_config = Arc::new(RwLock::new(config));
        
        // Keep the temp directory alive by storing it in a thread-local
        // This ensures it's not deleted while the config is in use
        TEMP_DIRS.with(|dirs| {
            dirs.borrow_mut().push(self.temp_dir);
        });
        
        Ok(global_config)
    }
}

// Thread-local storage for temp directories to keep them alive
thread_local! {
    static TEMP_DIRS: std::cell::RefCell<Vec<TempDir>> = std::cell::RefCell::new(Vec::new());
}

/// Helper to create a GlobalConfig from an existing config directory
pub async fn from_directory(config_dir: &Path) -> Result<GlobalConfig> {
    let config_dir_env = crate::utils::get_env_name("config_dir");
    env::set_var(&config_dir_env, config_dir);
    
    let config = Config::init(WorkingMode::Repl, false).await?;
    Ok(Arc::new(RwLock::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use serial_test::serial;
    
    #[tokio::test]
    #[serial]
    async fn test_temp_config_builder_basic() -> Result<()> {
        let builder = TempConfigBuilder::new()?;
        let config_dir = builder.config_dir().to_path_buf();
        
        // Verify temp directory was created
        assert!(config_dir.exists());
        assert!(config_dir.is_dir());
        
        // Create a config with a valid OpenAI model from models.yaml
        let config = builder
            .model("openai:gpt-4o-mini")  // Use full model ID with provider
            .temperature(0.7)
            .api_key("openai", "test-key")
            .build()
            .await?;
        
        // Verify config values
        assert_eq!(config.read().model_id, "openai:gpt-4o-mini");
        assert_eq!(config.read().temperature, Some(0.7));
        
        // Verify config file was created
        let config_file = config_dir.join("config.yaml");
        assert!(config_file.exists());
        
        // Verify directory structure
        assert!(config_dir.join("roles").exists());
        assert!(config_dir.join("sessions").exists());
        assert!(config_dir.join("rags").exists());
        assert!(config_dir.join("functions").exists());
        assert!(config_dir.join("functions/bin").exists());
        assert!(config_dir.join("agents").exists());
        assert!(config_dir.join("macros").exists());
        
        // Verify functions.json was created
        assert!(config_dir.join("functions/functions.json").exists());
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_multiple_api_keys() -> Result<()> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")  // Set a valid model
            .api_key("openai", "sk-test1")
            .api_key("anthropic", "sk-test2")
            .api_key("gemini", "test3")
            .build()
            .await?;
        
        // Verify clients were added
        let clients = &config.read().clients;
        assert_eq!(clients.len(), 3);
        
        // Note: We can't easily verify the exact client configs without 
        // exposing more internals, but we've verified they were added
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_all_config_options() -> Result<()> {
        let builder = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")  // Use a valid model with provider
            .api_key("openai", "sk-test")  // Need API key for the model
            .temperature(0.5)
            .stream(false)
            .function_calling(false)
            .set("save_session", serde_json::json!(true))
            .set("highlight", serde_json::json!(false));
        
        let config = builder.build().await?;
        
        let cfg = config.read();
        
        assert_eq!(cfg.model_id, "openai:gpt-4o-mini");
        assert_eq!(cfg.temperature, Some(0.5));
        assert!(!cfg.stream);
        assert!(!cfg.function_calling);
        assert_eq!(cfg.save_session, Some(true));
        assert!(!cfg.highlight);
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_config_yaml_content() -> Result<()> {
        let builder = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .temperature(0.8)
            .api_key("openai", "sk-test");
        
        let config_dir = builder.config_dir().to_path_buf();
        let _config = builder.build().await?;
        
        // Read and verify the generated YAML
        let yaml_content = fs::read_to_string(config_dir.join("config.yaml"))?;
        assert!(yaml_content.contains("model: openai:gpt-4o-mini"));
        assert!(yaml_content.contains("temperature: 0.8"));
        assert!(yaml_content.contains("type: openai"));
        assert!(yaml_content.contains("api_key: sk-test"));
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_from_directory() -> Result<()> {
        // First create a config
        let builder = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .temperature(0.9);
        let config_dir = builder.config_dir().to_path_buf();
        let _first_config = builder.build().await?;
        
        // Now load it from the directory
        let second_config = from_directory(&config_dir).await?;
        
        // Verify it loaded correctly
        assert_eq!(second_config.read().model_id, "openai:gpt-4o-mini");
        assert_eq!(second_config.read().temperature, Some(0.9));
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_temp_dir_cleanup() -> Result<()> {
        // We can't easily test automatic cleanup due to thread-local storage,
        // but we can verify the temp dirs are being tracked
        let builder1 = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test1");
        let dir1 = builder1.config_dir().to_path_buf();
        let _config1 = builder1.build().await?;
        
        let builder2 = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test2");
        let dir2 = builder2.config_dir().to_path_buf();
        let _config2 = builder2.build().await?;
        
        // Both directories should exist
        assert!(dir1.exists());
        assert!(dir2.exists());
        assert_ne!(dir1, dir2); // Should be different temp directories
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_empty_config() -> Result<()> {
        // Test minimal config with defaults - need at least one client with a model
        let config = TempConfigBuilder::new()?
            .api_key("openai", "sk-test")
            .model("openai:gpt-4o-mini")
            .build()
            .await?;
        
        // Should have default values
        let cfg = config.read();
        assert!(cfg.stream); // Default is true
        assert_eq!(cfg.keybindings, "emacs");
        assert!(cfg.function_calling); // Default is true
        assert!(!cfg.save); // Default is false
        
        Ok(())
    }
}