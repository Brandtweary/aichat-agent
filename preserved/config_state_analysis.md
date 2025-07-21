# AIChat Config and Global State Analysis

## Overview

AIChat uses a global configuration pattern with `Arc<RwLock<Config>>` (aliased as `GlobalConfig`) that is shared across the entire application. This design creates significant challenges for converting AIChat into a library that supports multiple concurrent instances.

## The Config Struct

The `Config` struct in `src/config/mod.rs` contains both static configuration and runtime state:

### Static Configuration Fields (from config file)
- `model_id: String` - Default model identifier
- `temperature: Option<f64>` - LLM temperature setting
- `top_p: Option<f64>` - LLM top-p setting
- `dry_run: bool` - Dry run mode flag
- `stream: bool` - Streaming response flag
- `save: bool` - Save conversations flag
- `keybindings: String` - Keybinding style (e.g., "emacs")
- `editor: Option<String>` - External editor command
- `wrap: Option<String>` - Text wrapping settings
- `wrap_code: bool` - Code wrapping flag
- `function_calling: bool` - Enable function calling
- `mapping_tools: IndexMap<String, String>` - Tool mappings
- `use_tools: Option<String>` - Tools to use
- `repl_prelude: Option<String>` - REPL initialization
- `cmd_prelude: Option<String>` - Command mode prelude
- `agent_prelude: Option<String>` - Agent prelude
- `save_session: Option<bool>` - Session saving preference
- `compress_threshold: usize` - Message compression threshold
- `summarize_prompt: Option<String>` - Summarization prompt
- `summary_prompt: Option<String>` - Summary generation prompt
- `rag_embedding_model: Option<String>` - RAG embedding model
- `rag_reranker_model: Option<String>` - RAG reranker model
- `rag_top_k: usize` - RAG top-k results
- `rag_chunk_size: Option<usize>` - RAG chunk size
- `rag_chunk_overlap: Option<usize>` - RAG chunk overlap
- `rag_template: Option<String>` - RAG template
- `document_loaders: HashMap<String, String>` - Document loaders
- `highlight: bool` - Syntax highlighting flag
- `theme: Option<String>` - Color theme
- `left_prompt: Option<String>` - REPL left prompt
- `right_prompt: Option<String>` - REPL right prompt
- `serve_addr: Option<String>` - Server address
- `user_agent: Option<String>` - HTTP user agent
- `save_shell_history: bool` - Shell history saving
- `sync_models_url: Option<String>` - Model sync URL
- `clients: Vec<ClientConfig>` - LLM client configurations

### Runtime State Fields (marked with `#[serde(skip)]`)
- `macro_flag: bool` - Macro execution flag
- `info_flag: bool` - Info mode flag
- `agent_variables: Option<AgentVariables>` - Agent variables
- `model: Model` - Current model instance
- `functions: Functions` - Available functions
- `working_mode: WorkingMode` - Current working mode (Cmd/Repl/Serve)
- `last_message: Option<LastMessage>` - Last message info
- `role: Option<Role>` - Active role
- `session: Option<Session>` - Active session
- `rag: Option<Arc<Rag>>` - RAG instance
- `agent: Option<Agent>` - Active agent

## Global State Access Pattern

The global config is initialized once in `main.rs`:
```rust
let config = Arc::new(RwLock::new(Config::init(working_mode, info_flag).await?));
```

It's then passed throughout the application and accessed via:
- `config.read()` - Read access (150+ occurrences)
- `config.write()` - Write access (for mutations)

## Components Depending on GlobalConfig

Files that depend on `GlobalConfig` or `Arc<RwLock<Config>>`:
1. `src/main.rs` - Initialization and orchestration
2. `src/serve.rs` - HTTP server
3. `src/repl/mod.rs` - REPL interface
4. `src/function.rs` - Function calling
5. `src/rag/mod.rs` - RAG functionality
6. `src/render/mod.rs` - Rendering
7. `src/config/agent.rs` - Agent management
8. `src/config/input.rs` - Input processing
9. `src/config/mod.rs` - Config management itself
10. `src/client/common.rs` - Client utilities
11. `src/client/macros.rs` - Macro processing
12. `src/repl/completer.rs` - REPL completion
13. `src/repl/highlighter.rs` - Syntax highlighting
14. `src/repl/prompt.rs` - REPL prompts

## State Mutations

