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
use tokio::sync::oneshot;
use std::net::TcpListener;
use std::error::Error;
use std::fs;
use std::time::{Duration, Instant};
use regex::Regex;
use tracing::{info, warn, error, debug, trace, Level};
use clap::Parser;
use std::io::{BufRead, BufReader};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

// Import our modules
mod pkm_data;
mod graph_manager;
use pkm_data::{PKMBlockData, PKMPageData};
use graph_manager::GraphManager;

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
    graph_manager: Mutex<GraphManager>,
    logseq_child: Mutex<Option<std::process::Child>>,
    plugin_init_tx: Mutex<Option<oneshot::Sender<()>>>,
    sync_complete_tx: Mutex<Option<oneshot::Sender<()>>>,
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
    // #[serde(rename = "graphName")]
    // graph_name: String,
    #[serde(default)]
    type_: Option<String>,
    payload: String,
}

// Log message from frontend
#[derive(Deserialize, Debug)]
struct LogMessage {
    level: String,
    message: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    details: Option<serde_json::Value>,
}

// Configuration structure
#[derive(Debug, Deserialize)]
struct Config {
    backend: BackendConfig,
    #[serde(default)]
    logseq: LogseqConfig,
    #[serde(default)]
    development: DevelopmentConfig,
}

#[derive(Debug, Deserialize)]
struct BackendConfig {
    port: u16,
    max_port_attempts: u16,
}

#[derive(Debug, Deserialize)]
struct LogseqConfig {
    #[serde(default = "default_auto_launch")]
    auto_launch: bool,
    #[serde(default)]
    executable_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevelopmentConfig {
    #[serde(default)]
    default_duration: Option<u64>,
}

fn default_auto_launch() -> bool {
    false
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
            logseq: LogseqConfig {
                auto_launch: false,
                executable_path: None,
            },
            development: DevelopmentConfig {
                default_duration: None,
            },
        }
    }
}

impl Default for LogseqConfig {
    fn default() -> Self {
        LogseqConfig {
            auto_launch: false,
            executable_path: None,
        }
    }
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        DevelopmentConfig {
            default_duration: None,
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

// Validate that JavaScript plugin configuration matches Rust configuration
fn validate_js_plugin_config(config: &Config) -> Result<(), Box<dyn Error>> {
    // Path to the JavaScript API file
    let api_js_path = PathBuf::from("../api.js");
    
    if !api_js_path.exists() {
        warn!("JavaScript API file not found at ../api.js - skipping config validation");
        return Ok(());
    }
    
    let api_js_content = fs::read_to_string(&api_js_path)?;
    
    // Extract defaultPort and maxPortAttempts from JavaScript
    let default_port_regex = Regex::new(r"const\s+defaultPort\s*=\s*(\d+)")?;
    let max_attempts_regex = Regex::new(r"const\s+maxPortAttempts\s*=\s*(\d+)")?;
    
    let js_default_port = default_port_regex
        .captures(&api_js_content)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u16>().ok());
        
    let js_max_attempts = max_attempts_regex
        .captures(&api_js_content)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u16>().ok());
    
    // Compare with Rust configuration
    let rust_default_port = config.backend.port;
    let rust_max_attempts = config.backend.max_port_attempts;
    
    let mut config_errors = Vec::new();
    
    if let Some(js_port) = js_default_port {
        if js_port != rust_default_port {
            config_errors.push(format!(
                "Port mismatch: JavaScript defaultPort={}, Rust port={}",
                js_port, rust_default_port
            ));
        }
    } else {
        config_errors.push("Could not find defaultPort in JavaScript API file".to_string());
    }
    
    if let Some(js_attempts) = js_max_attempts {
        if js_attempts != rust_max_attempts {
            config_errors.push(format!(
                "Max attempts mismatch: JavaScript maxPortAttempts={}, Rust max_port_attempts={}",
                js_attempts, rust_max_attempts
            ));
        }
    } else {
        config_errors.push("Could not find maxPortAttempts in JavaScript API file".to_string());
    }
    
