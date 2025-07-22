//! Example: Math Assistant with Custom Functions
//!
//! This example demonstrates how to create a custom AI assistant with
//! mathematical capabilities using the aichat-agent library.
//!
//! The assistant includes custom functions for calculations and can
//! explain its reasoning step by step.
//!
//! To run this example:
//! ```bash
//! cargo run --example math_assistant
//! ```

use aichat_agent::{
    TempConfigBuilder, ReplBuilder, AgentDefinitionBuilder,
    FunctionRegistry, Result
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Step 1: Create a temporary configuration from the example config file
    
    // Find the config file - works whether run from project root or examples dir
    let config_path = if std::path::Path::new("config.yaml").exists() {
        "config.yaml"  // Running from examples directory
    } else if std::path::Path::new("examples/config.yaml").exists() {
        "examples/config.yaml"  // Running from project root
    } else {
        return Err(anyhow::anyhow!(
            "Could not find config.yaml. Please run from project root or examples directory."
        ));
    };
    
    let config_builder = TempConfigBuilder::from_file(config_path)?
        .temperature(0.3);  // Lower temperature for accurate calculations
    
    let config_dir = config_builder.config_dir().to_path_buf();
    let config = config_builder.build().await?;

    // Step 2: Create and install custom math functions
    let mut functions = FunctionRegistry::new();

    // Basic calculator function
    functions.register("calculate", "Perform arithmetic calculations", |args| {
        let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("add");

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => if b != 0.0 { a / b } else { f64::NAN },
            "power" => a.powf(b),
            "sqrt" => a.sqrt(),
            _ => {
                return Ok(json!({
                    "error": format!("Unknown operation: {}", operation)
                }));
            }
        };

        Ok(json!({
            "operation": operation,
            "inputs": { "a": a, "b": b },
            "result": result,
            "formatted": format!("{} {} {} = {}", a, operation, b, result)
        }))
    });

    // Statistics function
    functions.register("statistics", "Calculate statistical measures", |args| {
        let numbers = args.get("numbers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64())
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default();

        if numbers.is_empty() {
            return Ok(json!({
                "error": "No numbers provided"
            }));
        }

        let sum: f64 = numbers.iter().sum();
        let mean = sum / numbers.len() as f64;
        
        let variance = numbers.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / numbers.len() as f64;
        
        let std_dev = variance.sqrt();
        
        let mut sorted = numbers.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        Ok(json!({
            "count": numbers.len(),
            "sum": sum,
            "mean": mean,
            "median": median,
            "min": sorted.first(),
            "max": sorted.last(),
            "variance": variance,
            "std_dev": std_dev
        }))
    });

    // Geometry function
    functions.register("geometry", "Calculate geometric properties", |args| {
        let shape = args.get("shape").and_then(|v| v.as_str()).unwrap_or("circle");
        let empty_dims = json!({});
        let dimensions = args.get("dimensions").unwrap_or(&empty_dims);

        let result = match shape {
            "circle" => {
                let radius = dimensions.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0);
                json!({
                    "shape": "circle",
                    "radius": radius,
                    "area": std::f64::consts::PI * radius * radius,
                    "circumference": 2.0 * std::f64::consts::PI * radius
                })
            }
            "rectangle" => {
                let width = dimensions.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let height = dimensions.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
                json!({
                    "shape": "rectangle",
                    "width": width,
                    "height": height,
                    "area": width * height,
                    "perimeter": 2.0 * (width + height)
                })
            }
            "triangle" => {
                let base = dimensions.get("base").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let height = dimensions.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
                json!({
                    "shape": "triangle",
                    "base": base,
                    "height": height,
                    "area": 0.5 * base * height
                })
            }
            _ => json!({
                "error": format!("Unknown shape: {}", shape)
            })
        };

        Ok(result)
    });

    // Install functions to the config directory
    functions.install(&config_dir)?;

    // Step 3: Create the Math Assistant agent
    AgentDefinitionBuilder::new("math-assistant")
        .description("A helpful math tutor that can perform calculations and explain concepts")
        .version("1.0.0")
        .instructions(r#"You are a friendly and patient math tutor. You help students understand mathematical concepts and solve problems step by step.

You have access to several functions:
- `calculate`: For arithmetic operations (add, subtract, multiply, divide, power, sqrt)
- `statistics`: For statistical analysis of number sets
- `geometry`: For geometric calculations (circle, rectangle, triangle)

When solving problems:
1. Break down complex problems into steps
2. Use the available functions for calculations
3. Explain your reasoning clearly
4. Provide educational context when appropriate
5. Encourage learning and understanding

Always show your work and help students learn the underlying concepts, not just get the answer."#)
        .add_starter("What math problem can I help you solve today?")
        .add_starter("Would you like me to explain a mathematical concept?")
        .add_starter("Need help with calculations? I can handle arithmetic, statistics, and geometry!")
        .save_to(&config_dir)?;

    // Step 4: Create and run the REPL session
    println!("Starting Math Assistant REPL...");
    println!("Try asking questions like:");
    println!("  - 'Calculate 15 * 23'");
    println!("  - 'What's the area of a circle with radius 5?'");
    println!("  - 'Find the mean and std deviation of [1, 2, 3, 4, 5]'");
    println!("  - 'Explain how to solve quadratic equations'");
    println!("\nType .help for available commands, .exit to quit\n");

    // NOTE: The example uses the config file in examples/config.yaml
    println!("📝 Using configuration from examples/config.yaml");
    println!("   (Update the API key in that file if needed)\n");

    // Run the REPL with our math assistant
    let session = ReplBuilder::with_config(config)
        .agent("math-assistant")
        .build()
        .await?;
    
    session.run().await?;

    println!("\nThanks for using Math Assistant!");
    Ok(())
}