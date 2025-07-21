# AIChat Architecture Analysis for Library Conversion

## Overview

AIChat (currently named "cymbiont" in this fork) is a CLI application built in Rust that provides a chat interface for various LLM providers. The codebase is organized as a binary crate without a library component (`lib.rs`), which means all functionality is currently tightly coupled to the CLI. This analysis examines the architecture to understand how to expose it as a library.

## 1. Overall Module Structure and Dependencies

### Project Structure
- **Binary Name**: `cymbiont` (originally AIChat, now forked)
- **Main Entry**: `src/main.rs` - CLI entry point with `#[tokio::main]`
- **No lib.rs**: Currently not structured as a library
- **Config Directory**: `.aichat/` or custom via `AICHAT_CONFIG_DIR`

### Core Modules
- **`main.rs`**: Entry point, handles CLI parsing, working mode detection, and orchestrates the application flow
- **`cli.rs`**: Command-line interface definition using clap with derive
- **`client/`**: LLM provider abstraction layer (OpenAI, Claude, Gemini, Cohere, etc.)
- **`config/`**: Configuration management, session handling, input processing, agents, and roles
- **`function.rs`**: Function/tool calling implementation with JSON Schema
- **`rag/`**: Retrieval-Augmented Generation functionality with embeddings
- **`render/`**: Output rendering (markdown, streaming, syntax highlighting)
- **`repl/`**: Interactive REPL mode implementation with reedline
- **`serve.rs`**: HTTP server mode for OpenAI-compatible API endpoints
- **`utils/`**: Utility functions (clipboard, commands, spinners, paths, etc.)

### Key Dependencies
- **Async Runtime**: `tokio` with multi-threaded runtime, signal handling
- **CLI Parsing**: `clap` with derive features for argument parsing
- **Interactive UI**: `inquire` for prompts, `reedline` for REPL interface
- **HTTP Client**: `reqwest` with rustls, streaming support
- **Serialization**: `serde`, `serde_json`, `serde_yaml`
- **Global State**: `parking_lot::RwLock` for thread-safe config access
- **Terminal**: `is_terminal` for TTY detection, `crossterm` for cursor control

## 2. Agent Implementation and Lifecycle

The agent system is implemented in `config/agent.rs`:

### Agent Structure
```rust
pub struct Agent {
    name: String,
    config: AgentConfig,
    definition: AgentDefinition,
    shared_variables: AgentVariables,
    session_variables: Option<AgentVariables>,
    shared_dynamic_instructions: Option<String>,
    session_dynamic_instructions: Option<String>,
    functions: Functions,
    rag: Option<Arc<Rag>>,
    model: Model,
}
```

### Agent Lifecycle
1. **Initialization**: 
   - Loads from `agents/{name}/index.yaml`
   - Reads agent-specific config from `agents/{name}/config.yaml`
   - Loads functions from `agents/{name}/functions.json`
   - Initializes RAG from `agents/{name}/rag/default.bin` if documents exist
   
2. **Variable Resolution**: 
   - Supports shared and session-specific variables
   - Interactive prompts for missing variables (if terminal available)
   - Variables can have defaults and descriptions
   - Used for dynamic instruction interpolation

3. **Function Loading**: 
   - Agents can have their own function definitions
   - Functions directory can contain executables
   - Tool placeholder replacement in instructions

4. **Model Selection**: 
   - Agents can specify their own model or inherit from global config
   - Temperature and top_p overrides supported

## 3. Function/Tool Calling Flow

Function calling is implemented in `function.rs`:

### Key Components
```rust
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
    pub agent: bool,  // Agent-specific function
}

pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub id: Option<String>,
}

pub struct ToolResult {
    pub call: ToolCall,
    pub output: Value,
}
```

### Execution Flow
1. **Declaration Loading**: Functions loaded from JSON files in functions directory
2. **LLM Integration**: During chat completion, LLM can generate tool calls
3. **Execution**: `eval_tool_calls()` processes calls sequentially:
   - Deduplicates calls by ID to prevent loops
   - Extracts command configuration from agent or global config
   - Executes external command with JSON arguments
   - Returns JSON output or "DONE" for null results
4. **Continuation**: Results fed back to LLM for next completion round

### Command Execution
- Functions map to external executables
- Arguments passed as JSON string to command
- Environment variables can be injected
- Output expected as JSON or plain text

## 4. Config and Global State Management