    if !config_errors.is_empty() {
        error!("JavaScript plugin configuration validation failed:");
        for err in &config_errors {
            error!("  {}", err);
        }
        error!("Please ensure api.js uses the same port configuration as config.yaml");
        error!("JavaScript: const defaultPort = {}; const maxPortAttempts = {};", 
               rust_default_port, rust_max_attempts);
        
        // Don't fail the server startup, just warn loudly
        warn!("Continuing startup despite configuration mismatch - plugin may not work correctly");
    } else {
        info!("JavaScript plugin configuration validated successfully");
    }
    
    Ok(())
}

// Constants
const SERVER_INFO_FILE: &str = "pkm_knowledge_graph_server.json";

// Platform-specific Logseq executable paths
fn find_logseq_executable() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let user_profile = std::env::var("USERPROFILE").ok()?;
        let path = PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("Logseq")
            .join("Logseq.exe");
        if path.exists() {
            return Some(path);
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/Applications/Logseq.app/Contents/MacOS/Logseq");
        if path.exists() {
            return Some(path);
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").ok()?;
        
        // 1. Check if logseq is in PATH (snap install)
        if let Ok(output) = Command::new("which").arg("logseq").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        
        // 2. Search for AppImage in common locations
        let common_paths = vec![
            PathBuf::from(&home).join(".local/share/applications/appimages"),
            PathBuf::from(&home).join(".local/share/applications"),
            PathBuf::from(&home).join("Applications"),
            PathBuf::from(&home).join("Downloads"),
            PathBuf::from(&home).join(".local/bin"),
            PathBuf::from("/opt"),
            PathBuf::from("/usr/local/bin"),
        ];
        
        for dir in common_paths {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy().to_lowercase();
                        if name_str.contains("logseq") && name_str.ends_with(".appimage") {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    
    None
}

// Launch Logseq process
fn launch_logseq(config: &LogseqConfig) -> Result<Option<std::process::Child>, Box<dyn Error>> {
    if !config.auto_launch {
        info!("Logseq auto-launch is disabled");
        return Ok(None);
    }
    
    let executable = if let Some(path) = &config.executable_path {
        PathBuf::from(path)
    } else if let Some(path) = find_logseq_executable() {
        path
    } else {
        error!("Could not find Logseq executable. Please specify executable_path in config.yaml");
        return Ok(None);
    };
    
    info!("Launching Logseq from: {:?}", executable);
    
    let mut child = Command::new(&executable)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch Logseq: {}", e))?;
    
    // Spawn threads to handle stdout and stderr
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Filter out xdg-mime warnings
                    if line.contains("xdg-mime:") && line.contains("application argument missing") {
                        trace!("Logseq stdout (xdg-mime warning): {}", line);
                    } else if line.contains("›") {
                        // Electron logs with › symbol
                        trace!("Logseq: {}", line);
                    } else {
                        trace!("Logseq stdout: {}", line);
                    }
                }
            }
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Filter out xdg-mime warnings and rsapi init
                    if line.contains("xdg-mime:") && line.contains("application argument missing") {
                        trace!("Logseq stderr (xdg-mime warning): {}", line);
                    } else if line.contains("(rsapi) init loggers") {
                        trace!("Logseq stderr (rsapi): {}", line);
                    } else if line.contains("Try 'xdg-mime --help'") {
                        trace!("Logseq stderr (xdg-mime help): {}", line);
                    } else {
                        // Log other stderr at debug level in case something important shows up
                        debug!("Logseq stderr: {}", line);
                    }
                }
            }
        });
    }
    
    Ok(Some(child))
}

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
        
        info!("Found server info file with PID: {pid}");
        
        // First check if the process actually exists
        #[cfg(target_family = "unix")]
        {
            // Check if process exists using kill -0 (doesn't actually kill)
            let check_result = Command::new("kill")
                .arg("-0")
                .arg(&pid)
                .output();
                
            match check_result {
                Ok(output) => {
                    if !output.status.success() {
                        info!("Process {pid} no longer exists, cleaning up stale PID file");
                        return false;
                    }
                },
                Err(e) => {
                    error!("Error checking process: {e}");
                    return false;
                }
            }
            
            // Process exists, try to terminate it
            info!("Process {pid} is running, attempting to terminate");
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
            // Check if process exists using tasklist
            let check_result = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid)])
                .output();
                
            match check_result {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if !output_str.contains(&pid) {
                        info!("Process {pid} no longer exists, cleaning up stale PID file");
                        return false;
                    }
                },
                Err(e) => {
                    error!("Error checking process: {}", e);
                    return false;
                }
            }
            
            // Process exists, try to terminate it
            info!("Process {pid} is running, attempting to terminate");
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

