# Examples

This directory contains example applications demonstrating the aichat-agent library.

## Configuration

Before running examples, update `config.yaml` with your API keys:

1. **For Claude API**: Update the `api_key` field under the `claude` client
2. **For OpenAI API**: Uncomment the `openai` client section and add your key
3. **For local Ollama**: Uncomment the `ollama` client section (no key needed)

Example config.yaml setup:

```yaml
model: openai:gpt-4o-mini
clients:
- type: openai
  api_key: sk-your-actual-openai-key
```

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

## Security Note

The current `config.yaml` contains a real API key for testing purposes. In production:
1. Never commit API keys to version control
2. Use environment variables or secure secret management
3. Rotate keys regularly