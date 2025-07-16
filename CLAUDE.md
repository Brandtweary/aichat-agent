# CYBERORGANISM DEVELOPMENT GUIDE

## Build/Test Commands
```bash
# In cyberorganism root
cargo build                      # Build cyberorganism
cargo test                       # Run all tests

# In extensions/pkm_knowledge_graph/backend/
cargo check                      # Quick syntax check
cargo build                      # Build backend server
cargo test                       # Run tests (quiet by default)
RUST_LOG=info cargo run          # Run backend server (uses default 3s duration from config)
RUST_LOG=info cargo run -- --duration 10  # Override duration when needed
```

## Architecture
- See `cyberorganism_architecture.md` for comprehensive codebase architecture

### Core Directories
- **src/**: AIChat core - LLM providers, CLI, RAG, function calling (minimize changes)
- **extensions/**: Cyberorganism-specific features
  - **pkm_knowledge_graph/**: Transforms PKM tools into queryable knowledge graphs
    - JavaScript: Logseq plugin for real-time sync
    - **backend/**: Rust server managing graph operations
  - **logseq_dummy_graph/**: Test data for development

### PKM Backend Structure (extensions/pkm_knowledge_graph/backend/)
- **src/**
  - **main.rs**: Server orchestration, lifecycle management, Logseq launching
  - **api.rs**: HTTP endpoints and request handlers for data sync
  - **graph_manager.rs**: Petgraph-based knowledge graph storage engine
  - **config.rs**: YAML configuration loading and validation
  - **utils.rs**: Process management, datetime parsing, general utilities
  - **logging.rs**: Custom formatter (file:line only for ERROR/WARN)
  - **pkm_data.rs**: Shared data structures (PKMBlockData, PKMPageData)
- **data/**: Graph persistence (knowledge_graph.json)
- **tests/**: Integration tests
- **Cargo.toml**: Dependencies and metadata

## Codebase Guidelines
- Rust backend: use `error!()`, `warn!()`, `info!()`, `debug!()`, `trace!()` macros for logging (tracing crate)
- JS plugin: use `KnowledgeGraphAPI.log.error/warn/info/debug/trace()` to send logs to the Rust server
- Don't make live LLM calls during tests

## Development Best Practices

### Read Files Completely

- When working with a file for the first time in a conversation, read it in its entirety before making changes
- Avoid hunting through large files with grep when a full read would be more efficient

### Clean Console Output

- Maintain clean console output: remove debug logs after troubleshooting.
- Do not add debug logging inside hot paths which will flood the console.
- Optimize logging levels if output becomes overwhelming.
- Filter logs with grep if you only need to locate specific messages.
- When reviewing logs, make sure to point out ANY warnings or errors. The user is NOT reading these logs, it is your responsibility to report issues.

### Fail-Fast During Feature Development

- **Prototype without fallbacks**: When developing new features, avoid default values or fallback mechanisms that mask underlying issues.
- **Explicit error handling**: Let failures be loud and visible during initial implementation - don't silently continue on errors.
- **No backwards compatibility**: Keeping deprecated code creates confusion and adds developer burden. Remove old code paths decisively.

### Eliminate Dead Code

- **Case-by-case evaluation**: Never blindly remove dead code without understanding its context in the larger codebase.
- **Consider multiple scenarios**: For each dead code instance, evaluate possible underlying causes (e.g., planned features that were forgotten, logic that got inlined elsewhere, or remains of deleted features requiring git history investigation).
- **YAGNI Principle**: "You Ain't Gonna Need It" - Only keep what you actually need right now. Avoid building for imagined future requirements.
- **Use `#[allow(dead_code)]` sparingly**: Only when the user explicitly confirms code is kept for forward-compatibility.
- **Use `#[cfg(test)]` for test code caught by the dead code checker**: Consider that there is usually a cleaner solution for this, such as using a test fixture.
- **NEVER prefix unused variables with underscores**: This makes it impossible to locate dead code later. Always investigate WHY code is unused first.

### End of the Dance: Identify and Fix Root Causes

- **Root cause analysis**: Thoroughly investigate and identify exactly what's causing a bug before implementing solutions.
- **No compensatory features**: Do NOT add new features as band-aids to work around bugs without proving they're necessary first. For example, don't add a checksum without first showing that the underlying data corruption isn't fixable at the source.
- **Minimal, targeted fixes**: Once the root cause is identified, implement the smallest possible fix.
- **Demand concrete proof**: Always insist on measurement and verification before implementing solutions - avoid endlessly theorizing about abstract causes.

## Continuous Documentation

- Keep the architecture document (`cyberorganism_architecture.md`) up to date
- When updating documentation, read it entirely first to avoid redundancy