### Config Structure (`Config` struct)
```rust
pub struct Config {
    // Model Settings
    pub model_id: String,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    
    // Behavior Flags
    pub dry_run: bool,
    pub stream: bool,
    pub save: bool,
    pub function_calling: bool,
    
    // UI Settings
    pub keybindings: String,  // "emacs" or "vi"
    pub editor: Option<String>,
    pub wrap: Option<String>,
    pub highlight: bool,
    pub theme: Option<String>,
    
    // Session Management
    pub save_session: Option<bool>,
    pub compress_threshold: usize,
    pub summarize_prompt: Option<String>,
    
    // RAG Settings
    pub rag_embedding_model: Option<String>,
    pub rag_reranker_model: Option<String>,
    pub rag_chunk_size: Option<usize>,
    
    // Clients Configuration
    pub clients: Vec<ClientConfig>,
    
    // Runtime State (not serialized)
    #[serde(skip)]
    pub role: Option<Role>,
    #[serde(skip)]
    pub session: Option<Session>,
    #[serde(skip)]
    pub rag: Option<Arc<Rag>>,
    #[serde(skip)]
    pub agent: Option<Agent>,
}
```

### Global State Management
- **`GlobalConfig`**: Type alias for `Arc<RwLock<Config>>`
- Passed throughout codebase for shared access
- Mutable runtime state mixed with configuration
- Environment variable loading with `CYMBIONT_` prefix

### Working Modes
```rust
pub enum WorkingMode {
    Cmd,    // One-shot command mode
    Repl,   // Interactive REPL mode  
    Serve,  // HTTP server mode
}
```

### Global Singletons (LazyLock/OnceLock)
- `EDITOR`: Editor preference cache
- `IS_STDOUT_TERMINAL`: Terminal detection result
- `NO_COLOR`: Color output control
- `CODE_BLOCK_RE`: Regex for code extraction
- `THINK_TAG_RE`: Regex for thought removal
- Various other regex patterns

## 5. Hardcoded CLI Assumptions

### Terminal Detection and Output
- **`IS_STDOUT_TERMINAL`**: Global check affects entire behavior
- **Color Output**: Controlled by `NO_COLOR` env var and terminal detection
- **Progress Spinners**: Shown during file loading, RAG operations
- **Error Rendering**: Terminal-specific formatting with colors
- **Shell Execution**: Special handling for macOS pipe restrictions

### Interactive Prompts Throughout
- **Model Selection**: `Select::new()` for choosing models
- **Role Management**: Interactive role creation/editing
- **Agent Variables**: Prompts for missing agent variables
- **Confirmation Dialogs**: For destructive operations
- **Shell Command Review**: Execute/revise/describe/copy options

### File System Dependencies
```
~/.aichat/
├── config.yaml
├── roles/
├── sessions/
├── rags/
├── functions/
├── agents/
└── macros/
```
- Hard-coded directory structure
- Platform-specific config paths (XDG on Linux)
- File-based storage for all persistent data

### Process and Environment
- **Signal Handling**: SIGINT/SIGTERM for graceful shutdown
- **Editor Spawning**: External editor for multi-line input
- **Shell Integration**: Platform-specific shell detection
- **Clipboard Access**: Platform-specific implementations
- **Command Execution**: Direct process spawning for functions

## 6. REPL/Interactive Features

The REPL is implemented in `repl/mod.rs` using `reedline`:

### Core Components
- **`Repl` struct**: Main REPL controller with state management
- **`ReplCompleter`**: Context-aware completion for commands, paths, models
- **`ReplHighlighter`**: Syntax highlighting with theme support
- **`ReplPrompt`**: Dynamic prompt showing role/session/agent state
- **`ReplValidator`**: Input validation for commands

### REPL Commands (36 total)
```
.help            Show help guide
.info            Show system info
.model           Switch LLM model
.role            Create/switch role
.session         Start/switch session
.agent           Use an agent
.rag             Initialize/access RAG
.macro           Execute macro
.file            Include file content
.copy            Copy last response
.exit            Exit REPL
```

### State Management
- Complex state transitions based on active role/session/agent
- Commands enabled/disabled based on current state
- Validation ensures valid state transitions

### Keybindings
- **Emacs mode**: Default with standard readline bindings
- **Vi mode**: Full vi insert/normal mode support
- **Special keys**:
  - `Alt+Enter`: Multiline input
  - `Tab`: Completion
  - `Ctrl+C`: Cancel current input
  - `Ctrl+D`: Exit REPL

## 7. Key Entry Points for Library Exposure

### Primary Functions to Expose

1. **Chat Completion API** (`client/common.rs`):
   ```rust
   pub async fn call_chat_completions(
       input: &Input,
       print: bool,
       extract_code: bool,
       client: &dyn Client,
       abort_signal: AbortSignal,
   ) -> Result<(String, Vec<ToolResult>)>
   ```

2. **Streaming Chat** (`client/common.rs`):
   ```rust
   pub async fn call_chat_completions_streaming(
       input: &Input,
       client: &dyn Client,
       abort_signal: AbortSignal,
   ) -> Result<(String, Vec<ToolResult>)>
   ```

3. **Configuration Management** (`config/mod.rs`):
   - `Config::init()` - Initialize configuration
   - `Config::set_model()` - Change model
   - `Config::use_role()` - Set role
   - `Config::use_session()` - Manage sessions
   - `Config::use_prompt()` - Set system prompt

