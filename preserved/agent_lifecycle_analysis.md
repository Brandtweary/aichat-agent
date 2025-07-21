# Agent Lifecycle Analysis in AIChat

## Overview

The Agent system in AIChat is a sophisticated feature that allows for specialized AI assistants with custom configurations, functions, RAG (Retrieval-Augmented Generation) support, and dynamic instructions. Agents are more powerful than simple roles - they can maintain state, use custom functions, and have their own RAG databases.

## Agent Struct Definition

Located in `src/config/agent.rs`, the Agent struct contains:

```rust
pub struct Agent {
    name: String,                                    // Agent identifier
    config: AgentConfig,                            // Runtime configuration
    definition: AgentDefinition,                    // Static definition from index.yaml
    shared_variables: AgentVariables,               // Variables shared across sessions
    session_variables: Option<AgentVariables>,      // Session-specific variables
    shared_dynamic_instructions: Option<String>,    // Cached dynamic instructions (shared)
    session_dynamic_instructions: Option<String>,   // Session-specific dynamic instructions
    functions: Functions,                           // Agent-specific functions
    rag: Option<Arc<Rag>>,                         // Optional RAG database
    model: Model,                                   // LLM model to use
}
```

### Key Components:

1. **AgentConfig**: Runtime configuration including model settings, temperature, use_tools, and agent_prelude
2. **AgentDefinition**: Static definition loaded from `index.yaml` containing instructions, variables, conversation starters, and documents
3. **AgentVariables**: Key-value pairs that can be interpolated into prompts using `{{variable_name}}` syntax
4. **Functions**: Agent-specific functions loaded from `functions.json`
5. **RAG**: Optional Retrieval-Augmented Generation database for document-based Q&A

## Agent Creation and Loading

### 1. **Agent Initialization** (`Agent::init`)

The agent loading process follows these steps:

```rust
pub async fn init(
    config: &GlobalConfig,
    name: &str,
    abort_signal: AbortSignal,
) -> Result<Self>
```

1. **Directory Structure**:
   - Functions directory: `~/.config/aichat/functions/agents/{agent_name}/`
   - Contains: `index.yaml` (definition), `functions.json` (optional), documents
   - Data directory: `~/.config/aichat/agents/{agent_name}/`
   - Contains: `config.yaml` (optional), `rag.yaml` (RAG database)

2. **Loading Process**:
   - Load `index.yaml` for AgentDefinition
   - Load `config.yaml` if exists, otherwise use defaults from GlobalConfig
   - Load `functions.json` for agent-specific functions
   - Replace `{{__tools__}}` placeholder in instructions with function list
   - Load environment variables with pattern `{AGENT_NAME}_{SETTING}`
   - Initialize or load RAG if `rag.yaml` exists or documents are specified

3. **Model Selection**:
   - Use agent-specific model if configured
   - Otherwise inherit from global config
   - Temperature and top_p can be agent-specific

### 2. **Agent Usage** (`Config::use_agent`)

```rust
pub async fn use_agent(
    config: &GlobalConfig,
    agent_name: &str,
    session_name: Option<&str>,
    abort_signal: AbortSignal,
) -> Result<()>
```

1. **Prerequisites**:
   - Function calling must be enabled
   - No other agent can be active (must exit first)

2. **Activation Process**:
   - Initialize the agent
   - Set agent's RAG as the active RAG
   - Store agent in config
   - Either start a session or initialize shared variables
   - If `agent_prelude` is set, it's used as the default session name

## Message Handling and Conversation Flow

### 1. **Input Processing**

When creating input with an active agent:

```rust
// In Input::from_str and Input::from_files
let (role, with_session, with_agent) = resolve_role(&config.read(), role);
```

- `with_agent` flag is set when an agent is active
- Agent's role is used for message building
- Input can access agent state through the config

### 2. **Message Building**

Agents implement the `RoleLike` trait:

```rust
impl RoleLike for Agent {
    fn to_role(&self) -> Role {
        let prompt = self.interpolated_instructions();
        let mut role = Role::new("", &prompt);
        role.sync(self);
        role
    }
}
```

