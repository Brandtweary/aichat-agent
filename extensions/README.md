# Cymbiont Extensions

This directory contains the core components that power Cymbiont's knowledge graph capabilities.

## Purpose

The extensions folder is designed to keep our changes contained and clearly separated from the original codebase, making it easier to:

1. Identify what has been modified from the original AIChat project
2. Maintain compatibility with upstream changes
3. Document and organize our custom features

## Structure

The extensions directory is organized as follows:

- `README.md` - This documentation file
- `pkm_knowledge_graph/` - Core knowledge graph integration with personal knowledge management tools
- `logseq_dummy_graph/` - Sample Logseq graph for testing and development

## Extensions

### PKM Knowledge Graph

The `pkm_knowledge_graph` module enables Cymbiont to transform your personal knowledge management system into a queryable knowledge graph. It consists of:

- A Logseq plugin that syncs block and page data in real-time
- A Rust backend server that maintains the knowledge graph structure
- Integration with petgraph for efficient graph operations and traversal

Currently supports Logseq, with Obsidian support planned based on user interest.

#### Prerequisites

- **Rust** (latest stable) - Install from [rustup.rs](https://rustup.rs/)
- **Node.js** (v16+) - For running Jest tests
- **Logseq** - Desktop app (plugin works with local graphs)

#### Installation & Setup

**1. Set up the backend server:**
```bash
cd extensions/pkm_knowledge_graph/backend
cargo build
```

**2. Configure the extension:**
```bash
cd extensions/pkm_knowledge_graph/backend
cp config.example.yaml config.yaml
# Edit config.yaml if needed (defaults usually work)
```

**3. Install the Logseq plugin:**
- Open Logseq
- Go to Settings → Advanced → Developer mode (enable it)
- Go to Settings → Plugins → Load unpacked plugin
- Select the `extensions/pkm_knowledge_graph/frontend` directory
- The plugin should appear in your plugins list

**4. Start the system:**
```bash
# Start the backend server
cd extensions/pkm_knowledge_graph/backend
RUST_LOG=info cargo run

# Open Logseq (click the app icon) and enable the plugin
# The plugin will automatically try to connect to the backend
```

**5. Verify it's working:**
- Check the backend terminal for "Backend server listening on 127.0.0.1:3000"
- In Logseq, check the plugin is enabled and shows "Connected" status
- Create a test page with some [[links]] and verify the backend logs show data processing

#### What You'll Get

Once set up, the system automatically:
- **Real-time sync**: Changes to pages/blocks sync immediately to the knowledge graph
- **Incremental sync**: Every 2 hours, syncs only modified content to catch any missed changes
- **Full sync**: Weekly re-indexing of entire PKM (optional, disabled by default)
- **Reference tracking**: Captures page links `[[Page Name]]`, block references `((block-id))`, tags `#tag`, and properties `key:: value`
- **Graph storage**: Maintains a queryable graph structure of your entire knowledge base

The knowledge graph runs in the background - you continue using Logseq normally while it builds a structured representation of your notes.

#### Troubleshooting

**Plugin won't load:**
- Ensure Developer mode is enabled in Logseq settings
- Verify the `package.json` file exists in the plugin directory

**Plugin shows "Disconnected":**
- Ensure the backend server is running first
- Try restarting both backend and Logseq

**No data being synced:**
- Check backend logs for "Received data from" messages
- Empty blocks are intentionally skipped (not an error)

#### Configuration

The extension uses its own configuration file separate from the main AIChat configuration:

1. Copy `extensions/pkm_knowledge_graph/backend/config.example.yaml` to `extensions/pkm_knowledge_graph/backend/config.yaml`
2. Edit the settings as needed (see config.yaml for available options)

**Key Configuration Options:**

**Backend Server:**
- `backend.port: 3000` - Server port (automatically tries alternatives if busy)
- `backend.max_port_attempts: 10` - Number of alternative ports to try

**Logseq Auto-Launch:**
- `logseq.auto_launch: true` - Automatically start Logseq when server starts
- `logseq.executable_path: /path/to/logseq` - Optional custom path to Logseq executable

**Development Settings:**
- `development.default_duration: 3` - Auto-exit timer for development (set to null for production)

**Sync Configuration:**
- `sync.incremental_interval_hours: 2` - Hours between incremental syncs (default: 2)
- `sync.full_interval_hours: 168` - Hours between full database syncs (default: 168/7 days)
- `sync.enable_full_sync: false` - Whether to perform full database syncs (default: false)

**Understanding the 3-Tiered Sync System:**

The PKM Knowledge Graph uses a 3-tiered sync system with multiple layers of redundancy:

1. **Real-time Sync** (Always on): Individual changes are synced immediately as you edit
2. **Incremental Sync** (Every 2 hours): Catches any blocks added while the plugin wasn't loaded and provides backup for real-time sync
3. **Full Database Sync** (Weekly, disabled by default): Re-indexes your entire PKM

**When to Enable Full Database Sync:**

The real-time and incremental sync layers handle virtually all normal use cases. Full database sync is only needed if you:

- **Modify Logseq files directly** without opening the app (e.g., using a text editor)
- **Use scripts or external tools** that modify your Logseq graph files
- **Sync your graph via cloud services** where files might change outside of Logseq

To enable full database sync:
```yaml
sync:
  enable_full_sync: true
  full_interval_hours: 168  # Weekly, adjust as needed
```

**Forcing Sync Operations:**

You can manually trigger syncs using CLI flags:
- `cargo run -- --force-incremental-sync` - Force an incremental sync on next plugin connection
- `cargo run -- --force-full-sync` - Force a full database sync on next plugin connection

**Logseq Auto-Launch Details:**

When `auto_launch: true` is enabled, the server will automatically:
1. **Find Logseq executable** by searching common installation locations:
   - **Linux**: Checks PATH (`which logseq`), then searches for AppImage files in `~/.local/share/applications/appimages/`, `~/Applications/`, `~/Downloads/`, etc.
   - **macOS**: Looks for `/Applications/Logseq.app/Contents/MacOS/Logseq`
   - **Windows**: Searches `%USERPROFILE%\AppData\Local\Logseq\Logseq.exe`
2. **Launch Logseq** after the server starts successfully
3. **Monitor plugin initialization** and wait for the plugin to connect
4. **Terminate Logseq gracefully** when the server shuts down

If auto-launch can't find your Logseq installation, specify the exact path:
```yaml
logseq:
  auto_launch: true
  executable_path: /custom/path/to/logseq
```

To disable auto-launch (useful if you prefer manual control):
```yaml
logseq:
  auto_launch: false
```

The server always binds to localhost for security. If the default port is unavailable, the server will automatically try the next available port.

Note: The `config.yaml` file is ignored by git to allow for local customization without affecting the repository.

#### Commands

**Backend Server (Rust)**
```bash
cd extensions/pkm_knowledge_graph/backend

# Build and run the backend server
RUST_LOG=info cargo run

# Run server for development (uses default 3-second duration from config)
RUST_LOG=info cargo run

# Override duration for specific testing needs
RUST_LOG=info cargo run -- --duration 10

# Build only
cargo build

# Run tests
cargo test
```

**Development Duration:**
- By default, development runs automatically exit after 3 seconds (configurable in `config.yaml`)
- This prevents servers from running indefinitely during development
- Use `--duration X` to override the default when needed
- Production builds warn if `default_duration` is not null

**Frontend Plugin (JavaScript)**
```bash
cd extensions/pkm_knowledge_graph/frontend

# Install dependencies
npm install

# Run tests (silent by default for minimal output)
npm test

# Run tests with verbose output for debugging
npm test -- --verbose
```

The backend server automatically:
- Terminates any previous instance on startup
- Finds an available port if the default is busy
- Writes server info to `pkm_knowledge_graph_server.json` for the JS plugin to discover
- Cleans up gracefully on shutdown

## Development Guidelines

When adding new features to Cymbiont:

1. Try to keep changes to the original codebase minimal
2. Place new functionality in this extensions directory when possible
3. Document any changes made to the original AIChat codebase
