/**
 * @module main
 * @description Backend server for the PKM Knowledge Graph Plugin
 * 
 * This module implements an HTTP server that receives data from the PKM frontend plugin,
 * processes it, and stores it in a persistent knowledge graph datastore. It serves as the
 * backend component of the PKM Knowledge Graph extension.
 * 
 * Key responsibilities:
 * - Setting up and running the HTTP server using the Axum framework
 * - Defining API endpoints for data reception and sync status management
 * - Processing incoming PKM data (blocks, pages, and their references)
 * - Managing the PKMDatastore for persistent storage (to be replaced with petgraph)
 * - Handling server lifecycle (startup, shutdown, port management)
 * - Configuration loading and management
 * - Process management (PID tracking, termination of previous instances)
 * 
 * API endpoints:
 * - GET  /          : Root endpoint that confirms the server is running
 * - POST /data      : Main endpoint for receiving data from the PKM plugin
 * - GET  /sync/status: Returns the current sync status (timestamp, node count, etc.)
 * - PATCH /sync     : Updates the sync timestamp after a full sync
 * 
 * The server handles several types of data:
 * - Individual blocks and pages
 * - Batches of blocks and pages for efficient processing
 * - Diagnostic information
 * 
 * TODO: Add Logseq process management
 * - Launch Logseq automatically when server starts
 * - Ensure plugin is loaded and enabled
 * - Handle Logseq shutdown when server stops
 * - This would eliminate most setup/troubleshooting issues
 * 
 * Dependencies:
 * - pkm_datastore: Module for persistent storage of the knowledge graph
 * - axum: Web framework for handling HTTP requests
 * - tokio: Async runtime
 * - serde: Serialization/deserialization of JSON data
 * 
 * @requires pkm_datastore
 * @requires axum
 * @requires tokio
 * @requires serde
 */

use axum::{
    extract::State,
    routing::{get, post, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::process::{Command, exit};
use std::net::TcpListener;
use std::error::Error;
use std::fs;
use std::time::Duration;
use tracing::{info, warn, error, debug, Level};
use clap::Parser;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

// Import our datastore module
mod pkm_datastore;
use pkm_datastore::{PKMDatastore, PKMBlockData, PKMPageData};

/// Custom formatter that conditionally shows file:line only for ERROR and WARN levels
struct ConditionalLocationFormatter;

impl<S, N> FormatEvent<S, N> for ConditionalLocationFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();
        let level = metadata.level();
        
        // Format level
        write!(&mut writer, "{}", level)?;
        
        // Only show module target and file:line for ERROR and WARN levels
        if matches!(level, &Level::ERROR | &Level::WARN) {
            write!(&mut writer, " {}", metadata.target())?;
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                write!(&mut writer, " {}:{}", file, line)?;
            }
        }
        
        write!(&mut writer, ": ")?;
        
        // Format all the spans in the event's span context
        if let Some(scope) = ctx.event_scope() {
            let mut first = true;
            for span in scope.from_root() {
                if !first {
                    write!(&mut writer, ":")?;
                }
                first = false;
                write!(writer, "{}", span.name())?;
                
                let ext = span.extensions();
                if let Some(fields) = ext.get::<tracing_subscriber::fmt::FormattedFields<N>>() {
                    if !fields.is_empty() {
                        write!(writer, "{{{}}}", fields)?;
                    }
                }
            }
            write!(writer, " ")?;
        }
        
        // Write the event fields
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        
        writeln!(writer)
    }
}

// CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run server for a specific duration in seconds (for testing)
    #[arg(long)]
    duration: Option<u64>,
}

// Application state that will be shared between handlers
struct AppState {
    datastore: Mutex<PKMDatastore>,
}

// Basic response for API calls
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

// Incoming data from the PKM plugin
#[derive(Deserialize, Debug)]
struct PKMData {
    source: String,
    timestamp: String,
    // #[serde(rename = "graphName")]
    // graph_name: String,
    #[serde(default)]
    type_: Option<String>,
    payload: String,
}

