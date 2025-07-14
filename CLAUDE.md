# CYBERORGANISM DEVELOPMENT GUIDE

## Build/Test Commands
```bash
# In cyberorganism root
cargo check                      # Quick syntax check
cargo build                      # Build cyberorganism
cargo test                       # Run all tests

# In extensions/pkm_knowledge_graph/backend/
cargo build                      # Build backend server
cargo test                       # Run tests (quiet by default)
RUST_LOG=info cargo run          # Run backend server
RUST_LOG=info cargo run -- --duration 3  # Run for 3 seconds (testing/validation), --duration 30 for user operations
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
- **main.rs**: HTTP server with endpoints for data sync and status
- **graph_manager.rs**: Petgraph-based knowledge graph storage engine
- **pkm_data.rs**: Shared data structures and validation logic

## Codebase Guidelines
- Use tracing with appropriate levels (error, warn, etc.) for Rust logging
- API endpoints should be RESTful

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

**Problematic Dead Code Scenarios - Investigate Each Case:**
- **Planned features that were forgotten**: Incomplete implementations that were started but never finished
- **Neglected code paths**: Logic that got accidentally inlined at calling sites, leaving the original function unused
- **Remains of accidentally deleted features**: Uncommon but tricky - requires git history investigation to understand what was lost

**Best Practices:**
- **Case-by-case evaluation**: Never blindly remove dead code without understanding its context in the larger codebase.
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
