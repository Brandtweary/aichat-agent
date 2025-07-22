//! # AIChat Agent Library
//! 
//! A Rust library that provides programmatic access to [AIChat](https://github.com/sigoden/aichat)'s 
//! powerful LLM capabilities, including interactive REPL sessions, custom AI agents, and native function integration.
//!
//! ## Overview
//!
//! This library exposes AIChat's functionality through a clean, idiomatic Rust API while respecting
//! AIChat's file-based configuration system. It enables you to:
//!
//! - Create and manage temporary AIChat configurations
//! - Build custom AI agents with specialized capabilities
//! - Register native Rust functions as LLM tools
//! - Run interactive REPL sessions programmatically
//! - Integrate with 20+ LLM providers through AIChat's unified interface
//!
//! ## Quick Start
//!
//! ```no_run
//! use aichat_agent::{TempConfigBuilder, ReplBuilder, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a temporary configuration
//!     let config = TempConfigBuilder::new()?
//!         .model("openai:gpt-4o-mini")
//!         .api_key("openai", "your-api-key")
//!         .build()
//!         .await?;
//!
//!     // Start an interactive REPL session
//!     ReplBuilder::with_config(config)
//!         .build()
//!         .await?
//!         .run()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Key Components
//!
//! - [`TempConfigBuilder`] - Create isolated AIChat configurations
//! - [`AgentDefinitionBuilder`] - Define custom AI agents with instructions and tools
//! - [`FunctionRegistry`] - Register native Rust functions as LLM-callable tools
//! - [`ReplBuilder`] / [`ReplSession`] - Manage interactive REPL sessions
//!
//! ## Examples
//!
//! See the `examples/` directory for complete working examples, including:
//! - `math_assistant.rs` - A math tutor with calculation functions
//!
//! ## Design Philosophy
//!
//! This library provides a thin wrapper around AIChat's existing functionality,
//! respecting its file-based architecture while offering programmatic convenience.
//! All configurations are temporary and isolated from the user's global AIChat settings.

#[macro_use]
extern crate log;

// Import modules from the parent src directory using path attributes
#[path = "../../src/config/mod.rs"]
pub mod config;

#[path = "../../src/client/mod.rs"]
pub mod client;

#[path = "../../src/function.rs"]
pub mod function;

#[path = "../../src/utils/mod.rs"]
pub mod utils;

#[path = "../../src/render/mod.rs"]
pub mod render;

#[path = "../../src/rag/mod.rs"]
pub mod rag;

#[path = "../../src/repl/mod.rs"]
pub mod repl;

// Don't import CLI-specific modules - they're not needed for library usage

// Re-export core types from config module
pub use config::{Config, GlobalConfig, Input, Role, Agent};

// Re-export client types
pub use client::{Client, ClientConfig, Model, Message, MessageContent, MessageRole};

// Re-export function types
pub use function::{Functions, FunctionDeclaration, ToolCall, ToolResult};

// Re-export useful utilities
pub use utils::{AbortSignal, multiline_text, create_abort_signal};

// Re-export render types for output
pub use render::MarkdownRender;

// Re-export RAG types
pub use rag::Rag;

// Re-export REPL types
pub use repl::{Repl, run_repl_command};

// Our wrapper APIs
pub mod temp_config;
pub mod functions;
pub mod repl_wrapper;
pub mod agents;

pub use temp_config::TempConfigBuilder;
pub use functions::{FunctionRegistry, FunctionsBuilder, NativeFunction};
pub use repl_wrapper::{ReplSession, ReplBuilder, ReplBuilderExt};
pub use agents::{AgentDefinition, AgentDefinitionBuilder, AgentVariable, AgentFunctionsBuilder};

// Prelude for convenience imports
pub mod prelude {
    pub use crate::{
        Config, GlobalConfig,
        Agent,
        Input,
        Client, Model,
        Message, MessageRole,
        FunctionDeclaration, ToolCall, ToolResult,
        AbortSignal,
        Repl, run_repl_command,
    };
}

// Re-export anyhow for error handling
pub use anyhow::{Result, Error};