// Configuration structure
#[derive(Debug, Deserialize)]
struct Config {
    backend: BackendConfig,
}

#[derive(Debug, Deserialize)]
struct BackendConfig {
    port: u16,
    max_port_attempts: u16,
}

// Server info written to file for JS plugin
#[derive(Serialize, Deserialize)]
struct ServerInfo {
    pid: u32,
    host: String,
    port: u16,
}

// Default configuration
impl Default for Config {
    fn default() -> Self {
        Config {
            backend: BackendConfig {
                port: 3000,
                max_port_attempts: 10,
            },
        }
    }
}

// Load configuration from file
fn load_config() -> Config {
    // Determine the executable directory
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    
    // Try to find config.yaml in parent directories
    let mut config_path = PathBuf::from(exe_dir);
    let mut found = false;
    
    // First check if config exists in the current directory
    if config_path.join("config.yaml").exists() {
        found = true;
    } else {
        // Try up to 3 parent directories
        for _ in 0..3 {
            config_path = match config_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => break,
            };
            
            if config_path.join("config.yaml").exists() {
                found = true;
                break;
            }
        }
    }
    
    // If config.yaml was found, try to load it
    if found {
        let config_file = config_path.join("config.yaml");
        match fs::read_to_string(&config_file) {
            Ok(contents) => {
                match serde_yaml::from_str(&contents) {
                    Ok(config) => {
                        debug!("Loaded configuration from {:?}", config_file);
                        return config;
                    },
                    Err(e) => {
                        error!("Error parsing config.yaml: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Error reading config.yaml: {}", e);
            }
        }
    }
    
    // If we get here, use default configuration
    debug!("Using default configuration");
    Config::default()
}

// Constants
const SERVER_INFO_FILE: &str = "pkm_knowledge_graph_server.json";

// Check if a port is available
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

// Try to terminate a previous instance of our server
fn terminate_previous_instance() -> bool {
    // Check if server info file exists
    if let Ok(info_str) = fs::read_to_string(SERVER_INFO_FILE) {
        if let Ok(info) = serde_json::from_str::<ServerInfo>(&info_str) {
            let pid = info.pid.to_string();
        
        info!("Found previous instance with PID: {pid}");
        
        // Try to terminate the process
        #[cfg(target_family = "unix")]
        {
            let kill_result = Command::new("kill")
                .arg("-15") // SIGTERM for graceful shutdown
                .arg(&pid)
                .output();
                
            match kill_result {
                Ok(output) => {
                    if output.status.success() {
                        info!("Successfully terminated previous instance");
                        // Give the process time to shut down
                        std::thread::sleep(Duration::from_millis(500));
                        return true;
                    }
                    error!("Failed to terminate process: {}", 
                        String::from_utf8_lossy(&output.stderr));
                },
                Err(e) => {
                    error!("Error terminating process: {e}");
                }
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            let kill_result = Command::new("taskkill")
                .args(&["/PID", &pid, "/F"])
                .output();
                
            match kill_result {
                Ok(output) => {
                    if output.status.success() {
                        info!("Successfully terminated previous instance");
                        // Give the process time to shut down
                        std::thread::sleep(Duration::from_millis(500));
                        return true;
                    } else {
                        error!("Failed to terminate process: {}", 
                            String::from_utf8_lossy(&output.stderr));
                    }
                },
                Err(e) => {
                    error!("Error terminating process: {}", e);
                }
            }
        }
        }
    }
    
    false
}


// Write server info including actual port
fn write_server_info(host: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let info = ServerInfo {
        pid: std::process::id(),
        host: host.to_string(),
        port,
    };
    let json = serde_json::to_string_pretty(&info)?;
    fs::write(SERVER_INFO_FILE, json)?;
    Ok(())
}

// Clean up server info file on exit
fn setup_exit_handler() {
    ctrlc::set_handler(move || {
        info!("Received shutdown signal, cleaning up...");
        if let Err(e) = fs::remove_file(SERVER_INFO_FILE) {
            error!("Error removing server info file: {e}");
        }
        exit(0);
    }).expect("Error setting Ctrl-C handler");
}

// Root endpoint
async fn root() -> &'static str {
    "PKM Knowledge Graph Backend Server"
}

// Endpoint to get sync status
async fn get_sync_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let status = state.datastore.lock().unwrap().get_sync_status();
    Json(status)
}