Key areas where state is mutated:
1. **Model switching** - `set_model()`, `set_model_id()`
2. **Role management** - `set_role()`, `clear_role()`
3. **Session management** - `set_session()`, `end_session()`
4. **Agent operations** - `use_agent()`, `exit_agent()`
5. **RAG operations** - `set_rag()`, `rebuild_rag()`
6. **Runtime flags** - `set_wrap()`, `set_temperature()`, etc.

## File-Based Persistence

Configuration is persisted to:
- Main config: `~/.config/aichat/config.yaml` (or custom via env)
- Agent configs: `~/.config/aichat/agents/<name>/config.yaml`
- Sessions: `~/.config/aichat/sessions/`
- Roles: Built-in from assets + `~/.config/aichat/roles/`

## Thread Safety and Synchronization

The `Arc<RwLock<Config>>` pattern provides:
- Thread-safe shared access
- Multiple concurrent readers
- Exclusive write access
- But creates global state coupling

## Analysis: Global vs Instance-Specific State

### Truly Global Configuration
- Client configurations (API keys, endpoints)
- Default model preferences
- UI preferences (theme, highlighting, keybindings)
- System paths and directories

### Should Be Instance-Specific
- `model: Model` - Current model
- `role: Option<Role>` - Active role
- `session: Option<Session>` - Active session
- `agent: Option<Agent>` - Active agent
- `rag: Option<Arc<Rag>>` - RAG instance
- `functions: Functions` - Available functions
- `working_mode: WorkingMode` - Mode of operation
- `last_message: Option<LastMessage>` - Message tracking
- `agent_variables: Option<AgentVariables>` - Agent state

### Hybrid (Context-Dependent)
- Temperature, top_p - Could be global defaults or instance overrides
- Function calling settings - Global capability vs instance usage
- Save preferences - Global default vs session-specific

## Refactoring Strategy for Library Use

### 1. Split Config into Two Structures

```rust
// Global configuration (shared across instances)
pub struct GlobalConfig {
    // API configurations
    clients: Vec<ClientConfig>,
    
    // System paths
    config_dir: PathBuf,
    cache_dir: PathBuf,
    
    // Global defaults
    default_model_id: String,
    default_temperature: Option<f64>,
    default_top_p: Option<f64>,
    
    // UI preferences (for CLI use)
    theme: Option<String>,
    keybindings: String,
    highlight: bool,
}

// Instance-specific state
pub struct AgentState {
    // Active components
    model: Model,
    role: Option<Role>,
    session: Option<Session>,
    agent: Option<Agent>,
    rag: Option<Arc<Rag>>,
    functions: Functions,
    
    // Runtime state
    working_mode: WorkingMode,
    last_message: Option<LastMessage>,
    agent_variables: Option<AgentVariables>,
    
    // Instance overrides
    temperature: Option<f64>,
    top_p: Option<f64>,
    stream: bool,
    save: bool,
}
```

### 2. Create Agent Context

```rust
pub struct AgentContext {
    global_config: Arc<GlobalConfig>,
    state: RwLock<AgentState>,
}
```

### 3. Eliminate Global State Access

- Pass `AgentContext` explicitly to all functions
- Remove all `config.read()` and `config.write()` calls
- Make functions accept context parameters

### 4. Configuration for Cymbiont

Cymbiont should control:
- Which models to expose
- Function/tool availability
- Session management policies
- RAG configuration
- Agent definitions
- Temperature/sampling defaults

### 5. Support Multiple Concurrent Instances

```rust
// Library API
pub struct AIChatLibrary {
    global_config: Arc<GlobalConfig>,
}

impl AIChatLibrary {
    pub fn new_agent(&self) -> Agent {
        Agent::new(self.global_config.clone())
    }
}
```

## Key Challenges

1. **Pervasive Global Access** - 150+ locations access global config
2. **Mixed Concerns** - Config struct mixes configuration with runtime state
3. **Implicit Dependencies** - Many functions assume global config availability
4. **File I/O Coupling** - Direct file system access throughout
5. **Session/Role/Agent State** - Tightly coupled to global config

## Recommendations

1. **Phase 1**: Create parallel structures without breaking existing code
2. **Phase 2**: Gradually migrate components to use new structures
3. **Phase 3**: Remove global state access patterns
4. **Phase 4**: Create clean library API
5. **Phase 5**: Adapt Cymbiont to use library API

The refactoring will require systematic changes throughout the codebase, but will result in a much more flexible and library-friendly architecture.