- Agent instructions are interpolated with variables
- Dynamic instructions can override static ones
- Session-specific instructions take precedence over shared ones

### 3. **Conversation Context**

The agent's context is built through:

1. **Instructions Hierarchy**:
   - Session dynamic instructions (highest priority)
   - Shared dynamic instructions
   - Config instructions
   - Definition instructions (lowest priority)

2. **Variable Interpolation**:
   - Variables are replaced in instructions using `{{variable_name}}` syntax
   - Session variables override shared variables
   - Environment variables are also interpolated

## Function and RAG Integration

### 1. **Function Selection** (`Config::select_functions`)

```rust
pub fn select_functions(&self, role: &Role) -> Option<Vec<FunctionDeclaration>>
```

When an agent is active:
- Agent functions are prioritized
- Global functions are added if not overridden by agent functions
- Functions marked with `agent: true` are filtered out from the list
- The `use_tools` setting controls which functions are available

### 2. **RAG Integration**

- Agents can have their own RAG database at `{agent_data_dir}/rag.yaml`
- RAG is automatically initialized if documents are specified in `index.yaml`
- When agent is active, its RAG becomes the global RAG
- RAG is used for embeddings-based retrieval during conversations

## State Management

### 1. **Variable Management**

Agents maintain two levels of variables:
- **Shared Variables**: Persist across sessions
- **Session Variables**: Specific to current session

Variables can be:
- Set via CLI: `--agent myagent key=value`
- Set via REPL: `.agent myagent session-name key=value`
- Loaded from environment: `{AGENT_NAME}_VARIABLES`
- Defined with defaults in `index.yaml`

### 2. **Dynamic Instructions**

- Agents can have dynamic instructions generated by `_instructions` function
- Cached at shared and session levels
- Regenerated on demand or when forced

### 3. **Session Integration**

- Agents can work with or without sessions
- With session: Full conversation history with agent context
- Without session: One-shot interactions with agent capabilities
- `agent_prelude` can specify a default session name

## Lifecycle Events

### 1. **Agent Start**
1. Load configuration and definition
2. Initialize functions and RAG
3. Set up variables (prompt for missing required ones)
4. Cache dynamic instructions if needed
5. Start session if specified

### 2. **During Conversation**
1. Agent instructions are used as system prompt
2. Functions are available based on `use_tools` setting
3. RAG is queried if relevant
4. Variables are interpolated in real-time

### 3. **Agent Exit**
1. `.exit agent` command or `exit_agent()` method
2. Session is closed if active
3. RAG reference is cleared
4. Agent state is removed from config
5. Returns to default role

## Hooks for Cymbiont Integration

The agent system provides several hooks where Cymbiont can inject context:

1. **Instruction Injection**:
   - Override `instructions` in AgentConfig
   - Provide dynamic instructions via `_instructions` function
   - Use variable interpolation for dynamic content

2. **Function Integration**:
   - Add Cymbiont-specific functions to agent's function set
   - Override function selection logic
   - Implement custom function handlers

3. **RAG Enhancement**:
   - Extend RAG with PKM knowledge graph data
   - Override embedding search behavior
   - Inject additional context from knowledge graph

4. **State Management**:
   - Add custom variables programmatically
   - Maintain Cymbiont-specific state in agent variables
   - Hook into session creation/destruction

5. **Message Pipeline**:
   - Intercept message building process
   - Add context before sending to LLM
   - Post-process responses

## Key Insights for Library Conversion

1. **Agent as Context Manager**: Agents essentially manage the full context window through their instructions, functions, and RAG integration.

2. **Flexible Architecture**: The agent system is already designed for extensibility with clear separation between definition, configuration, and runtime state.

3. **Session Integration**: Agents work seamlessly with sessions, allowing for both stateful and stateless interactions.

4. **Dynamic Behavior**: Dynamic instructions and variable interpolation provide runtime flexibility without code changes.

5. **Clear Lifecycle**: Well-defined initialization, runtime, and cleanup phases make it easy to hook into the agent lifecycle.

The agent system provides a solid foundation for Cymbiont to build upon, with natural integration points for knowledge graph functionality and enhanced context management.