// Cleanup function to handle graceful shutdown
fn cleanup_and_exit(app_state: Option<Arc<AppState>>, start_time: Instant) {
    let total_runtime = start_time.elapsed();
    info!("Cleaning up... (total runtime: {:.2}s)", total_runtime.as_secs_f64());
    
    // Terminate Logseq if it was launched by us
    if let Some(state) = app_state {
        if let Ok(mut child_guard) = state.logseq_child.lock() {
            if let Some(mut child) = child_guard.take() {
                match child.kill() {
                    Ok(_) => info!("Logseq terminated successfully"),
                    Err(e) => error!("Error terminating Logseq: {}", e),
                }
            }
        }
    }
    
    if let Err(e) = fs::remove_file(SERVER_INFO_FILE) {
        error!("Error removing server info file: {e}");
    }
}

// Root endpoint
async fn root() -> &'static str {
    "PKM Knowledge Graph Backend Server"
}

// Endpoint to get sync status
async fn get_sync_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let status = state.graph_manager.lock().unwrap().get_sync_status();
    Json(status)
}

// Endpoint to update sync timestamp after a full sync
async fn update_sync_timestamp(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse> {
    let mut graph_manager = state.graph_manager.lock().unwrap();
    
    match graph_manager.update_full_sync_timestamp() {
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

// Endpoint to receive log messages from the frontend
async fn receive_log(
    State(_state): State<Arc<AppState>>,
    Json(log): Json<LogMessage>,
) -> Json<ApiResponse> {
    let source = log.source.as_deref().unwrap_or("JS Plugin");
    
    // Convert JS log level to Rust tracing level and log appropriately
    match log.level.to_lowercase().as_str() {
        "error" => {
            if let Some(details) = &log.details {
                error!("[{}] {}: {:?}", source, log.message, details);
            } else {
                error!("[{}] {}", source, log.message);
            }
        },
        "warn" => {
            if let Some(details) = &log.details {
                warn!("[{}] {}: {:?}", source, log.message, details);
            } else {
                warn!("[{}] {}", source, log.message);
            }
        },
        "info" => {
            if let Some(details) = &log.details {
                info!("[{}] {}: {:?}", source, log.message, details);
            } else {
                info!("[{}] {}", source, log.message);
            }
        },
        "debug" => {
            if let Some(details) = &log.details {
                debug!("[{}] {}: {:?}", source, log.message, details);
            } else {
                debug!("[{}] {}", source, log.message);
            }
        },
        "trace" => {
            if let Some(details) = &log.details {
                trace!("[{}] {}: {:?}", source, log.message, details);
            } else {
                trace!("[{}] {}", source, log.message);
            }
        },
        _ => {
            // Default to info for unknown levels
            if let Some(details) = &log.details {
                info!("[{}] {}: {:?}", source, log.message, details);
            } else {
                info!("[{}] {}", source, log.message);
            }
        }
    }
    
    Json(ApiResponse {
        success: true,
        message: "Log received".to_string(),
    })
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
    let mut graph_manager = state.graph_manager.lock().unwrap();
    
    match graph_manager.create_or_update_node_from_pkm_block(&block_data) {
        Ok(node_idx) => {
            debug!("Block processed successfully: {:?}", node_idx);
            // Note: GraphManager already saves periodically
            drop(graph_manager);
            Ok("Block processed successfully".to_string())
        },
        Err(e) => {
            drop(graph_manager);
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
    let mut graph_manager = state.graph_manager.lock().unwrap();
    
    match graph_manager.create_or_update_node_from_pkm_page(&page_data) {
        Ok(node_idx) => {
            debug!("Page processed successfully: {:?}", node_idx);
            // Note: GraphManager already saves periodically
            drop(graph_manager);
            Ok("Page processed successfully".to_string())
        },
        Err(e) => {
            drop(graph_manager);
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
    
    // Get a single lock on the graph for the entire batch
    let mut graph_manager = state.graph_manager.lock().unwrap();
    
    // Disable auto-save during batch processing to avoid interleaved saves
    graph_manager.disable_auto_save();
    
    for block_data in blocks {
        // Validate and process each block
        if block_data.validate().is_ok() {
            match graph_manager.create_or_update_node_from_pkm_block(&block_data) {
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
    
    // Re-enable auto-save and force save after batch
    graph_manager.enable_auto_save();
    if success_count > 0 {
        if let Err(e) = graph_manager.save_graph() {
            error!("Error saving graph after batch processing: {e:?}");
        }
    }
    
    // Release the lock
    drop(graph_manager);
    
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
    
    // Get a single lock on the graph for the entire batch
    let mut graph_manager = state.graph_manager.lock().unwrap();
    
    // Disable auto-save during batch processing to avoid interleaved saves
    graph_manager.disable_auto_save();
    
    for page_data in pages {
        // Validate and process each page
        if page_data.validate().is_ok() {
            match graph_manager.create_or_update_node_from_pkm_page(&page_data) {
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
    
    // Re-enable auto-save and force save after batch
    graph_manager.enable_auto_save();
    if success_count > 0 {
        if let Err(e) = graph_manager.save_graph() {
            error!("Error saving graph after batch processing: {e:?}");
        }
    }
    
    // Release the lock
    drop(graph_manager);
    
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
        Some("plugin_initialized") => {
            // Signal plugin initialization if we have a waiting channel
            if let Ok(mut tx_guard) = state.plugin_init_tx.lock() {
                if let Some(tx) = tx_guard.take() {
                    let _ = tx.send(());
                }
            }
            
            Json(ApiResponse {
                success: true,
                message: "Plugin initialization acknowledged".to_string(),
            })
        },
        Some("sync_complete") => {
            // Signal sync completion if we have a waiting channel
            if let Ok(mut tx_guard) = state.sync_complete_tx.lock() {
                if let Some(tx) = tx_guard.take() {
                    let _ = tx.send(());
                    debug!("Sync completion signal received");
                }
            }
            
            Json(ApiResponse {
                success: true,
                message: "Sync completion acknowledged".to_string(),
            })
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
    // Start runtime timer
    let start_time = Instant::now();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize tracing subscriber with custom formatting
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .event_format(ConditionalLocationFormatter)
        .init();
    
    // Load configuration
    let config = load_config();
    
    // Validate JavaScript plugin configuration
    if let Err(e) = validate_js_plugin_config(&config) {
        warn!("Failed to validate JavaScript plugin configuration: {}", e);
    }
    
    // Terminate any previous instance
    if fs::metadata(SERVER_INFO_FILE).is_ok() {
        terminate_previous_instance();
        let _ = fs::remove_file(SERVER_INFO_FILE);
    }
    
    // Initialize the graph manager
    let data_dir = PathBuf::from("data");
    let graph_manager = GraphManager::new(data_dir)
        .map_err(|e| Box::<dyn Error>::from(format!("Graph manager error: {e:?}")))?;
    
    // Create shared application state
    let app_state = Arc::new(AppState {
        graph_manager: Mutex::new(graph_manager),
        logseq_child: Mutex::new(None),
        plugin_init_tx: Mutex::new(None),
        sync_complete_tx: Mutex::new(None),
    });
    
    // Set up exit handler
    let app_state_clone = app_state.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        cleanup_and_exit(Some(app_state_clone.clone()), start_time);
        exit(0);
    }).expect("Error setting Ctrl-C handler");
    
    // Define the application routes
    let app = Router::new()
        .route("/", get(root))
        .route("/data", post(receive_data))
        .route("/sync/status", get(get_sync_status))
        .route("/sync", patch(update_sync_timestamp))
        .route("/log", post(receive_log))
        .with_state(app_state.clone());

    // Find available port
    let port = find_available_port(&config.backend)?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    // Write server info file for JS plugin
    write_server_info("127.0.0.1", port)?;
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| Box::<dyn Error>::from(format!("Listener error: {e}")))?;
    
    info!("Backend server listening on {}", addr);
    
    // Launch Logseq after server is ready
    let logseq_child = launch_logseq(&config.logseq)?;
    
    // Create channel for plugin initialization if we launched Logseq
    let plugin_init_rx = if let Some(child) = logseq_child {
        // Store child process
        if let Ok(mut child_guard) = app_state.logseq_child.lock() {
            *child_guard = Some(child);
        }
        
        // Create initialization channel
        let (tx, rx) = oneshot::channel::<()>();
        if let Ok(mut tx_guard) = app_state.plugin_init_tx.lock() {
            *tx_guard = Some(tx);
        }
        Some(rx)
    } else {
        None
    };
    
    // Determine duration: explicit CLI arg takes precedence over config default
    let duration_secs = args.duration.or(config.development.default_duration);
    
    // Warn if using default duration in release build
    #[cfg(not(debug_assertions))]
    if let Some(duration) = config.development.default_duration {
        warn!("Development default_duration ({} seconds) detected in release build - this should be null in production!", duration);
    }
    
    // Run server with appropriate configuration
    if let Some(duration) = duration_secs {
        if let Some(rx) = plugin_init_rx {
            // Wait for plugin initialization before starting timer
            run_with_duration(listener, app, app_state.clone(), rx, duration).await?;
        } else {
            // No Logseq, start timer immediately
            info!("Server will run for {} seconds", duration);
            run_server_with_timeout(listener, app, duration).await?;
        }
    } else {
        // Run indefinitely
        if let Some(rx) = plugin_init_rx {
            // Monitor plugin initialization in background
            tokio::spawn(async move {
                match rx.await {
                    Ok(_) => info!("Plugin initialization confirmed"),
                    Err(_) => debug!("Plugin initialization channel closed"),
                }
            });
        }
        
        axum::serve(listener, app).await
            .map_err(|e| Box::<dyn Error>::from(format!("Server error: {e}")))?;
    }
    
    // Clean up before exiting
    cleanup_and_exit(Some(app_state), start_time);
    
    Ok(())
}

// Helper function to find an available port
fn find_available_port(config: &BackendConfig) -> Result<u16, Box<dyn Error>> {
    let port = config.port;
    
    if is_port_available(port) {
        return Ok(port);
    }
    
    warn!("Configured port {} is not available.", port);
    
    for p in (port + 1)..=(port + config.max_port_attempts) {
        if is_port_available(p) {
            info!("Using alternative port: {}", p);
            return Ok(p);
        }
    }
    
    Err(Box::<dyn Error>::from("Could not find an available port"))
}

// Run server with duration timer starting after plugin initialization
async fn run_with_duration(
    listener: tokio::net::TcpListener,
    app: Router,
    app_state: Arc<AppState>,
    plugin_initialized: oneshot::Receiver<()>,
    duration_secs: u64,
) -> Result<(), Box<dyn Error>> {
    // Create graceful shutdown signal
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    
    // Create sync completion channel BEFORE plugin starts
    let (sync_tx, sync_rx) = oneshot::channel::<()>();
    if let Ok(mut tx_guard) = app_state.sync_complete_tx.lock() {
        *tx_guard = Some(sync_tx);
    }
    
    // Serve with graceful shutdown capability
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        });
    
    // Wait for plugin initialization, then start duration timer
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = async {
            // Wait for plugin to initialize
            match plugin_initialized.await {
                Ok(_) => {
                    info!("Server will run for {} seconds after plugin initialization", duration_secs);
                    tokio::time::sleep(Duration::from_secs(duration_secs)).await;
                    info!("Duration limit reached, checking for active sync...");
                    
                    // Wait for sync completion with timeout
                    tokio::select! {
                        _ = sync_rx => {
                            info!("Sync completion received, shutting down gracefully");
                        }
                        _ = tokio::time::sleep(Duration::from_secs(10)) => {
                            info!("Timeout waiting for sync completion, shutting down anyway");
                        }
                    }
                },
                Err(_) => {
                    // If plugin init fails, still run with timer
                    info!("Plugin initialization failed, running with {} second timer anyway", duration_secs);
                    tokio::time::sleep(Duration::from_secs(duration_secs)).await;
                    info!("Duration limit reached, shutting down gracefully");
                }
            }
            
            // Signal server to start graceful shutdown
            let _ = shutdown_tx.send(());
        } => {}
    }
    
    Ok(())
}

// Simple timeout for when Logseq is not launched
async fn run_server_with_timeout(
    listener: tokio::net::TcpListener,
    app: Router,
    duration_secs: u64,
) -> Result<(), Box<dyn Error>> {
    let server = axum::serve(listener, app);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = tokio::time::sleep(Duration::from_secs(duration_secs)) => {
            info!("Duration limit reached, shutting down gracefully");
        }
    }
    
    Ok(())
}
