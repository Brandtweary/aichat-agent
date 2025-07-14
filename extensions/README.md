# Cyberorganism Extensions

This directory contains the core components that power Cyberorganism's knowledge graph capabilities.

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

The `pkm_knowledge_graph` module enables Cyberorganism to transform your personal knowledge management system into a queryable knowledge graph. It consists of:

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
cd extensions/pkm_knowledge_graph
cp config.example.yaml config.yaml
# Edit config.yaml if needed (defaults usually work)
```

**3. Install the Logseq plugin:**
- Open Logseq
- Go to Settings → Advanced → Developer mode (enable it)
- Go to Settings → Plugins → Load unpacked plugin
- Select the `extensions/pkm_knowledge_graph` directory
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
- **Full sync**: Every 2 hours, performs a complete sync to catch any missed changes  
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

1. Copy `extensions/pkm_knowledge_graph/config.example.yaml` to `extensions/pkm_knowledge_graph/config.yaml`
2. Edit the settings as needed (see config.yaml for available options)

The server always binds to localhost for security. If the default port is unavailable, the server will automatically try the next available port.

Note: The `config.yaml` file is ignored by git to allow for local customization without affecting the repository.

#### Commands

**Backend Server (Rust)**
```bash
cd extensions/pkm_knowledge_graph/backend

# Build and run the backend server
RUST_LOG=info cargo run

# Run server for testing (exits after specified seconds)
RUST_LOG=info cargo run -- --duration 3

# Build only
cargo build

# Run tests
cargo test
```

**Frontend Plugin (JavaScript)**
```bash
cd extensions/pkm_knowledge_graph

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

When adding new features to Cyberorganism:

1. Try to keep changes to the original codebase minimal
2. Place new functionality in this extensions directory when possible
3. Document any changes made to the original AIChat codebase