// Endpoint to update sync timestamp after a full sync
async fn update_sync_timestamp(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse> {
    let mut datastore = state.datastore.lock().unwrap();
    
    match datastore.update_full_sync_timestamp() {
        Ok(()) => {
            debug!("Sync timestamp updated successfully");
            Json(ApiResponse {
                success: true,
                message: "Sync timestamp updated successfully".to_string(),
            })
        },
        Err(e) => {
            error!("Error updating sync timestamp: {e:?}");
            Json(ApiResponse {
                success: false,
                message: format!("Error updating sync timestamp: {e:?}"),
            })
        }
    }
}

// Helper functions for data parsing
fn parse_block_data(payload: &str) -> Result<PKMBlockData, serde_json::Error> {
    serde_json::from_str::<PKMBlockData>(payload)
}

fn parse_page_data(payload: &str) -> Result<PKMPageData, serde_json::Error> {
    serde_json::from_str::<PKMPageData>(payload)
}

// Helper function for handling block data
fn handle_block_data(state: Arc<AppState>, payload: &str) -> Result<String, String> {
    // Parse the payload as a PKMBlockData
    let block_data = parse_block_data(payload)
        .map_err(|e| format!("Could not parse block data: {e}"))?;
    
    // Validate the block data
    if block_data.id.is_empty() {
        return Err("Block ID is empty".to_string());
    }
    
    // Process the block data
    let mut datastore = state.datastore.lock().unwrap();
    
    match datastore.create_or_update_node_from_pkm_block(&block_data) {
        Ok(node_id) => {
            debug!("Block processed successfully: {node_id}");
            let result = datastore.save_state()
                .map_err(|e| format!("Error saving state: {e:?}"));
            drop(datastore);
            result?;
            Ok("Block processed successfully".to_string())
        },
        Err(e) => {
            drop(datastore);
            Err(format!("Error processing block: {e:?}"))
        }
    }
}

// Helper function for handling page data
fn handle_page_data(state: Arc<AppState>, payload: &str) -> Result<String, String> {
    // Parse the payload as a PKMPageData
    let page_data = parse_page_data(payload)
        .map_err(|e| format!("Could not parse page data: {e}"))?;
    
    // Validate the page data
    if page_data.name.is_empty() {
        return Err("Page name is empty".to_string());
    }
    
    // Process the page data
    let mut datastore = state.datastore.lock().unwrap();
    
    match datastore.create_or_update_node_from_pkm_page(&page_data) {
        Ok(node_id) => {
            debug!("Page processed successfully: {node_id}");
            let result = datastore.save_state()
                .map_err(|e| format!("Error saving state: {e:?}"));
            drop(datastore);
            result?;
            Ok("Page processed successfully".to_string())
        },
        Err(e) => {
            drop(datastore);
            Err(format!("Error processing page: {e:?}"))
        }
    }
}

// Helper function for handling default data
fn handle_default_data(source: &str) -> Result<String, String> {
    // For DB changes, just acknowledge receipt without verbose logging
    if source == "PKM DB Change" {
        // Minimal logging for DB changes
        debug!("Processing DB change event");
    } else {
        debug!("Processing data with unspecified type");
    }
    
    Ok("Data received".to_string())
}

// Helper function for handling batch block data
fn handle_batch_blocks(state: Arc<AppState>, payload: &str) -> Result<String, String> {
    // Parse the payload as an array of PKMBlockData
    let blocks: Vec<PKMBlockData> = serde_json::from_str(payload)
        .map_err(|e| format!("Could not parse batch blocks: {e}"))?;
    
    debug!("Processing batch of {} blocks", blocks.len());
    
    let mut success_count = 0;
    let mut error_count = 0;
    let total_blocks = blocks.len();
    
    // Get a single lock on the datastore for the entire batch
    let mut datastore = state.datastore.lock().unwrap();
    
    for block_data in blocks {
        // Validate and process each block
        if block_data.validate().is_ok() {
            match datastore.create_or_update_node_from_pkm_block(&block_data) {
                Ok(_) => {
                    success_count += 1;
                },
                Err(_) => {
                    error_count += 1;
                }
            }
        } else {
            error_count += 1;
        }
    }
    
    // Save state once after processing the entire batch
    if success_count > 0 {
        if let Err(e) = datastore.save_state() {
            error!("Error saving state after batch processing: {e:?}");
        }
    }
    
    // Release the lock
    drop(datastore);
    
    // Report results
    if error_count == 0 {
        Ok(format!("Successfully processed all {total_blocks} blocks"))
    } else if success_count > 0 {
        Ok(format!("Processed {success_count}/{total_blocks} blocks successfully, {error_count} errors"))
    } else {
        Err(format!("Failed to process any blocks, {error_count} errors"))
    }
}

// Helper function for handling batch page data
fn handle_batch_pages(state: Arc<AppState>, payload: &str) -> Result<String, String> {
    // Parse the payload as an array of PKMPageData
    let pages: Vec<PKMPageData> = serde_json::from_str(payload)
        .map_err(|e| format!("Could not parse batch pages: {e}"))?;
    
    debug!("Processing batch of {} pages", pages.len());
    
    let mut success_count = 0;
    let mut error_count = 0;
    let total_pages = pages.len();
    
    // Get a single lock on the datastore for the entire batch
    let mut datastore = state.datastore.lock().unwrap();
    
    for page_data in pages {
        // Validate and process each page
        if page_data.validate().is_ok() {
            match datastore.create_or_update_node_from_pkm_page(&page_data) {
                Ok(_) => {
                    success_count += 1;
                },
                Err(_) => {
                    error_count += 1;
                }
            }
        } else {
            error_count += 1;
        }
    }
    
    // Save state once after processing the entire batch
    if success_count > 0 {
        if let Err(e) = datastore.save_state() {
            error!("Error saving state after batch processing: {e:?}");
        }
    }
    
    // Release the lock
    drop(datastore);
    
    // Report results
    if error_count == 0 {
        Ok(format!("Successfully processed all {total_pages} pages"))
    } else if success_count > 0 {
        Ok(format!("Processed {success_count}/{total_pages} pages successfully, {error_count} errors"))
    } else {
        Err(format!("Failed to process any pages, {error_count} errors"))
    }
}

// Endpoint to receive data from the PKM plugin
async fn receive_data(
    State(state): State<Arc<AppState>>,
    Json(data): Json<PKMData>,
) -> Json<ApiResponse> {
    // Log the source of the data
    debug!("Received data from: {} at {}", data.source, data.timestamp);
    
    // Process based on the type of data
    match data.type_.as_deref() {
        Some("block") => {
            match handle_block_data(state, &data.payload) {
                Ok(message) => {
                    Json(ApiResponse {
                        success: true,
                        message,
                    })
                },
                Err(message) => {
                    Json(ApiResponse {
                        success: false,
                        message,
                    })
                }
            }
        },
        Some("block_batch") | Some("blocks") => {
            match handle_batch_blocks(state, &data.payload) {
                Ok(message) => {
                    Json(ApiResponse {
                        success: true,
                        message,
                    })
                },
                Err(message) => {
                    Json(ApiResponse {
                        success: false,
                        message,
                    })
                }
            }
        },
        Some("page") => {
            match handle_page_data(state, &data.payload) {
                Ok(message) => {
                    Json(ApiResponse {
                        success: true,
                        message,
                    })
                },
                Err(message) => {
                    Json(ApiResponse {
                        success: false,
                        message,
                    })
                }
            }
        },
        Some("page_batch") | Some("pages") => {
            match handle_batch_pages(state, &data.payload) {
                Ok(message) => {
                    Json(ApiResponse {
                        success: true,
                        message,
                    })
                },
                Err(message) => {
                    Json(ApiResponse {
                        success: false,
                        message,
                    })
                }
            }
        },
        // For DB change events and other unspecified types
        _ => {
            match handle_default_data(&data.source) {
                Ok(message) => {
                    Json(ApiResponse {
                        success: true,
                        message,
                    })
                },
                Err(message) => {
                    Json(ApiResponse {
                        success: false,
                        message,
                    })
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize tracing subscriber with custom formatting
    // Optimized for LLM readability: no colors, no timestamps, no thread info, clean plain text
    // File:line only shown for ERROR and WARN levels to reduce verbosity
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .event_format(ConditionalLocationFormatter)
        .init();
    
    // Set up exit handler to clean up PID file
    setup_exit_handler();
    
    // Load configuration
    let config = load_config();
    
    // Check for previous instance and terminate it
    if fs::metadata(SERVER_INFO_FILE).is_ok() {
        terminate_previous_instance();
        // Remove the server info file in case the process doesn't exist anymore
        let _ = fs::remove_file(SERVER_INFO_FILE);
    }
    
    // Initialize the datastore
    let data_dir = PathBuf::from("data");
    let datastore = PKMDatastore::new(data_dir)
        .map_err(|e| Box::<dyn Error>::from(format!("Datastore error: {e:?}")))?;
    
    // Create shared application state
    let app_state = Arc::new(AppState {
        datastore: Mutex::new(datastore),
    });
    
    // Define the application routes
    let app = Router::new()
        .route("/", get(root))
        .route("/data", post(receive_data))
        .route("/sync/status", get(get_sync_status))
        .route("/sync", patch(update_sync_timestamp))
        .with_state(app_state);

    // Try to use the configured port
    let mut port = config.backend.port;
    
    // If configured port is not available, find another one
    if !is_port_available(port) {
        warn!("Configured port {port} is not available.");
        
        // Try a few alternative ports
        for p in (port + 1)..=(port + config.backend.max_port_attempts) {
            if is_port_available(p) {
                port = p;
                info!("Using alternative port: {port}");
                break;
            }
        }
        
        if port == config.backend.port {
            return Err(Box::<dyn Error>::from("Could not find an available port"));
        }
    }
    
    // Always bind to localhost for security - ignore config host setting
    let host_addr = [127, 0, 0, 1]; // localhost only
    
    let addr = SocketAddr::from((host_addr, port));
    info!("Backend server listening on {addr}");
    
    // Write server info file for JS plugin
    write_server_info("127.0.0.1", port)?;

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| Box::<dyn Error>::from(format!("Listener error: {e}")))?;
    
    // Check if we should run with a time limit
    if let Some(duration_secs) = args.duration {
        info!("Server will run for {} seconds", duration_secs);
        
        // Create a handle to the server
        let server = axum::serve(listener, app);
        
        // Run server with timeout
        tokio::select! {
            result = server => {
                if let Err(e) = result {
                    error!("Server error: {e}");
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(duration_secs)) => {
                info!("Duration limit reached, shutting down gracefully");
            }
        }
    } else {
        // Run server indefinitely
        axum::serve(listener, app).await
            .map_err(|e| Box::<dyn Error>::from(format!("Server error: {e}")))?;
    }
    
    // Clean up server info file before exiting
    if let Err(e) = fs::remove_file(SERVER_INFO_FILE) {
        error!("Error removing server info file: {e}");
    }
    
    Ok(())
}
