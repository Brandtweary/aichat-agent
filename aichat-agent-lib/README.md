# AIChat Agent Library

A Rust library that converts [AIChat](https://github.com/sigoden/aichat) into a reusable library for building AI applications with custom agents and functions.

## Overview

This library provides a clean, programmatic interface to AIChat's powerful LLM capabilities. Create temporary configurations, define custom agents, register native Rust functions, and run interactive chat sessions - all programmatically.

## Features

- **Temporary Configurations**: Isolated, temporary AIChat configs that don't interfere with user settings
- **Agent Creation**: Programmatically define AI agents with custom instructions and capabilities  
- **Native Functions**: Register Rust functions that the AI can call directly (no subprocess overhead)
- **REPL Integration**: Run AIChat's interactive chat with your custom setup
- **Multi-Provider Support**: Access to 20+ LLM providers (OpenAI, Claude, Gemini, Ollama, etc.)
- **Zero Dependencies**: Thin wrapper using path imports - no modifications to AIChat source

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
aichat-agent = "0.30.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Simple Chat Session

```rust
use aichat_agent::{TempConfigBuilder, ReplBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create temporary configuration
    let config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", std::env::var("OPENAI_API_KEY")?)
        .temperature(0.7)
        .build()
        .await?;

    // Start interactive chat
    ReplBuilder::with_config(config)
        .build()
        .await?
        .run()
        .await?;

    Ok(())
}
```

### Creating Custom Agents

```rust
use aichat_agent::{TempConfigBuilder, ReplBuilder, AgentDefinitionBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up configuration
    let config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini") 
        .api_key("openai", std::env::var("OPENAI_API_KEY")?)
        .build()
        .await?;

    let config_dir = config.read().config_dir();
    
    // Create a custom agent
    AgentDefinitionBuilder::new("coding-assistant")
        .description("A helpful coding assistant")
        .instructions("You are an expert programmer. Help users write clean, efficient code.")
        .add_starter("What coding problem can I help you solve?")
        .add_starter("Need help debugging or optimizing code?")
        .save_to(&config_dir.join("agents"))?;

    // Start chat with the custom agent
    ReplBuilder::with_config(config)
        .agent("coding-assistant")
        .build()
        .await?
        .run()
        .await?;

    Ok(())
}
```

### Native Function Integration

```rust
use aichat_agent::{
    TempConfigBuilder, ReplBuilder, AgentDefinitionBuilder,
    FunctionRegistry, Result
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", std::env::var("OPENAI_API_KEY")?)
        .build()
        .await?;

    let config_dir = config.read().config_dir();

    // Register native Rust functions
    let mut functions = FunctionRegistry::new();
    
    functions.register("calculate", "Perform arithmetic calculations", |args| {
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        let op = args["operation"].as_str().unwrap_or("add");
        
        let result = match op {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => if b != 0.0 { a / b } else { f64::NAN },
            _ => f64::NAN,
        };
        
        Ok(json!({
            "result": result,
            "expression": format!("{} {} {} = {}", a, op, b, result)
        }))
    });

    // Install functions to config directory
    functions.install(&config_dir.join("functions"))?;

    // Create agent that uses the functions
    AgentDefinitionBuilder::new("calculator")
        .description("A calculator assistant")
        .instructions("You can perform calculations using the calculate function. Always show your work step by step.")
        .save_to(&config_dir.join("agents"))?;

    // Start interactive session
    ReplBuilder::with_config(config)
        .agent("calculator")
        .build()
        .await?
        .run()
        .await?;

    Ok(())
}
```

## Advanced Usage

### Builder Pattern APIs

All components use fluent builder patterns for easy configuration:

```rust
// Configure everything step by step
let config = TempConfigBuilder::new()?
    .model("anthropic:claude-3-sonnet")
    .api_key("anthropic", api_key)
    .temperature(0.3)
    .top_p(0.9) 
    .stream(true)
    .set("save", json!(false))  // Custom config values
    .build()
    .await?;

// Create sophisticated agents
let agent = AgentDefinitionBuilder::new("data-analyst") 
    .description("Expert data analyst and visualization specialist")
    .version("2.1.0")
    .instructions(include_str!("prompts/analyst_instructions.txt"))
    .dynamic_instructions(true)
    .add_variable("output_format", "Preferred output format")
    .add_variable_with_default("precision", "Decimal precision", "2")
    .add_starter("What data would you like me to analyze?")
    .add_starter("I can help with statistics, trends, and visualizations")
    .save_to(agents_dir)?;
```

### Multiple Function Types

Register different types of functions with full type safety:

```rust
let mut functions = FunctionRegistry::new();

// Mathematical functions
functions.register("statistics", "Calculate statistical measures", |args| {
    let numbers: Vec<f64> = args["numbers"]
        .as_array().unwrap()
        .iter().filter_map(|v| v.as_f64())
        .collect();
    
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let variance = numbers.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / numbers.len() as f64;
    
    Ok(json!({
        "mean": mean,
        "variance": variance,
        "std_dev": variance.sqrt(),
        "count": numbers.len()
    }))
});

// File operations
functions.register("read_file", "Read file contents", |args| {
    let path = args["path"].as_str().unwrap();
    let content = std::fs::read_to_string(path)?;
    Ok(json!({ "content": content }))
});

// API calls
functions.register("fetch_data", "Fetch data from API", |args| {
    let url = args["url"].as_str().unwrap();
    // Implementation would use reqwest or similar
    Ok(json!({ "status": "success", "data": "..." }))
});
```

### Complete Application Example

Here's how you might structure a complete application:

```rust
use aichat_agent::*;

pub struct MyAIApp {
    config: GlobalConfig,
}

impl MyAIApp {
    pub async fn new() -> Result<Self> {
        let config = TempConfigBuilder::new()?
            .model("openai:gpt-4o")
            .api_key("openai", std::env::var("OPENAI_API_KEY")?)
            .temperature(0.7)
            .build()
            .await?;
        
        Ok(Self { config })
    }
    
    pub async fn setup_domain_expert(&self, domain: &str) -> Result<()> {
        let config_dir = self.config.read().config_dir();
        
        // Set up domain-specific functions
        let mut functions = FunctionRegistry::new();
        self.register_domain_functions(&mut functions, domain)?;
        functions.install(&config_dir.join("functions"))?;
        
        // Create specialized agent
        AgentDefinitionBuilder::new(&format!("{}-expert", domain))
            .description(&format!("Expert assistant for {}", domain))
            .instructions(&self.get_domain_instructions(domain))
            .save_to(&config_dir.join("agents"))?;
        
        Ok(())
    }
    
    pub async fn start_session(&self, agent: Option<&str>) -> Result<()> {
        let mut builder = ReplBuilder::with_config(self.config.clone());
        
        if let Some(agent_name) = agent {
            builder = builder.agent(agent_name);
        }
        
        builder.build().await?.run().await
    }
    
    fn register_domain_functions(&self, registry: &mut FunctionRegistry, domain: &str) -> Result<()> {
        match domain {
            "math" => {
                registry.register("solve_equation", "Solve mathematical equations", |args| {
                    // Implementation
                    Ok(json!({"solution": "x = 42"}))
                });
            }
            "code" => {
                registry.register("lint_code", "Check code quality", |args| {
                    // Implementation  
                    Ok(json!({"issues": []}))
                });
            }
            _ => {}
        }
        Ok(())
    }
    
    fn get_domain_instructions(&self, domain: &str) -> String {
        match domain {
            "math" => "You are a mathematics expert...".to_string(),
            "code" => "You are a senior software engineer...".to_string(),
            _ => "You are a helpful assistant...".to_string(),
        }
    }
}

#[tokio::main] 
async fn main() -> Result<()> {
    let app = MyAIApp::new().await?;
    app.setup_domain_expert("math").await?;
    app.start_session(Some("math-expert")).await?;
    Ok(())
}
```

## API Reference

### Core Types

- `TempConfigBuilder` - Creates isolated AIChat configurations
- `ReplBuilder` - Builds interactive chat sessions  
- `AgentDefinitionBuilder` - Creates custom AI agents
- `FunctionRegistry` - Manages native Rust functions
- `GlobalConfig` - Thread-safe AIChat configuration

### Key Functions

- `TempConfigBuilder::new()` - Start building a temporary config
- `ReplBuilder::with_config(config)` - Create REPL with specific config
- `AgentDefinitionBuilder::new(name)` - Start building an agent
- `FunctionRegistry::register(name, desc, func)` - Add native function

## Examples

Run the included example:

```bash
# First, update examples/config.yaml with your API keys
# Then run the math assistant example
cargo run --example math_assistant
```

This creates a math tutor with calculation, statistics, and geometry functions. The example uses a config file (`examples/config.yaml`) instead of environment variables, making it easier to configure multiple providers and settings.

## Architecture

This library uses a thin wrapper approach:
- **Path imports**: Direct access to AIChat modules via `#[path = "..."]`
- **Zero modifications**: No changes to AIChat source code
- **Temporary isolation**: Configs don't interfere with user's AIChat setup
- **Native performance**: Direct function calls, no subprocess overhead

## License

MIT OR Apache-2.0 (same as AIChat)

## Contributing

This is a thin wrapper around AIChat. For core functionality improvements, contribute to the [upstream AIChat repository](https://github.com/sigoden/aichat).