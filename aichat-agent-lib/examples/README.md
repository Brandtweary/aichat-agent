# Examples

This directory contains example applications demonstrating the aichat-agent library.

## Configuration

Before running examples, create a `config.yaml` file from the template:

```bash
cp config.example.yaml config.yaml
```

Then update `config.yaml` with your API key:

1. Replace `YOUR_ANTHROPIC_API_KEY_HERE` with your actual Claude API key
2. Optionally adjust the model, temperature, and other settings

**Note**: The `config.yaml` file is gitignored to prevent accidental API key commits.

## Running Examples

### Math Assistant

A complete AI math tutor with custom calculation functions:

```bash
cargo run --example math_assistant
```

This example demonstrates:
- Loading configuration from file
- Creating custom agents
- Registering native Rust functions
- Running an interactive REPL session

The math assistant can:
- Perform arithmetic calculations
- Calculate statistics (mean, median, std dev)
- Compute geometric properties (area, perimeter)
- Explain mathematical concepts step by step
