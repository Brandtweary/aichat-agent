# Feature Taskpad: Architectural Overhaul

## Feature Description
Transform the current Cymbiont fork into a focused Rust library called "aichat-agent" that exposes only AIChat's agent functionality, and create a standalone Cymbiont repository that uses this library directly. Cymbiont will import AIChat's agent functionality as a library, control the agent loop using AIChat's internals, and massively extend it with knowledge graph integration. No HTTP overhead, direct function calls, and complete control over agent behavior while leveraging AIChat's LLM provider abstractions.

## Specifications
- Current fork becomes "aichat-agent" - focused library exposing only agent functionality
- New standalone "cymbiont" repository imports this library
- No HTTP/server infrastructure - direct Rust function calls
- Minimal API surface: agent creation, tool registration, config, LLM client access
- Cymbiont controls agent loop but uses AIChat's internals
- Fork maintained as git submodule for easy source inspection
- Both RAG (from AIChat) and KG (from cymbiont) available for comparison
- No general-purpose API - specifically designed for Cymbiont's needs
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

### Cymbiont Extensions (to be migrated)
- `extensions/pkm_knowledge_graph/backend/`: Core KG implementation
- `extensions/pkm_knowledge_graph/frontend/`: Logseq plugin
- `cymbiont_architecture.md`: Documentation
- Current usage: To become the core of standalone cymbiont

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
- [ ] Document all cymbiont-specific changes
- [ ] Tag current state as "pre-cymbiont-migration"
- [ ] Create new feature branch for library conversion
- [ ] **IMPORTANT**: Restore fork to upstream AIChat state
  - The current fork has too many cymbiont-specific changes
  - Creating a thin API wrapper requires starting fresh from upstream
  - All cymbiont changes will be migrated to the new cymbiont repo separately
- [ ] Reset the feature branch to upstream AIChat main branch

### 4. Library Conversion (on clean upstream code)
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
- [x] Create new "cymbiont" repository on GitHub (completed: https://github.com/Brandtweary/cymbiont.git)
- [ ] Update local git remotes to point to aichat-agent
- [ ] Add aichat-agent as git submodule in cymbiont
- [ ] Configure Cargo.toml to use local path dependency
- [ ] Set up workspace if using multiple crates
- [ ] Test submodule workflow

### 6. Cymbiont Core Implementation (separate from library work)
- [ ] **Migration Strategy**:
  - Create temporary `cymbiont/` folder inside aichat-agent root (no naming conflicts)
  - COPY files (don't move) to preserve originals during migration
  - Connect to remote cymbiont repo once satisfied
  - Move folder outside aichat-agent when ready (cymbiont will contain aichat-agent as submodule)
- [ ] **New Repository Structure**:
  ```
  cymbiont/
  ├── src/                    # Backend server code
  ├── tests/                  # Backend tests
  ├── data/                   # Knowledge graph persistence
  │   └── archived_nodes/     # Deleted node archives
  ├── logseq_plugin/          # Renamed from frontend/
  ├── logseq_databases/       # Test graphs and multi-graph support
  │   └── dummy_graph/        # Current test graph
  ├── aichat-agent/           # Submodule of the library fork
  ├── Cargo.toml              
  ├── Cargo.lock
  ├── config.yaml
  ├── config.example.yaml
  ├── cymbiont_architecture.md
  ├── CLAUDE.md
  ├── README.md               # Complete rewrite focusing on KG agent
  └── .gitignore
  ```
- [ ] **Migration Strategy (Simplified)**:
  - [ ] Copy entire backend/* contents to cymbiont/
  - [ ] Copy entire frontend/* to cymbiont/logseq_plugin/
  - [ ] Copy entire logseq_dummy_graph/* to cymbiont/logseq_databases/dummy_graph/
  - [ ] Copy cymbiont_architecture.md to cymbiont/
  - [ ] Copy CLAUDE.md to cymbiont/
  - [ ] Create new README.md from scratch
  - [ ] Add .gitignore (can adapt from current one)
- [ ] **Post-Migration Code Updates**:
  - [ ] Update hardcoded file paths for new structure:
    - Backend config.yaml location
    - Server info JSON file path
    - Archive directory path
    - Knowledge graph persistence path
    - Logseq dummy graph references
  - [ ] Adjust imports and module paths
  - [ ] Update config file locations
  - [ ] Fix references to aichat-agent library
  - [ ] Update any relative paths in JavaScript plugin
  - [ ] Ensure stress test generator points to correct paths
- [ ] Import aichat-agent as git submodule
- [ ] Test basic compilation and functionality

### 7. Agent Integration
- [ ] Create cymbiont Agent wrapper
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
- [ ] Create cymbiont user guide
- [ ] Document architecture decisions
- [ ] Add inline code documentation

### 10. Release and Maintenance
- [ ] Clean up any remaining fork artifacts
- [ ] Tag initial library release
- [ ] Create cymbiont v1.0 release
- [ ] Set up CI/CD for both repos
- [ ] Plan upstream contribution strategy
- [ ] Document maintenance workflow

### 11. Worktree Cleanup
- [ ] Remove api-conversion symlink from cymbiont-workspace
- [ ] Prune the api-conversion worktree
- [ ] Verify aichat-agent submodule is working in cymbiont
- [ ] Create new symlinks as needed for parallel development

## Development Notes
- Library approach eliminates HTTP overhead completely
- Direct access to AIChat internals enables tight integration
- Submodule keeps source visible for debugging and learning
- Fork maintenance burden already exists - library adds minimal complexity
- Clear separation: aichat-agent = LLM/agent core, cymbiont = KG enhancement
- No need for server/client architecture when everything runs locally
- Focused API for Cymbiont's specific needs - not general purpose

## Future Tasks
- Performance benchmarking and optimization
- RAG vs KG retrieval comparison study
- Test with all LLM providers
- Create specialized agents for different PKM tasks
- Build advanced KG algorithms for retrieval
- Add support for multiple PKM tools beyond Logseq
- Implement agent collaboration features

## Final Implementation
(To be completed when feature is finished)