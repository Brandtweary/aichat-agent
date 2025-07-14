# Cyberorganism Architecture

A guide to core modules, system design, and data flow for developers.

## Recent Updates

### Documentation Overhaul
**Status**: Completed comprehensive documentation update
- **Architecture Document**: Created `cyberorganism_architecture.md` to provide clear system overview and component relationships
- **Developer Guide**: Established `CLAUDE.md` with essential build commands and development best practices
- **TODO Tracking**: Organized future improvements in `notes/PKM_Knowledge_Graph_TODO.md` with `#maybe` tags for uncertain items

## System Overview

Cyberorganism is a self-organizing knowledge graph agent that transforms personal knowledge management systems into queryable, intelligent networks. Built on top of AIChat, it maintains clean separation between the base LLM functionality and knowledge graph extensions.

## Repository Layout

```
cyberorganism/
├── src/                           # AIChat core (minimal modifications)
│   ├── main.rs                    # CLI entry point
│   ├── config/                    # Configuration management
│   ├── client/                    # LLM provider integrations
│   ├── rag/                       # RAG implementation
│   └── function/                  # Function calling framework
├── extensions/                    # Cyberorganism-specific features
│   ├── pkm_knowledge_graph/       # Knowledge graph integration
│   │   ├── index.js               # Logseq plugin entry point
│   │   ├── api.js                 # Backend communication layer
│   │   ├── data_processor.js      # Data validation and processing
│   │   ├── config.js              # Configuration loader
│   │   ├── backend/               # Rust knowledge graph server
│   │   │   ├── src/
│   │   │   │   ├── main.rs        # HTTP server and endpoints
│   │   │   │   └── pkm_datastore.rs  # Graph storage (migrating to petgraph)
│   │   │   └── Cargo.toml
│   │   └── package.json
│   └── logseq_dummy_graph/        # Test data
└── notes/                         # Architecture documentation
    └── PKM_Knowledge_Graph_TODO.md
```

## Core Components

### AIChat Base (src/)
The foundation provides CLI interface, multi-provider LLM support, RAG capabilities, and function calling. Cyberorganism preserves all AIChat functionality while adding knowledge graph capabilities through the extension system.

### PKM Knowledge Graph Extension

**JavaScript Frontend (Logseq Plugin)**
- **index.js**: Orchestrates plugin lifecycle, handles Logseq events, manages sync logic
- **api.js**: Provides HTTP communication with backend server
- **data_processor.js**: Validates and transforms Logseq data before transmission

**Rust Backend Server**
- **main.rs**: HTTP server with RESTful endpoints:
  - `POST /data`: Receives blocks/pages from Logseq
  - `GET /sync/status`: Returns sync state and statistics
  - `PATCH /sync`: Updates sync timestamp (RESTful design)
- **pkm_datastore.rs**: Current JSON-based storage layer maintaining nodes, references, and mappings (planned migration to petgraph for direct graph operations)
- **Logging**: Uses tracing crate with custom formatter optimized for LLM readability (no colors, minimal context)

**Operation Notes**
- Backend server must be running before loading the Logseq plugin
- Empty blocks are intentionally skipped during sync (not treated as errors)
- Individual changes sync immediately; full sync runs every 2 hours to catch offline edits

**Process Management**
The backend server automatically manages its lifecycle:
- On startup, checks for `pkm_knowledge_graph_server.json` file
- If found, reads the PID and sends SIGTERM to terminate the previous instance
- Writes new server info (PID, host, port) to the JSON file
- On shutdown (Ctrl+C or normal exit), removes the server info file
- If the configured port is busy, automatically tries alternative ports (3001, 3002, etc.)
- The JavaScript plugin reads the server info file to discover the actual port in use
- No manual process management needed - just run `cargo run` to start fresh

## Data Flow

### Real-time Sync
```
Logseq DB Change → onChanged Event → Validate Data → Batch Queue → HTTP POST → Backend Processing
```

### Full Sync (Every 2 Hours)
```
Check Timestamps → Query All Pages/Blocks → Process in Batches → Update Backend → Update Sync Timestamp
```

### Data Structures
- **Nodes**: Pages and blocks with content, timestamps, and properties
- **References**: Page links, block references, tags, and parent-child relationships
- **Mappings**: PKM IDs to internal node IDs for consistent graph structure

## Configuration

**Main Config** (`config.yaml`):
- LLM provider settings
- Model selection
- API keys

**PKM Extension Config** (`extensions/pkm_knowledge_graph/config.yaml`):
- Port configuration for the backend server
- Server always binds to localhost for security
- See config.yaml file for current options


## Planned Architecture Changes

### Petgraph Migration
- Remove intermediate JSON datastore
- Store all node/edge data directly in petgraph
- Implement efficient graph algorithms
- Add periodic serialization for persistence

This will simplify the architecture from:
```
Logseq → JS → Rust → JSON Datastore → Query Layer
```
To:
```
Logseq → JS → Rust → Petgraph (in-memory) → Serialization
```

## Testing

- **JavaScript**: `npm test` - Jest test suite with 23 tests covering data validation and reference extraction (silent by default)
- **Rust**: `cargo test` - Backend component tests (quiet by default via .cargo/config.toml)
- **Integration**: RESTful endpoint tests and logseq_dummy_graph for development
- **Development**: `cargo run -- --duration 3` - Run backend server for testing (auto-exits after specified seconds)