# Cyberorganism Architecture

A guide to core modules, system design, and data flow for developers.

## Recent Updates

### Backend Module Refactoring Complete
**Status**: Major refactoring to improve code organization and maintainability
- **Module Extraction**: Extracted functionality from monolithic main.rs into focused modules
- **New Module Structure**:
  - `config.rs`: Configuration management, YAML loading, and JavaScript config validation
  - `logging.rs`: Custom tracing formatter that shows file:line only for ERROR/WARN levels
  - `api.rs`: Consolidated API types, handlers, and router configuration
  - `utils.rs`: Cross-cutting utilities including process management, datetime parsing, JSON helpers
- **Test Coverage**: Added unit tests across modules for regression prevention
- **Maintained Functionality**: All code migrated exactly with no functional changes
- **Improved Architecture**: Clear separation of concerns with focused, maintainable modules

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
│   │   │   │   ├── main.rs        # HTTP server orchestration
│   │   │   │   ├── config.rs      # Configuration management
│   │   │   │   ├── logging.rs     # Custom tracing formatter
│   │   │   │   ├── api.rs         # API types, handlers, routes
│   │   │   │   ├── utils.rs       # Utility functions
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
  - Sends message types to backend: 'block', 'blocks', 'page', 'pages', 'plugin_initialized'
  - Monitors DB changes via `logseq.DB.onChanged` and batches updates
  - Handles full database sync every 2 hours
- **api.js**: HTTP communication layer (exposed as `window.KnowledgeGraphAPI`)
  - `sendToBackend(data)`: Sends data to POST /data endpoint, returns boolean
  - `sendBatchToBackend(type, batch, graphName)`: Wrapper for batch operations, formats as `${type}_batch`
  - `log.error/warn/info/debug/trace(message, details, source)`: Sends logs to POST /log endpoint
  - `checkBackendAvailabilityWithRetry(maxRetries, delayMs)`: Health check with retries (used before sync)
  - Port discovery (tries 3000-3010), sync status queries
  - Full API documentation in the module header comments of api.js
- **data_processor.js**: Validates and transforms Logseq data before transmission
  - Processes blocks and pages into standardized format
  - Extracts references (page refs, block refs, tags)

**Rust Backend Server**
- **main.rs**: HTTP server orchestration and application state management
  - Manages server lifecycle, AppState, and high-level control flow
  - Coordinates with extracted modules for specific functionality
  - Handles Logseq launching and process termination
- **config.rs**: Configuration management module
  - Loads configuration from `config.yaml` with fallback to defaults
  - Validates JavaScript plugin configuration matches Rust settings
  - Provides Config, BackendConfig, LogseqConfig, DevelopmentConfig structs
- **logging.rs**: Custom logging configuration
  - Implements ConditionalLocationFormatter for cleaner log output
  - Shows file:line information only for ERROR and WARN levels
- **api.rs**: Consolidated API implementation
  - API types: ApiResponse, PKMData, LogMessage
  - All endpoint handlers: root, receive_data, sync operations, logging
  - Router configuration via create_router()
  - Helper functions for data processing and batch operations
- **utils.rs**: Cross-cutting utility functions
  - Logseq executable discovery (Windows/macOS/Linux) and process launching
  - Process management: port checking, server info file, previous instance termination
  - DateTime parsing with multiple format support (RFC3339, ISO 8601, Unix timestamps)
  - JSON utilities: generic deserialization, JSON-to-HashMap conversion

**API Endpoints** (unchanged):
  
  **Endpoints:**
  - `GET /` - Health check endpoint
    - Returns: `"PKM Knowledge Graph Backend Server"`
    - Used by JavaScript plugin to verify server availability
  
  - `POST /data` - Main data ingestion endpoint
    - Accepts: `PKMData` JSON object with fields:
      - `source`: String identifying data origin
      - `timestamp`: String timestamp
      - `type_`: Optional string determining processing logic
      - `payload`: String containing the actual data (usually stringified JSON)
    - Type values and their payloads:
      - `"block"` - Single PKMBlockData object
      - `"blocks"` or `"block_batch"` - Array of PKMBlockData objects
      - `"page"` - Single PKMPageData object  
      - `"pages"` or `"page_batch"` - Array of PKMPageData objects
      - `"plugin_initialized"` - Plugin startup notification
      - `null/other` - Generic acknowledgment (used for real-time sync)
    - Returns: `ApiResponse` with `success: bool` and `message: string`
  
  - `GET /sync/status` - Sync status and graph statistics
    - Returns: JSON object with:
      - `last_full_sync`: ISO timestamp string or null
      - `hours_since_sync`: Float hours since last sync
      - `full_sync_needed`: Boolean (true if >2 hours or never synced)
      - `node_count`: Total nodes in graph
      - `reference_count`: Total edges in graph
  
  - `PATCH /sync` - Update sync timestamp
    - Called after successful full database sync
    - Updates internal timestamp used for sync scheduling
    - Returns: `ApiResponse` with success status
  
  - `POST /log` - Logging endpoint for JavaScript plugin
    - Accepts: `LogMessage` JSON object with:
      - `level`: String ("error", "warn", "info", "debug", "trace")
      - `message`: String log message
      - `source`: Optional string identifying log source
      - `details`: Optional JSON value with additional context
    - Maps JavaScript log levels to Rust tracing macros
    - Returns: `ApiResponse` confirming receipt
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
- **Logseq Auto-Launch**: If `auto_launch: true` in config.yaml, the server will:
  - Search for Logseq executable in common locations (Linux/macOS/Windows support)
  - Launch Logseq after server starts and wait for plugin initialization
  - Filter Electron/xdg-mime logs to trace level to keep console clean
  - Terminate Logseq gracefully on server shutdown
  - Custom executable path can be specified via `executable_path` config option

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

## Testing

- **JavaScript**: `npm test` - Jest test suite covering data validation and reference extraction (silent by default)
- **Rust**: `cargo test` - Backend unit tests for core modules (quiet by default via .cargo/config.toml)
- **Development**: `RUST_LOG=debug cargo run` - Run backend server with default 3-second duration for testing

## Development Features

**Graceful Shutdown System:**
- Server waits for sync operations to complete before shutting down
- Protects against data corruption from interrupted batch operations
- Uses Axum's graceful shutdown to handle in-flight HTTP requests
- 10-second timeout prevents indefinite hangs

**Development Duration Configuration:**
- `development.default_duration: 3` in config.yaml sets automatic exit timer
- Prevents servers from running indefinitely during development workflows
- CLI `--duration X` overrides config default when needed
- Production builds warn if `default_duration` is not null (should be null for production)
- Graceful shutdown ensures sync operations complete before timer expires