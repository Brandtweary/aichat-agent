//! REPL wrapper for running interactive AIChat sessions
//! 
//! This module provides [`ReplSession`] and [`ReplBuilder`] for programmatically managing
//! AIChat's interactive REPL (Read-Eval-Print Loop) with custom configurations and agents.
//!
//! ## Overview
//!
//! The REPL provides an interactive chat interface with features including:
//! - Tab autocompletion
//! - Multi-line input support
//! - Command history
//! - Built-in commands (`.help`, `.model`, `.agent`, etc.)
//! - Session management
//! - File and URL input capabilities
//!
//! ## Builder Pattern
//!
//! [`ReplBuilder`] offers a fluent API for configuring REPL sessions:
//! - Start from scratch with `ReplBuilder::new()`
//! - Use existing config with `ReplBuilder::with_config()`
//! - Load specific agents before starting
//!
//! ## Examples
//!
//! ### Basic REPL session
//! ```no_run
//! # use aichat_agent::{ReplBuilder, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! ReplBuilder::new()?
//!     .model("openai:gpt-4o-mini")
//!     .api_key("openai", "sk-...")
//!     .build()
//!     .await?
//!     .run()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### REPL with a custom agent
//! ```no_run
//! # use aichat_agent::{TempConfigBuilder, ReplBuilder, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let config = TempConfigBuilder::new()?
//!     .model("claude:claude-3-5-sonnet-20240620")
//!     .api_key("claude", "sk-ant-...")
//!     .build()
//!     .await?;
//!
//! ReplBuilder::with_config(config)
//!     .agent("coding-assistant")
//!     .build()
//!     .await?
//!     .run()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::{Config, GlobalConfig, Repl as AichatRepl, TempConfigBuilder};
use anyhow::Result;

/// A REPL session that runs AIChat's interactive interface
pub struct ReplSession {
    config: GlobalConfig,
    agent: Option<String>,
}

impl ReplSession {
    /// Create a new REPL session with the given configuration
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::{TempConfigBuilder, ReplSession};
    /// 
    /// let config = TempConfigBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .build()
    ///     .await?;
    /// 
    /// let session = ReplSession::new(config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: GlobalConfig) -> Self {
        Self { 
            config,
            agent: None,
        }
    }
    
    /// Create a new REPL session with a specific agent
    pub fn with_agent(config: GlobalConfig, agent: String) -> Self {
        Self {
            config,
            agent: Some(agent),
        }
    }
    
    /// Get the agent name if one is loaded
    pub fn agent(&self) -> Option<&str> {
        self.agent.as_deref()
    }
    
    /// Run the interactive REPL
    /// 
    /// This starts AIChat's full interactive terminal interface with:
    /// - Command completion
    /// - Syntax highlighting  
    /// - Multi-line editing
    /// - All REPL commands (.model, .agent, etc.)
    pub async fn run(self) -> Result<()> {
        let mut repl = AichatRepl::init(&self.config)?;
        repl.run().await
    }
}

/// Builder for creating REPL sessions with custom configuration
pub struct ReplBuilder {
    temp_builder: Option<TempConfigBuilder>,
    existing_config: Option<GlobalConfig>,
    agent_name: Option<String>,
}

impl ReplBuilder {
    /// Create a new REPL builder with temporary configuration
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::ReplBuilder;
    /// 
    /// let session = ReplBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        Ok(Self {
            temp_builder: Some(TempConfigBuilder::new()?),
            existing_config: None,
            agent_name: None,
        })
    }
    
    /// Create a REPL builder using an existing configuration
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::{TempConfigBuilder, ReplBuilder};
    /// 
    /// let config = TempConfigBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .build()
    ///     .await?;
    /// 
    /// let session = ReplBuilder::with_config(config)
    ///     .agent("math-assistant")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_config(config: GlobalConfig) -> Self {
        Self {
            temp_builder: None,
            existing_config: Some(config),
            agent_name: None,
        }
    }
    
    /// Set the API key for a provider (only works with temp config)
    pub fn api_key(mut self, provider: &str, key: &str) -> Self {
        if let Some(builder) = self.temp_builder.take() {
            self.temp_builder = Some(builder.api_key(provider, key));
        }
        self
    }
    
    /// Set the model (only works with temp config)
    pub fn model(mut self, model: &str) -> Self {
        if let Some(builder) = self.temp_builder.take() {
            self.temp_builder = Some(builder.model(model));
        }
        self
    }
    
    /// Set the agent to load
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::ReplBuilder;
    /// 
    /// let session = ReplBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .agent("math-assistant")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn agent(mut self, agent_name: &str) -> Self {
        // We'll store the agent name and load it during build
        self.agent_name = Some(agent_name.to_string());
        self
    }
    
    /// Build and return the REPL session
    /// 
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use aichat_agent::ReplBuilder;
    /// 
    /// let session = ReplBuilder::new()?
    ///     .model("openai:gpt-4o-mini")
    ///     .api_key("openai", "sk-test-key")
    ///     .agent("math-assistant")
    ///     .build()
    ///     .await?;
    /// 
    /// // session.run().await?; // Start the interactive REPL
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> Result<ReplSession> {
        let agent_name = self.agent_name.clone();
        let config = self.build_config().await?;
        
        // Load agent if specified
        if let Some(agent_name) = agent_name {
            let abort_signal = crate::utils::create_abort_signal();
            Config::use_agent(&config, &agent_name, None, abort_signal).await?;
            Ok(ReplSession::with_agent(config, agent_name))
        } else {
            Ok(ReplSession::new(config))
        }
    }
    
    /// Convenience method to build and run immediately
    pub async fn run(self) -> Result<()> {
        self.build().await?.run().await
    }
    
    /// Internal helper to build the config
    async fn build_config(self) -> Result<GlobalConfig> {
        match (self.temp_builder, self.existing_config) {
            (Some(builder), None) => builder.build().await,
            (None, Some(config)) => Ok(config),
            _ => unreachable!("Invalid state"),
        }
    }
}

