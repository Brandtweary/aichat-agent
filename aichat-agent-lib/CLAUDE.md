# AICHAT-AGENT LIBRARY DEVELOPMENT GUIDE

## Current Task
Converting AIChat CLI into a reusable Rust library that exposes agent functionality with full programmatic control. See `preserved/feature_taskpad_architectural_overhaul.md` for details.

## Build Commands
```bash
cargo check              # Quick syntax check
cargo build              # Build project
cargo test               # Run tests
cargo doc --open         # View documentation
```

## Development Best Practices

### Read Files Completely
- Always read entire files before making changes
- Avoid grep hunting when full context would help

### Clean Console Output
- Remove temporary debug logs before committing
- Report ANY warnings or errors to the user
- NEVER filter cargo check output - show everything

### Fail-Fast During Development
- Let failures be loud and visible
- No silent error handling
- YAGNI: Only build what's needed now

### Eliminate Dead Code
- Understand context before removing
- Use `#[allow(dead_code)]` sparingly
- NEVER prefix unused variables with underscores

### Root Cause Analysis
- Investigate thoroughly before implementing fixes
- Demand concrete proof, not theories
- No band-aid features

## Basic AIChat Usage
```bash
aichat "Hello"                          # Basic query
aichat -m claude-3-opus "Explain X"     # Specific model
aichat                                  # Interactive REPL
aichat -A coder "Write Python"          # With agent
```

See [AIChat README](./README.md) for full documentation.