//! Integration tests for aichat-agent library

use aichat_agent::{
    TempConfigBuilder, ReplBuilder, AgentDefinitionBuilder, 
    AgentFunctionsBuilder, FunctionRegistry, FunctionsBuilder,
    function::FunctionDeclaration, Result
};
use serial_test::serial;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
#[serial]
async fn test_repl_session_creation() -> Result<()> {
    // Create a temporary config
    let config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", "sk-test-key")
        .build()
        .await?;
    
    // Create a REPL session without an agent (since we don't have any agents configured)
    let session = ReplBuilder::with_config(config)
        .build()
        .await?;
    
    // We can't easily test the actual REPL interaction in unit tests
    // since it requires terminal input/output, but we can verify
    // that the session was created successfully
    assert!(session.agent().is_none());
    
    Ok(())
}

#[tokio::test] 
#[serial]
async fn test_repl_with_custom_prelude() -> Result<()> {
    let config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", "sk-test-key")
        .set("repl_prelude", serde_json::json!("Welcome to my custom REPL!"))
        .build()
        .await?;
    
    let session = ReplBuilder::with_config(config)
        .build()
        .await?;
    
    // Session should be created successfully
    assert!(session.agent().is_none());
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_agent_creation_and_loading() -> Result<()> {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new()?;
    let _agents_dir = temp_dir.path().join("functions/agents");
    
    // Create an agent definition
    let agent = AgentDefinitionBuilder::new("test-assistant")
        .description("A test assistant for integration testing")
        .version("1.0.0")
        .instructions("You are a helpful test assistant. Always be concise.")
        .add_variable("api_endpoint", "API endpoint for testing")
        .add_variable_with_default("timeout", "Request timeout in seconds", "30")
        .add_starter("How can I help you test today?")
        .add_starter("What would you like to verify?")
        .save_to(temp_dir.path())?;
    
    // Verify the agent was saved correctly
    assert_eq!(agent.name, "test-assistant");
    assert_eq!(agent.variables.len(), 2);
    
    // Create a config that includes our agent directory
    let _config = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", "sk-test-key")
        .build()
        .await?;
    
    // Copy our agent to the config's agents directory
    let config_agents_dir = temp_dir.path().join("agents");
    std::fs::create_dir_all(&config_agents_dir)?;
    
    // In a real scenario, we'd load the agent using AIChat's mechanisms
    // For now, we just verify the files exist
    let agent_dir = temp_dir.path().join("functions/agents/test-assistant");
    assert!(agent_dir.join("index.yaml").exists());
    assert!(agent_dir.join("functions.json").exists());
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_custom_functions_registration() -> Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let functions_dir = temp_dir.path().join("functions");
    
    // Create and register custom functions
    let mut registry = FunctionRegistry::new();
    
    // Register a simple calculator function
    registry.register("calculate", "Perform basic calculations", |args| {
        let operation = args["operation"].as_str().unwrap_or("add");
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        
        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => if b != 0.0 { a / b } else { 0.0 },
            _ => 0.0,
        };
        
        Ok(json!({
            "result": result,
            "operation": operation,
            "inputs": { "a": a, "b": b }
        }))
    });
    
    // Register a string manipulation function
    registry.register("text_transform", "Transform text in various ways", |args| {
        let text = args["text"].as_str().unwrap_or("");
        let transform = args["transform"].as_str().unwrap_or("uppercase");
        
        let result = match transform {
            "uppercase" => text.to_uppercase(),
            "lowercase" => text.to_lowercase(),
            "reverse" => text.chars().rev().collect(),
            "title" => text.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            _ => text.to_string(),
        };
        
        Ok(json!({
            "original": text,
            "transformed": result,
            "transform_type": transform
        }))
    });
    
    // Test function execution
    let calc_result = registry.execute("calculate", json!({
        "operation": "multiply",
        "a": 7,
        "b": 6
    }))?;
    assert_eq!(calc_result["result"], 42.0);
    
    let text_result = registry.execute("text_transform", json!({
        "text": "hello world",
        "transform": "title"
    }))?;
    assert_eq!(text_result["transformed"], "Hello World");
    
    // Install the functions
    registry.install(temp_dir.path())?;
    
    // Verify installation
    assert!(functions_dir.join("functions.json").exists());
    assert!(functions_dir.join("bin").exists());
    
    #[cfg(unix)]
    {
        assert!(functions_dir.join("bin/calculate").exists());
        assert!(functions_dir.join("bin/text_transform").exists());
    }
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_agent_with_custom_functions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let _agents_dir = temp_dir.path().join("agents");
    
    // Create an agent with custom functions
    let _agent = AgentDefinitionBuilder::new("math-assistant")
        .description("A mathematical assistant with calculation capabilities")
        .instructions(r#"You are a mathematical assistant. You can perform calculations using the available functions.
        
When asked to calculate, use the calculate function with appropriate operations.
Always show your work and explain the calculations."#)
        .add_starter("What calculation would you like me to perform?")
        .add_starter("Need help with math? I can add, subtract, multiply, and divide!")
        .save_to(temp_dir.path())?;
    
    // Create agent-specific functions
    use aichat_agent::function::{FunctionDeclaration, JsonSchema};
    use indexmap::indexmap;
    
    let calc_function = FunctionDeclaration {
        name: "calculate".to_string(),
        description: "Perform mathematical calculations".to_string(),
        parameters: JsonSchema {
            type_value: Some("object".to_string()),
            description: Some("Parameters for calculation".to_string()),
            properties: Some(indexmap! {
                "operation".to_string() => JsonSchema {
                    type_value: Some("string".to_string()),
                    description: Some("The operation to perform".to_string()),
                    enum_value: Some(vec![
                        "add".to_string(),
                        "subtract".to_string(), 
                        "multiply".to_string(),
                        "divide".to_string()
                    ]),
                    properties: None,
                    items: None,
                    any_of: None,
                    default: None,
                    required: None,
                },
                "a".to_string() => JsonSchema {
                    type_value: Some("number".to_string()),
                    description: Some("First number".to_string()),
                    properties: None,
                    items: None,
                    any_of: None,
                    enum_value: None,
                    default: None,
                    required: None,
                },
                "b".to_string() => JsonSchema {
                    type_value: Some("number".to_string()),
                    description: Some("Second number".to_string()),
                    properties: None,
                    items: None,
                    any_of: None,
                    enum_value: None,
                    default: None,
                    required: None,
                },
            }),
            items: None,
            any_of: None,
            enum_value: None,
            default: None,
            required: Some(vec!["operation".to_string(), "a".to_string(), "b".to_string()]),
        },
        agent: true,
    };
    
    AgentFunctionsBuilder::new("math-assistant")
        .add_function(calc_function)
        .save_to(temp_dir.path())?;
    
    // Verify the complete agent setup
    let agent_dir = temp_dir.path().join("functions/agents/math-assistant");
    assert!(agent_dir.join("index.yaml").exists());
    
    let functions_content = std::fs::read_to_string(agent_dir.join("functions.json"))?;
    let functions: Vec<FunctionDeclaration> = serde_json::from_str(&functions_content)?;
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].name, "calculate");
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_functions_builder_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let functions_dir = temp_dir.path().join("functions");
    
    // Use the FunctionsBuilder for a more ergonomic API
    let functions = FunctionsBuilder::new(temp_dir.path())
        .register("get_time", "Get current time in various formats", |args| {
            let format = args["format"].as_str().unwrap_or("iso");
            
            // In a real implementation, we'd use chrono to get actual time
            // For testing, we'll return a fixed response
            let time_str = match format {
                "unix" => "1234567890",
                "iso" => "2024-01-15T10:30:00Z",
                "human" => "January 15, 2024 at 10:30 AM",
                _ => "Unknown format",
            };
            
            Ok(json!({
                "time": time_str,
                "format": format
            }))
        })
        .register("random_number", "Generate a random number", |args| {
            let min = args["min"].as_i64().unwrap_or(0);
            let max = args["max"].as_i64().unwrap_or(100);
            
            // For testing, return a predictable "random" number
            let number = (min + max) / 2;
            
            Ok(json!({
                "number": number,
                "range": { "min": min, "max": max }
            }))
        })
        .build()?;
    
    // Verify the functions were created
    assert_eq!(functions.declarations().len(), 2);
    
    // Verify the function files exist
    assert!(functions_dir.join("functions.json").exists());
    assert!(functions_dir.join("bin").exists());
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_complete_integration_workflow() -> Result<()> {
    // This test demonstrates a complete workflow:
    // 1. Create a temporary config
    // 2. Register custom functions
    // 3. Create a custom agent
    // 4. Build a REPL session with everything configured
    
    let _temp_dir = TempDir::new()?;
    
    // Step 1: Create base configuration
    let config_builder = TempConfigBuilder::new()?
        .model("openai:gpt-4o-mini")
        .api_key("openai", "sk-test-key")
        .temperature(0.7)
        .stream(true);
    
    let config_dir = config_builder.config_dir().to_path_buf();
    
    // Step 2: Set up custom functions in the config directory
    let functions_dir = config_dir.join("functions");
    std::fs::create_dir_all(&functions_dir)?;
    
    let mut registry = FunctionRegistry::new();
    
    // Add a knowledge graph query function (simulated)
    registry.register("query_knowledge_graph", "Query the knowledge graph", |args| {
        let query = args["query"].as_str().unwrap_or("");
        let entity_type = args["entity_type"].as_str().unwrap_or("any");
        
        // Simulate a knowledge graph response
        Ok(json!({
            "query": query,
            "entity_type": entity_type,
            "results": [
                {
                    "id": "entity_1",
                    "type": entity_type,
                    "name": format!("Sample {} for '{}'", entity_type, query),
                    "properties": {
                        "created": "2024-01-15",
                        "relevance": 0.95
                    }
                }
            ],
            "total_results": 1
        }))
    });
    
    // Add a data processing function
    registry.register("process_data", "Process and transform data", |args| {
        let data = &args["data"];
        let operation = args["operation"].as_str().unwrap_or("summarize");
        
        let result = match operation {
            "summarize" => json!({
                "summary": format!("Processed {} items", data.as_array().map(|a| a.len()).unwrap_or(0)),
                "operation": operation
            }),
            "filter" => json!({
                "filtered": data,
                "operation": operation,
                "count": data.as_array().map(|a| a.len()).unwrap_or(0)
            }),
            _ => json!({
                "error": "Unknown operation",
                "operation": operation
            })
        };
        
        Ok(result)
    });
    
    registry.install(&config_dir)?;
    
    // Step 3: Create a custom agent in the config directory
    let _agents_dir = config_dir.join("agents");
    
    let agent = AgentDefinitionBuilder::new("knowledge-assistant")
        .description("An AI assistant with knowledge graph access")
        .version("2.0.0")
        .instructions(r#"You are a knowledge assistant with access to a knowledge graph and data processing capabilities.

You can:
1. Query the knowledge graph using the query_knowledge_graph function
2. Process data using the process_data function

Always be helpful and explain your findings clearly."#)
        .dynamic_instructions(false)
        .add_variable("default_entity_type", "Default entity type for queries")
        .add_variable_with_default("max_results", "Maximum results to return", "10")
        .add_starter("What would you like to know from the knowledge graph?")
        .add_starter("I can help you query entities and process data.")
        .save_to(&config_dir)?;
    
    // Verify complete setup
    assert_eq!(agent.name, "knowledge-assistant");
    assert_eq!(agent.version, "2.0.0");
    assert_eq!(agent.variables.len(), 2);
    
    // Verify all files exist  
    assert!(config_dir.join("functions/functions.json").exists());
    assert!(config_dir.join("functions/bin").exists());
    assert!(config_dir.join("functions/agents/knowledge-assistant/index.yaml").exists());
    
    // Read and verify the functions.json content
    let functions_json = std::fs::read_to_string(config_dir.join("functions/functions.json"))?;
    let functions: Vec<FunctionDeclaration> = serde_json::from_str(&functions_json)?;
    assert_eq!(functions.len(), 2);
    
    let function_names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    assert!(function_names.contains(&"query_knowledge_graph"));
    assert!(function_names.contains(&"process_data"));
    
    // Step 4: Build the config and create a REPL session
    let config = config_builder.build().await?;
    
    // Verify the config has the expected settings
    {
        let cfg = config.read();
        assert_eq!(cfg.model_id, "openai:gpt-4o-mini");
        assert_eq!(cfg.temperature, Some(0.7));
        assert!(cfg.stream);
    }
    
    // Create a REPL session (we can't load the agent in tests due to file paths)
    let session = ReplBuilder::with_config(config)
        .build()
        .await?;
    
    // We can't actually run the REPL in tests, but we've verified
    // that all components are properly set up
    assert!(session.agent().is_none()); // Agent isn't loaded yet
    
    Ok(())
}