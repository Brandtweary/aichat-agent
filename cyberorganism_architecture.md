# Cyberorganism Architecture

A guide to core modules, system design, and data flow for developers.

## Recent Updates

### Petgraph Integration Complete
**Status**: Replaced JSON datastore with direct petgraph implementation
- **GraphManager Module**: New core component managing knowledge graph using petgraph's StableGraph
- **Direct Graph Storage**: Eliminated intermediate JSON layer - PKM data now stored directly as graph nodes
- **Improved Sync**: Fixed critical bugs preventing block synchronization (0→74 blocks)
- **Batch Optimization**: Disabled auto-save during batch processing to prevent interleaved saves
- **Comprehensive Testing**: 23 JavaScript tests, Rust unit tests, and integration tests all passing

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
│   │   │   │   ├── graph_manager.rs # Petgraph-based knowledge graph storage
│   │   │   │   └── pkm_data.rs     # Data structures and validation
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
  - `POST /data`: Receives blocks/pages from Logseq (supports both individual items and batches)
  - `GET /sync/status`: Returns sync state and graph statistics (node count, edge count)
  - `PATCH /sync`: Updates sync timestamp after full sync
- **graph_manager.rs**: Core graph storage using petgraph:
  - StableGraph structure maintains consistent node indices across modifications
  - Node types: Page and Block with full metadata (content, properties, timestamps)
  - Edge types: PageRef, BlockRef, Tag, Property, ParentChild, PageToBlock
  - HashMap for O(1) PKM ID → NodeIndex lookups
  - Automatic saves: time-based (5 min) or operation-based (10 ops), disabled during batches
  - Graph persistence to `knowledge_graph.json` with full serialization
- **pkm_data.rs**: Shared data structures and validation logic
- **Logging**: Uses tracing crate with conditional formatter (file:line only for WARN/ERROR)

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

### Graph Structure
**Nodes** (petgraph vertices):
- **Page Nodes**: Created from Logseq pages (name, properties, timestamps)
- **Block Nodes**: Created from Logseq blocks (content, properties, parent reference)
- **Tag Nodes**: Automatically created pages from #tags (without # prefix)

**Edges** (typed relationships):
- **PageRef**: Block/page references another page via [[Page Name]]
- **BlockRef**: Block references another block via ((block-id))
- **Tag**: Block/page uses a #tag
- **Property**: Block/page has property key (key:: value creates edge to key page)
- **ParentChild**: Hierarchical relationship between blocks
- **PageToBlock**: Links page to its root-level blocks

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

### Graph Algorithms and Analysis
- Implement Personalized PageRank (PPR) for similarity ranking
- Add graph visualization endpoint for debugging
- CLI flag for graph structure analysis (load, analyze, exit)

### Enhanced Persistence
- Write-Ahead Log (WAL) for crash recovery
- Tiered backup retention (hourly/daily/weekly/monthly snapshots)
- Graph compression for disk storage

### Process Automation
- Automate Logseq launch when backend starts
- Ensure plugin is loaded and enabled automatically
- Handle graceful Logseq shutdown when server stops

## Testing

- **JavaScript**: `npm test` - Jest test suite with 23 tests covering data validation and reference extraction (silent by default)
- **Rust**: `cargo test` - Backend component tests (quiet by default via .cargo/config.toml)
- **Integration**: RESTful endpoint tests and logseq_dummy_graph for development
- **Development**: `cargo run -- --duration 3` - Run backend server for testing (auto-exits after specified seconds)