/**
 * @module api
 * @description Communication layer for the Logseq Knowledge Graph Plugin
 * 
 * CRITICAL WARNING FOR LLM ASSISTANTS:
 * =====================================
 * This is a BROWSER-BASED module. DO NOT add Node.js features.
 * This file exposes window.KnowledgeGraphAPI - do not change this pattern.
 * Breaking changes here will cause silent failures in Logseq.
 * 
 * This module provides a comprehensive API for all communication between the Logseq frontend
 * and the Rust backend server. It handles constructing API endpoints, sending data, checking
 * server availability, and managing sync operations.
 * 
 * The module exposes its functionality through the global `window.KnowledgeGraphAPI` object,
 * making these functions available to other parts of the plugin, particularly index.js.
 * 
 * Key responsibilities:
 * - Constructing backend URLs for various endpoints
 * - Sending data (blocks, pages, diagnostics) to the backend
 * - Checking backend server availability
 * - Managing sync status and operations
 * - Handling batch operations for efficient data transfer
 * - Error handling and reporting for network operations
 * 
 * Public interfaces:
 * - getBackendUrl(endpoint): Constructs a complete backend URL for a given endpoint
 * - sendToBackend(data): Sends data to the backend's /data endpoint
 * - sendDiagnosticInfo(message, details): Sends diagnostic information to the backend
 * - checkBackendAvailability(): Verifies if the backend server is running
 * - checkIfFullSyncNeeded(): Determines if a full database sync is required
 * - updateSyncTimestamp(): Updates the last sync timestamp on the backend
 * - sendBatchToBackend(type, batch, graphName): Sends a batch of blocks or pages
 * 
 * Dependencies:
 * - config.js: Contains backend configuration (host, port)
 * - Logseq API: For displaying messages and getting graph information
 * 
 * @requires config
 */

// Create a global API object to hold all the functions
console.log('Loading KnowledgeGraphAPI module...');
window.KnowledgeGraphAPI = {};
console.log('KnowledgeGraphAPI module loaded successfully.');

// Cache for server info
let serverInfoCache = null;
let serverInfoLastChecked = 0;
const SERVER_INFO_CACHE_MS = 5000; // Cache for 5 seconds

/**
 * Read server info from the JSON file written by the backend
 * @returns {Object|null} - Server info or null if not found
 */
function readServerInfo() {
  // Check cache first
  const now = Date.now();
  if (serverInfoCache && (now - serverInfoLastChecked) < SERVER_INFO_CACHE_MS) {
    return serverInfoCache;
  }
  
  try {
    // Look for server info file in parent directories
    let currentDir = __dirname;
    for (let i = 0; i < 4; i++) {
      const serverInfoPath = path.join(currentDir, 'pkm_knowledge_graph_server.json');
      
      if (fs.existsSync(serverInfoPath)) {
        const serverInfo = JSON.parse(fs.readFileSync(serverInfoPath, 'utf8'));
        serverInfoCache = serverInfo;
        serverInfoLastChecked = now;
        return serverInfo;
      }
      
      const parentDir = path.dirname(currentDir);
      if (parentDir === currentDir) break;
      currentDir = parentDir;
    }
  } catch (error) {
    console.error('Error reading server info:', error);
  }
  
  return null;
}

/**
 * Get the backend URL for a specific endpoint
 * @param {string} endpoint - The endpoint path (e.g., '/data', '/')
 * @returns {string} - The complete backend URL
 */
window.KnowledgeGraphAPI.getBackendUrl = function(endpoint) {
  // Try to read actual server info first
  const serverInfo = readServerInfo();
  
  if (serverInfo) {
    return `http://${serverInfo.host}:${serverInfo.port}${endpoint}`;
  }
  
  // Fall back to hardcoded values
  return `http://127.0.0.1:3000${endpoint}`;
};

/**
 * Send data to the backend server
 * @param {Object} data - Data to send to the backend
 * @returns {Promise<boolean>} - Whether the data was sent successfully
 */
