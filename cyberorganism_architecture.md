# Cyberorganism Architecture

A guide to core modules, system design, and data flow for developers.

## Recent Updates

### JavaScript Plugin Modularization (Latest)
**Status**: Refactored JavaScript plugin for improved maintainability and code quality
- **Module Extraction**: Separated sync orchestration logic from main plugin file
  - Created `sync.js` module containing all database synchronization functionality (~400 lines)
  - Reduced `index.js` from 892 to 475 lines (47% reduction)
  - Follows browser-compatible module pattern using global window objects
- **Test Infrastructure**: Expanded Jest test coverage
  - Added comprehensive tests for `sync.js` module (tree traversal, sync status logic)
  - Configured ESLint for dead code detection across different environments
  - All tests passing with proper mocking of browser and Logseq APIs
- **Code Quality Improvements**:
  - Removed deprecated functions (`sendDiagnosticInfo`, unused `checkBackendAvailability`)
  - Eliminated slash command for sync status (command-line debugging preferred)
  - ESLint configuration handles browser, Jest, and Node.js environments separately
- **Module Structure**:
  - `sync.js`: Contains `syncDatabase()`, `checkSyncStatus()`, `processBlocksRecursively()`, and tree utilities
  - `index.js`: Retains plugin lifecycle, real-time sync handling, and event management
  - Clear separation between scheduled sync operations and real-time change processing

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
│   │   ├── index.js               # Logseq plugin entry point (orchestration)
│   │   ├── sync.js                # Database synchronization module
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
└── notes/                         # Additional documentation
```

## Core Components

### AIChat Base (src/)
The foundation provides CLI interface, multi-provider LLM support, RAG capabilities, and function calling. Cyberorganism preserves all AIChat functionality while adding knowledge graph capabilities through the extension system.

### PKM Knowledge Graph Extension

**JavaScript Frontend (Logseq Plugin)**
- **index.js**: Plugin lifecycle management and real-time event handling
  - Initializes plugin and verifies module dependencies
  - Monitors DB changes via `logseq.DB.onChanged` for real-time sync
  - Handles route changes and plugin initialization
  - Exposes helper functions to other modules via window globals
  - Manages timestamp queue for block property updates
  - Coordinates between sync operations and real-time changes
- **sync.js**: Database synchronization orchestration module
  - Implements 3-tiered sync system with configurable intervals:
    - Real-time: Individual changes synced immediately (handled by index.js)
    - Incremental: Every 2 hours (default), syncs only modified content
    - Full: Every 7 days (default, disabled), re-indexes entire PKM
  - Filters pages by built-in `updatedAt` field, blocks by custom `cyberorganism-updated-ms` property
  - Manages sync status checking and timestamp updates
  - Handles tree traversal for block counting and ID collection
  - Sends all PKM IDs to /sync/verify for deletion detection
- **api.js**: HTTP communication layer (exposed as `window.KnowledgeGraphAPI`)
  - `sendToBackend(data)`: Sends data to POST /data endpoint, returns boolean
  - `sendBatchToBackend(type, batch, graphName)`: Wrapper for batch operations, formats as `${type}_batch`
  - `log.error/warn/info/debug/trace(message, details, source)`: Sends logs to POST /log endpoint
  - `checkBackendAvailabilityWithRetry(maxRetries, delayMs)`: Health check with retries (used before sync)
  - Port discovery (tries 3000-3010), sync status queries
  - Full API documentation in the module header comments of api.js
- **data_processor.js**: Validates and transforms Logseq data before transmission
  - Processes blocks and pages into standardized format
  - Adds normalized_name (lowercase) to pages for consistent lookups
  - Extracts references (page refs, block refs, tags)

**Rust Backend Server**
- **main.rs**: HTTP server orchestration and application state management
  - Manages server lifecycle, AppState, and high-level control flow
  - Coordinates with extracted modules for specific functionality
  - Handles Logseq launching and process termination
- **config.rs**: Configuration management module
  - Loads configuration from `config.yaml` with fallback to defaults
  - Validates JavaScript plugin configuration matches Rust settings
  - Provides Config, BackendConfig, LogseqConfig, DevelopmentConfig, SyncConfig structs
  - Uses `CARGO_MANIFEST_DIR` to reliably locate api.js for validation
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
      - `last_incremental_sync`: Unix timestamp in milliseconds or null
      - `last_incremental_sync_iso`: ISO timestamp string or null
      - `hours_since_incremental`: Float hours since last incremental sync
      - `incremental_sync_needed`: Boolean (based on config interval)
      - `last_full_sync`: Unix timestamp in milliseconds or null
      - `last_full_sync_iso`: ISO timestamp string or null
      - `hours_since_full`: Float hours since last full sync
      - `true_full_sync_needed`: Boolean (based on config interval)
      - `force_incremental_sync`: Boolean (true if --force-incremental-sync flag was used)
      - `force_full_sync`: Boolean (true if --force-full-sync flag was used)
      - `sync_config`: Object with sync configuration (intervals and enable_full_sync)
      - `node_count`: Total nodes in graph
      - `edge_count`: Total edges in graph
  
  - `PATCH /sync` - Update sync timestamp
    - Called after successful sync completion
    - Accepts: JSON object with optional `sync_type` field ("incremental" or "full", defaults to "incremental")
    - Updates internal timestamp for the specified sync type
    - Returns: `ApiResponse` with success status
  
  - `POST /sync/verify` - Verify PKM IDs and archive deleted nodes
    - Called after full sync to detect deletions
    - Accepts: JSON object with:
      - `pages`: Array of all current page names in PKM
      - `blocks`: Array of all current block UUIDs in PKM
    - Archives nodes that no longer exist to `archived_nodes/` directory
    - Returns: `ApiResponse` with archived count and details
  
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
  - HashMap for O(1) PKM ID → NodeIndex lookups (uses normalized lowercase names for pages)
  - Separate sync timestamps: `last_incremental_sync` and `last_full_sync`
  - Sync status methods: `is_incremental_sync_needed()` and `is_true_full_sync_needed()`
  - Automatic saves: time-based (5 min) or operation-based (10 ops), disabled during batches
  - Graph persistence to `knowledge_graph.json` with full serialization
  - Node archival: Deleted nodes saved to `archived_nodes/archive_YYYYMMDD_HHMMSS.json`
  - Deletion detection via `verify_and_archive_missing_nodes()` after sync
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

### Incremental Sync (Every 2 Hours by default)
```
Check Last Incremental Sync → Query All Pages/Blocks → Filter by Modified Date → Process in Batches → Send PKM IDs for Deletion Detection → Update Backend → Update Incremental Sync Timestamp
```
- **Timestamp Filtering**: Pages use built-in `updatedAt` field; blocks use custom `cyberorganism-updated-ms` property
- **Efficient**: Only processes content modified since last incremental sync

### Full Database Sync (Every 7 Days by default, disabled)
```
Check Last Full Sync → Query All Pages/Blocks → Process ALL Content (No Filtering) → Send PKM IDs for Deletion Detection → Update Backend → Update Full Sync Timestamp
```
- **Complete Re-index**: Processes entire PKM without timestamp filtering
- **Use Cases**: Recovers from external file modifications, ensures data integrity
- **Deletion Detection**: After both sync types, sends all current PKM IDs to verify endpoint
- **Archival**: Deleted nodes are preserved in timestamped JSON files

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
- Backend server configuration (port, max port attempts)
- Sync intervals and configuration:
  - `incremental_interval_hours`: Hours between incremental syncs (default: 2)
  - `full_interval_hours`: Hours between full database syncs (default: 168/7 days)
  - `enable_full_sync`: Whether to perform full syncs (default: false)
- Logseq auto-launch settings
- Development duration for auto-shutdown
- Server always binds to localhost for security

## Testing

- **JavaScript Plugin**: `npm test` (in extensions/pkm_knowledge_graph/) - Jest test suite with comprehensive coverage:
  - `data_processor.test.js`: Tests for reference extraction and data validation
  - `sync.test.js`: Tests for sync status logic, tree traversal utilities
  - Browser environment mocking for Logseq plugin testing
- **Code Quality**: `npx eslint *.js` - ESLint configured for browser, Jest, and Node.js environments
- **Rust Backend**: `cargo test` (in extensions/pkm_knowledge_graph/backend/) - Unit tests for core modules (quiet by default)
- **Rust Core**: `cargo test` (in cyberorganism root) - Unit tests for AIChat core functionality (rarely needed)
- **Development**: `RUST_LOG=debug cargo run` (from backend directory) - Run backend server with default 3-second duration for testing
- **Force Incremental Sync**: `cargo run -- --force-incremental-sync` - Override sync status to force an incremental sync on next plugin connection
- **Force Full Sync**: `cargo run -- --force-full-sync` - Override sync status to force a full database sync on next plugin connection

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