/// Extension trait for GlobalConfig to add REPL builder functionality
pub trait ReplBuilderExt {
    /// Create a new REPL builder with this configuration
    fn build_repl(&self) -> ReplBuilder;
}

impl ReplBuilderExt for GlobalConfig {
    fn build_repl(&self) -> ReplBuilder {
        ReplBuilder::with_config(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_new() -> Result<()> {
        let builder = ReplBuilder::new()?;
        assert!(builder.temp_builder.is_some());
        assert!(builder.existing_config.is_none());
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_with_config() -> Result<()> {
        // Create a test config
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        let builder = ReplBuilder::with_config(config.clone());
        assert!(builder.temp_builder.is_none());
        assert!(builder.existing_config.is_some());
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_api_key() -> Result<()> {
        let builder = ReplBuilder::new()?
            .api_key("openai", "sk-test-123")
            .api_key("anthropic", "sk-ant-456");
        
        // Can't directly verify the API keys were set without building
        // but we can check the builder state
        assert!(builder.temp_builder.is_some());
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_model() -> Result<()> {
        let builder = ReplBuilder::new()?
            .model("openai:gpt-4o-mini");
        
        assert!(builder.temp_builder.is_some());
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_build() -> Result<()> {
        let session = ReplBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        // Verify we got a ReplSession
        assert!(session.config.read().model_id == "openai:gpt-4o-mini");
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_session_new() -> Result<()> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        let session = ReplSession::new(config.clone());
        
        // Can't easily verify much about the session without running it
        // but we can check it was created
        assert_eq!(
            session.config.read().model_id,
            config.read().model_id
        );
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_ext() -> Result<()> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        // Test the extension trait
        let builder = config.build_repl();
        assert!(builder.existing_config.is_some());
        assert!(builder.temp_builder.is_none());
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_with_existing_config_ignores_mutations() -> Result<()> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        // When using existing config, api_key and model calls should be ignored
        let builder = ReplBuilder::with_config(config.clone())
            .api_key("anthropic", "sk-new")  // This should be ignored
            .model("claude-3");              // This should be ignored
        
        let session = builder.build().await?;
        
        // Should still have original model
        assert_eq!(session.config.read().model_id, "openai:gpt-4o-mini");
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_build_config_with_temp_builder() -> Result<()> {
        let builder = ReplBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test");
        
        let config = builder.build_config().await?;
        assert_eq!(config.read().model_id, "openai:gpt-4o-mini");
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_build_config_with_existing() -> Result<()> {
        let existing_config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        let builder = ReplBuilder::with_config(existing_config.clone());
        let config = builder.build_config().await?;
        
        // Should be the same config
        assert_eq!(
            config.read().model_id,
            existing_config.read().model_id
        );
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_builder_agent() -> Result<()> {
        let builder = ReplBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .agent("test-agent");
        
        assert!(builder.agent_name.is_some());
        assert_eq!(builder.agent_name.as_ref().unwrap(), "test-agent");
        
        Ok(())
    }
    
    #[tokio::test]
    #[serial]
    async fn test_repl_session_with_agent() -> Result<()> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o-mini")
            .api_key("openai", "sk-test")
            .build()
            .await?;
        
        let session = ReplSession::with_agent(config, "test-agent".to_string());
        
        assert_eq!(session.agent(), Some("test-agent"));
        
        Ok(())
    }
}