4. **Agent System** (`config/agent.rs`):
   - `Agent::init()` - Initialize agent with variables
   - `Config::use_agent()` - Activate agent in config

5. **Client Creation** (`client/common.rs`):
   - `create_client_config()` - Build client from config
   - Model selection and validation

## 8. Major Challenges for Library Conversion

### Global State Issues
1. **`GlobalConfig` Everywhere**: `Arc<RwLock<Config>>` threaded through all functions
2. **Mutable Runtime State**: Config mixes settings with runtime state
3. **LazyLock Statics**: Terminal detection, regexes cached globally
4. **Environment Coupling**: Direct env var access throughout

### Interactive Code Coupling
1. **Inquire Prompts**: 50+ interactive prompts scattered in code
2. **Spinner Usage**: Progress indication assumes terminal
3. **REPL Integration**: Commands directly modify global state
4. **Editor Spawning**: External process management

### File System Assumptions
1. **Hard-coded Paths**: `~/.aichat/` directory structure
2. **Config Loading**: YAML files expected at specific locations
3. **Session Persistence**: File-based session storage
4. **Function Discovery**: Scans directories for executables

### Process and I/O
1. **Tokio Runtime**: `#[tokio::main]` in binary
2. **Signal Handling**: Global signal handlers
3. **Process Exit**: Direct `process::exit()` calls
4. **Stdout/Stderr**: Direct writes for output

## 9. Recommended Library API Design

### Core Types to Expose
```rust
// Main client interface
pub struct AIChatClient {
    config: Config,
    clients: HashMap<String, Box<dyn Client>>,
}

// Request/Response types
pub struct ChatRequest {
    pub message: String,
    pub role: Option<String>,
    pub session_id: Option<String>,
    pub files: Vec<PathBuf>,
    pub stream: bool,
}

pub struct ChatResponse {
    pub content: String,
    pub tool_results: Vec<ToolResult>,
    pub model_info: ModelInfo,
}

// Abstraction traits
pub trait StorageBackend {
    async fn load_config(&self) -> Result<Config>;
    async fn save_session(&self, id: &str, session: &Session) -> Result<()>;
    async fn load_session(&self, id: &str) -> Result<Session>;
}

pub trait UIBackend {
    async fn prompt_selection<T>(&self, message: &str, options: Vec<T>) -> Result<T>;
    async fn prompt_text(&self, message: &str) -> Result<String>;
    async fn show_progress(&self, message: &str);
}
```

### Clean API Surface
```rust
impl AIChatClient {
    // Construction
    pub fn builder() -> AIChatBuilder;
    pub async fn new(config: Config) -> Result<Self>;
    
    // Core chat operations
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    pub async fn chat_streaming(
        &self, 
        request: ChatRequest,
        handler: impl StreamHandler,
    ) -> Result<ChatResponse>;
    
    // Configuration
    pub fn set_model(&mut self, model_id: &str) -> Result<()>;
    pub fn use_role(&mut self, role_name: &str) -> Result<()>;
    pub async fn use_agent(&mut self, agent_name: &str) -> Result<()>;
    
    // Session management
    pub async fn create_session(&mut self, name: &str) -> Result<SessionHandle>;
    pub async fn load_session(&mut self, name: &str) -> Result<SessionHandle>;
    pub async fn save_session(&self, handle: &SessionHandle) -> Result<()>;
}
```

## 10. Migration Strategy

### Phase 1: Internal Refactoring
1. Create `src/lib.rs` exposing core types
2. Move non-interactive logic to library modules
3. Create abstraction traits for I/O and storage
4. Eliminate `process::exit()` calls

### Phase 2: State Management
1. Remove `GlobalConfig` in favor of instance-based state
2. Separate configuration from runtime state
3. Make all functions accept explicit parameters
4. Remove global LazyLock usage in library

### Phase 3: API Design
1. Design builder pattern for client construction
2. Create request/response types for all operations
3. Implement streaming with callback traits
4. Document all public APIs

### Phase 4: CLI Adapter
1. Implement storage/UI traits for CLI
2. Keep `main.rs` as thin wrapper
3. Maintain backward compatibility
4. Move interactive features to CLI layer

### Phase 5: Testing and Documentation
1. Unit tests for all library functions
2. Integration tests with mock backends
3. Example programs showing library usage
4. Migration guide for existing users

## Summary

AIChat's current architecture is deeply coupled to CLI usage with:
- Global state management via `Arc<RwLock<Config>>`
- Interactive prompts and terminal I/O throughout
- File-based configuration and storage
- Direct process management and signal handling

Converting to a library requires:
1. **State isolation**: Replace global config with instance-based state
2. **I/O abstraction**: Traits for storage, UI, and progress
3. **Clean API**: Request/response pattern with builder configuration
4. **Async-first**: Let users control the runtime
5. **Testability**: Mock implementations for all external dependencies

The core LLM client logic is well-structured but needs significant refactoring to decouple from CLI assumptions.