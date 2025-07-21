# AIChat Function Calling Analysis

## Overview

AIChat implements a sophisticated function calling system that allows LLMs to interact with external tools and execute code. The system is designed to be extensible and supports multiple execution models.

## Architecture Components

### 1. Function Definition and Registration

#### Function Declaration Structure (`src/function.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
    #[serde(skip_serializing, default)]
    pub agent: bool,  // Indicates if this is an agent-specific function
}
```

#### JSON Schema Support
Functions use JSON Schema for parameter validation:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub type_value: Option<String>,
    pub description: Option<String>,
    pub properties: Option<IndexMap<String, JsonSchema>>,
    pub items: Option<Box<JsonSchema>>,
    pub any_of: Option<Vec<JsonSchema>>,
    pub enum_value: Option<Vec<String>>,
    pub default: Option<Value>,
    pub required: Option<Vec<String>>,
}
```

#### Function Storage
- Global functions: Stored in `$AICHAT_FUNCTIONS_DIR/functions.json`
- Agent functions: Stored in `$AICHAT_FUNCTIONS_DIR/agents/{agent_name}/functions.json`
- Function binaries: Located in `bin/` subdirectories

### 2. Function Calling Protocol

#### Tool Call Structure
```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub id: Option<String>,  // Used for deduplication
}
```

#### Tool Result Structure
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolResult {
    pub call: ToolCall,
    pub output: Value,
}
```

### 3. Execution Flow

#### Main Execution Pipeline
1. **LLM Decision**: The LLM analyzes the prompt and available functions, deciding which to call
2. **Tool Call Extraction**: Client implementations parse LLM responses for tool calls
3. **Evaluation**: `eval_tool_calls()` processes the calls:
   - Deduplicates calls by ID
   - Executes each function
   - Collects results
4. **Result Integration**: Results are fed back into the conversation

#### Function Execution Model (`run_llm_function`)
```rust
pub fn run_llm_function(
    cmd_name: String,
    cmd_args: Vec<String>,
    mut envs: HashMap<String, String>,
) -> Result<Option<String>>
```

**Execution Details:**
- Functions are executed as external processes
- Arguments are passed as JSON strings
- Environment variables:
  - `PATH`: Extended with function bin directories
  - `LLM_OUTPUT`: Temporary file for capturing output
- Exit code 0 indicates success
- Output is read from the `LLM_OUTPUT` file

### 4. Client Integration

#### Chat Completions Data
```rust
pub struct ChatCompletionsData {
    pub messages: Vec<Message>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub functions: Option<Vec<FunctionDeclaration>>,  // Available functions
    pub stream: bool,
}
```

#### Client-Specific Handling
Each LLM client implements function calling differently:
- **OpenAI**: Native function calling with structured tool_calls
- **Claude**: Tool use blocks with XML-like structure
- **Others**: Various proprietary formats

### 5. Message Flow with Functions

#### Function Call Message Structure
```rust
pub struct MessageContentToolCalls {
    pub tool_results: Vec<ToolResult>,
    pub text: String,  // Optional text accompanying the calls
    pub sequence: bool,  // Whether calls are sequential
}
```

#### Conversation Flow
1. User provides input
2. System adds available functions to the request
3. LLM responds with text and/or tool calls
4. System executes tool calls
5. Results are merged back into the conversation
6. Process repeats if more tool calls are needed

### 6. Agent Integration

Agents can have their own functions that:
- Are scoped to the agent's directory
- Can access agent-specific environment variables
- Support both agent-specific and global functions

## Key Design Decisions

### 1. Process-Based Execution
- Functions run as separate processes for isolation
- Supports any executable (scripts, binaries)
- Clear input/output contract via environment variables

### 2. JSON Schema Validation
- Strongly typed parameter definitions
- Automatic validation before execution
- Rich type support (objects, arrays, enums)

### 3. Deduplication Mechanism
- Prevents infinite loops of function calls
- Uses call IDs when available
- Maintains call order

### 4. Flexible Output Handling
- Functions write to `$LLM_OUTPUT` file
- Supports both JSON and plain text output
- Null outputs are converted to "DONE"

## Integration Points for Cymbiont

### 1. Custom Function Registration
Cymbiont can register PKM/KG functions by:
- Creating a `functions.json` file with declarations
- Placing executables in the `bin/` directory
- Using the standard JSON Schema format

### 2. Function Examples for PKM
```json
{
  "name": "search_knowledge_graph",
  "description": "Search the knowledge graph for related concepts",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "The search query"
      },
      "depth": {
        "type": "integer",
        "description": "How many hops to traverse",
        "default": 2
      }
    },
    "required": ["query"]
  }
}
```

### 3. Execution Model Options
- **Direct API calls**: Function executable makes HTTP requests to PKM backend
- **Library integration**: Function executable uses Rust library to query graph
- **Protocol-based**: Function communicates via pipes or sockets

### 4. Result Integration
- Return structured JSON with graph data
- Include both direct results and related nodes
- Support streaming for large result sets

## Security Considerations

1. **Process Isolation**: Each function runs in its own process
2. **Environment Control**: Limited environment variables passed
3. **Output Validation**: Results are parsed and validated
4. **Resource Limits**: Relies on OS-level process limits

## Extensibility

The system is designed for easy extension:
1. **New function types**: Just add to functions.json
2. **Custom executables**: Any language that can read args and write files
3. **Agent-specific tools**: Scoped functions for different contexts
4. **Dynamic loading**: Functions are loaded at runtime

## Best Practices for Function Development

1. **Clear Descriptions**: Help the LLM understand when to use the function
2. **Robust Parameters**: Use JSON Schema features for validation
3. **Error Handling**: Return error messages in JSON format
4. **Performance**: Keep functions fast to avoid blocking
5. **Documentation**: Include usage examples in descriptions