window.KnowledgeGraphAPI.sendToBackend = async function(data) {
  const backendUrl = window.KnowledgeGraphAPI.getBackendUrl('/data');
  
  try {
    console.log(`Sending data to backend: ${backendUrl}`);
    const response = await fetch(backendUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (response.ok) {
      console.log('Data sent successfully to backend.');
      logseq.App.showMsg('Sent data to backend successfully!', 'success');
      return true;
    } else {
      console.error(`Backend server responded with status: ${response.status}`);
      logseq.App.showMsg(`Error sending data: Backend responded with ${response.status}`, 'error');
      return false;
    }
  } catch (error) {
    console.error('Failed to send data to backend:', error);
    logseq.App.showMsg('Failed to connect to backend server. Is it running?', 'error');
    return false;
  }
}

/**
 * Send diagnostic information to the backend server
 * @param {string} message - Diagnostic message
 * @param {Object} details - Additional details
 */
window.KnowledgeGraphAPI.sendDiagnosticInfo = async function(message, details = {}) {
  console.log(`DIAGNOSTIC: ${message}`, details);
  
  try {
    const graph = await logseq.App.getCurrentGraph();
    await window.KnowledgeGraphAPI.sendToBackend({
      source: 'Diagnostic',
      timestamp: new Date().toISOString(),
      graphName: graph ? graph.name : 'unknown',
      type_: 'diagnostic',
      payload: JSON.stringify({
        message,
        details
      })
    });
  } catch (error) {
    console.error('Error sending diagnostic info:', error);
  }
}

/**
 * Check if backend server is available
 * @returns {Promise<boolean>} - Whether the backend server is available
 */
window.KnowledgeGraphAPI.checkBackendAvailability = async function() {
  console.log('Checking backend server availability...');
  try {
    const response = await fetch(window.KnowledgeGraphAPI.getBackendUrl('/'), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    return response.ok;
  } catch (error) {
    console.error('Error checking backend availability:', error);
    return false;
  }
}

/**
 * Check if a full sync is needed by querying the backend
 * @returns {Promise<boolean>} - Whether a full sync is needed
 */
window.KnowledgeGraphAPI.checkIfFullSyncNeeded = async function() {
  console.log('Checking if full sync is needed...');
  try {
    // Check if backend is available
    const backendAvailable = await window.KnowledgeGraphAPI.checkBackendAvailability();
    if (!backendAvailable) {
      console.log('Backend not available, skipping full sync check');
      return false;
    }
    
    // Query the backend for sync status
    const response = await fetch(window.KnowledgeGraphAPI.getBackendUrl('/sync/status'), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    if (!response.ok) {
      console.error('Error getting sync status from backend');
      return false;
    }
    
    const status = await response.json();
    console.log('Sync status from backend:', status);
    
    // Return whether a full sync is needed
    return status.full_sync_needed === true;
  } catch (error) {
    console.error('Error checking if full sync is needed:', error);
    await window.KnowledgeGraphAPI.sendDiagnosticInfo('Error checking if full sync needed', { 
      error: error.message,
      stack: error.stack
    });
    return false;
  }
}

/**
 * Update the sync timestamp on the backend
 * @returns {Promise<boolean>} - Whether the update was successful
 */
window.KnowledgeGraphAPI.updateSyncTimestamp = async function() {
  try {
    const response = await fetch(window.KnowledgeGraphAPI.getBackendUrl('/sync'), {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    if (!response.ok) {
      console.error('Error updating sync timestamp on backend');
      return false;
    }
    
    const result = await response.json();
    console.log('Sync timestamp updated:', result);
    
    return result.success === true;
  } catch (error) {
    console.error('Error updating sync timestamp:', error);
    await window.KnowledgeGraphAPI.sendDiagnosticInfo('Error updating sync timestamp', { 
      error: error.message,
      stack: error.stack
    });
    return false;
  }
}

/**
 * Send a batch of data to the backend
 * @param {string} type - Type of data (block or page)
 * @param {Array} batch - Array of data items
 * @param {string} graphName - Name of the graph
 */
window.KnowledgeGraphAPI.sendBatchToBackend = async function(type, batch, graphName) {
  if (batch.length === 0) return;
  
  console.log(`Sending batch of ${batch.length} ${type}s to backend`);
  
  try {
    await window.KnowledgeGraphAPI.sendToBackend({
      source: 'Full Sync',
      timestamp: new Date().toISOString(),
      graphName: graphName,
      type_: `${type}_batch`,
      payload: JSON.stringify(batch)
    });
  } catch (error) {
    console.error(`Error sending ${type} batch:`, error);
  }
}
