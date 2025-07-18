# Feature Taskpad: Architectural Overhaul

## Feature Description
Transform the current Cyberorganism fork into a focused Rust library called "aichat-agent" that exposes only AIChat's agent functionality, and create a standalone Cyberorganism repository that uses this library directly. Cyberorganism will import AIChat's agent functionality as a library, control the agent loop using AIChat's internals, and massively extend it with knowledge graph integration. No HTTP overhead, direct function calls, and complete control over agent behavior while leveraging AIChat's LLM provider abstractions.

## Specifications
- Current fork becomes "aichat-agent" - focused library exposing only agent functionality
- New standalone "cyberorganism" repository imports this library
- No HTTP/server infrastructure - direct Rust function calls
- Minimal API surface: agent creation, tool registration, config, LLM client access
- Cyberorganism controls agent loop but uses AIChat's internals
- Fork maintained as git submodule for easy source inspection
- Both RAG (from AIChat) and KG (from cyberorganism) available for comparison
- No general-purpose API - specifically designed for Cyberorganism's needs
- Zero overhead from serialization or network calls

## Relevant Components

### AIChat Core (to expose as library)
- `src/main.rs`: CLI entry point (to be complemented with lib.rs)
- `src/agent/`: Agent implementation with state management
- `src/client/`: LLM provider integrations (OpenAI, Claude, etc.)
- `src/function/`: Tool/function calling framework
- `src/config/`: Configuration management
- `src/rag/`: RAG functionality to be extended
- Current usage: Internal modules to be made public

### Cyberorganism Extensions (to be migrated)
- `extensions/pkm_knowledge_graph/backend/`: Core KG implementation
- `extensions/pkm_knowledge_graph/frontend/`: Logseq plugin
- `cyberorganism_architecture.md`: Documentation
- Current usage: To become the core of standalone cyberorganism

### Library API Design (new)
- Public traits: Client, Agent, Functions
- Configuration builders for easy setup
- Simplified agent creation and management
- Current usage: New public API surface

## Development Plan

### 1. Research Phase (Priority: HIGH - Do First)
- [ ] Study AIChat's internal architecture and module dependencies
- [ ] Identify which modules need to be made public
- [ ] Understand Agent struct and its lifecycle
- [ ] Map out function/tool calling flow
- [ ] Document Config and global state management
- [ ] Identify potential issues with making internals public
- [ ] Check for any hardcoded CLI assumptions
- [ ] Research how to handle REPL/interactive features

### 2. Library Architecture Design
- [ ] Design minimal public API surface
- [ ] Plan which internal modules to expose
- [ ] Create facade pattern for complex internals if needed
- [ ] Design builder patterns for configuration
- [ ] Plan error handling strategy for library use
- [ ] Consider async/sync API decisions
- [ ] Design agent lifecycle management API

### 3. Fork Preparation
- [ ] Create comprehensive backup of current repository
- [ ] Document all cyberorganism-specific changes
- [ ] Tag current state as "pre-library-conversion"
- [ ] Create branch for library conversion work

### 4. Library Conversion
- [ ] Create src/lib.rs with public exports
- [ ] Make necessary modules public (client, agent, config)
- [ ] Expose key structs and traits
- [ ] Create builder patterns for common use cases
- [ ] Remove or isolate CLI-specific code
- [ ] Handle global state issues (if any)
- [ ] Add library-specific documentation
- [ ] Create examples/ directory with usage patterns

### 5. Git Repository Setup
- [x] Rename current fork to "aichat-agent" (completed: https://github.com/Brandtweary/aichat-agent.git)
- [x] Create new "cyberorganism" repository on GitHub (completed: https://github.com/Brandtweary/cyberorganism.git)
- [ ] Update local git remotes to point to aichat-agent
- [ ] Add aichat-agent as git submodule in cyberorganism
- [ ] Configure Cargo.toml to use local path dependency
- [ ] Set up workspace if using multiple crates
- [ ] Test submodule workflow

### 6. Cyberorganism Core Implementation
- [ ] Initialize standalone Rust project
- [ ] Move extensions/pkm_knowledge_graph/ to new repo
- [ ] Import AIChat library and test basic usage
- [ ] Implement custom agent that uses KG context
- [ ] Create simplified API for agent creation
- [ ] Build KG-enhanced tool set
- [ ] Integrate PKM sync with agent capabilities

### 7. Agent Integration
- [ ] Create cyberorganism Agent wrapper
- [ ] Implement KG-aware context injection
- [ ] Add custom tools for KG queries
- [ ] Build agent loop that leverages both RAG and KG

### 8. Basic Integration Testing
- [ ] Test agent creation through library API
- [ ] Verify basic LLM inference works
- [ ] Test tool/function calling
- [ ] Ensure PKM sync still works
- [ ] Run existing test suites

### 9. Documentation and Polish
- [ ] Write library API documentation
- [ ] Create cyberorganism user guide
- [ ] Document architecture decisions
- [ ] Write migration guide from fork version
- [ ] Create examples for common use cases
- [ ] Document RAG vs KG comparison results
- [ ] Add inline code documentation

### 10. Release and Maintenance
- [ ] Clean up any remaining fork artifacts
- [ ] Tag initial library release
- [ ] Create cyberorganism v1.0 release
- [ ] Set up CI/CD for both repos
- [ ] Plan upstream contribution strategy
- [ ] Document maintenance workflow

## Development Notes
- Library approach eliminates HTTP overhead completely
- Direct access to AIChat internals enables tight integration
- Submodule keeps source visible for debugging and learning
- Fork maintenance burden already exists - library adds minimal complexity
- Clear separation: aichat-agent = LLM/agent core, cyberorganism = KG enhancement
- No need for server/client architecture when everything runs locally
- Focused API for Cyberorganism's specific needs - not general purpose
- Skip PR/issue - Cyberorganism needs deep, non-standard integrations

## Future Tasks
- Performance benchmarking and optimization
- RAG vs KG retrieval comparison study
- Test with all LLM providers
- Create specialized agents for different PKM tasks
- Build advanced KG algorithms for retrieval
- Add support for multiple PKM tools beyond Logseq
- Implement agent collaboration features
- Consider WASM compilation for browser usage

## Final Implementation
(To be completed when feature is finished)