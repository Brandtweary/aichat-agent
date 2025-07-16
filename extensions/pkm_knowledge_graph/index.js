/**
 * @module index
 * @description Main entry point for the Logseq Knowledge Graph Plugin
 * 
 * CRITICAL WARNING FOR LLM ASSISTANTS:
 * =====================================
 * This is a BROWSER-BASED Logseq plugin. DO NOT use Node.js features like:
 * - require() or import statements
 * - module.exports
 * - fs, path, or any Node.js modules
 * 
 * All dependencies are loaded via <script> tags in index.html and exposed as globals.
 * Breaking this plugin makes it very difficult to debug due to Logseq's opaque error handling.
 * 
 * DO NOT make "improvements" or "modernizations" without explicit user request.
 * This code works as-is. Random changes have broken production systems before.
 * 
 * This module orchestrates the entire plugin functionality, connecting Logseq to a Rust-based 
 * knowledge graph backend. It handles initialization, event registration, data synchronization,
 * and communication between the Logseq frontend and the Rust backend server.
 * 
 * Key responsibilities:
 * - Plugin initialization and setup
 * - Registering Logseq slash commands for user interaction
 * - Setting up event listeners for database changes and page navigation
 * - Coordinating data processing and validation through the KnowledgeGraphDataProcessor
 * - Managing backend communication through the KnowledgeGraphAPI
 * - Implementing full database sync and incremental sync logic
 * - Handling batch processing of blocks and pages
 * - Tracking and reporting validation issues
 * 
 * API Communication (via window.KnowledgeGraphAPI):
 * - sendToBackend(data) - Send data to the backend server
 * - checkSyncStatus() - Check current sync status with backend
 * - getBackendUrl(endpoint) - Get the backend URL for an endpoint
 * - updateSyncTimestamp() - Update the last sync timestamp
 * - log.error/warn/info/debug/trace(message, details, source) - Send logs to backend
 * 
 * Message types sent to backend:
 * - type_: 'block' - Individual block data
 * - type_: 'blocks' - Batch of block data
 * - type_: 'page' - Individual page data
 * - type_: 'pages' - Batch of page data
 * - type_: 'plugin_initialized' - Plugin startup notification
 * 
 * Public interfaces:
 * - The plugin registers the following slash command in Logseq:
 *   - "/Check Sync Status": Checks and displays the current sync status with the backend
 * 
 * The plugin automatically:
 * - Monitors database changes via logseq.DB.onChanged
 * - Tracks page navigation via logseq.App.onRouteChanged
 * - Checks if a full sync is needed on startup
 * 
 * Dependencies:
 * - api.js: Handles all HTTP communication with the backend (loaded as KnowledgeGraphAPI global)
 * - data_processor.js: Processes and validates Logseq data (loaded as KnowledgeGraphDataProcessor global)
 */

/**
 * Logseq Knowledge Graph Plugin
 * Connects Logseq to a Rust-based knowledge graph backend
 */

// The API and config are loaded via script tags in index.html
// They are available as global objects: KnowledgeGraphAPI and KnowledgeGraphDataProcessor

//=============================================================================
// LOGSEQ API INTERACTION
//=============================================================================

//=============================================================================
// BACKEND COMMUNICATION
// These functions now use the global KnowledgeGraphAPI object
//=============================================================================

// This function is now deprecated - use KnowledgeGraphAPI.log instead
// Keeping for backwards compatibility but redirecting to new logging system
async function sendDiagnosticInfo(message, details = {}) {
  return KnowledgeGraphAPI.log.info(message, details, 'Diagnostic');
}

// Check if backend server is available
async function checkBackendAvailability() {
  // Use the global KnowledgeGraphAPI object's checkBackendAvailability function
  return KnowledgeGraphAPI.checkBackendAvailability();
}

// Check if backend server is available with retry logic
async function checkBackendAvailabilityWithRetry(maxRetries = 3, retryDelayMs = 1000) {
  // Use the global KnowledgeGraphAPI object's checkBackendAvailabilityWithRetry function
  return KnowledgeGraphAPI.checkBackendAvailabilityWithRetry(maxRetries, retryDelayMs);
}

//=============================================================================
// DATA PROCESSING & EXTRACTION
// These functions now use the global KnowledgeGraphDataProcessor object
//=============================================================================

// Process block data and extract relevant information
async function processBlockData(block) {
  return KnowledgeGraphDataProcessor.processBlockData(block);
}

// Process page data and extract relevant information
async function processPageData(page) {
  return KnowledgeGraphDataProcessor.processPageData(page);
}

//=============================================================================
// DATA VALIDATION
// These functions now use the global KnowledgeGraphDataProcessor object
//=============================================================================

// Validate block data before sending to backend
function validateBlockData(blockData) {
  return KnowledgeGraphDataProcessor.validateBlockData(blockData);
}

// Validate page data before sending to backend
function validatePageData(pageData) {
  return KnowledgeGraphDataProcessor.validatePageData(pageData);
}

//=============================================================================
// VALIDATION ISSUE TRACKING
// Now uses the global KnowledgeGraphDataProcessor.validationIssues object
//=============================================================================

// Global validation issue tracker - reference to the one in KnowledgeGraphDataProcessor
const validationIssues = KnowledgeGraphDataProcessor.validationIssues;

//=============================================================================
// REAL-TIME SYNC HANDLING
//=============================================================================

// Process a batch of pages or blocks
async function processBatch(type, items, graphName, batchSize = 100) {
  if (!items || items.length === 0) return;
  
  // Only log large batches at debug level
  if (items.length > 100) {
    KnowledgeGraphAPI.log.debug(`Processing ${items.length} ${type}s`);
  }
  const batch = [];
  
  for (const item of items) {
    try {
      if (type === 'block') {
        if (!item.uuid) {
          KnowledgeGraphAPI.log.error('Block missing UUID', {block: item});
          continue;
        }
        const blockData = await processBlockData(item);
        if (!blockData) {
          // Skip silently - empty blocks are normal
          continue;
        }
        const validation = validateBlockData(blockData);
        if (validation.valid) {
          batch.push(blockData);
        } else {
          KnowledgeGraphAPI.log.warn(`Invalid block data for UUID ${item.uuid}`, validation.errors);
          validationIssues.addBlockIssue(blockData.id, blockData.page, validation.errors);
        }
      } else if (type === 'page') {
        if (!item.name) {
          KnowledgeGraphAPI.log.error('Page missing name', {page: item});
          continue;
        }
        const pageData = await processPageData(item);
        if (!pageData) {
          // Skip silently
          continue;
        }
        const validation = validatePageData(pageData);
        if (validation.valid) {
          batch.push(pageData);
        } else {
          KnowledgeGraphAPI.log.warn(`Invalid page data for "${item.name}"`, validation.errors);
          validationIssues.addPageIssue(pageData.name, validation.errors);
        }
      }

      if (batch.length >= batchSize) {
        await sendBatchToBackend(type, batch, graphName);
        batch.length = 0;
      }
    } catch (error) {
      const identifier = type === 'block' ? item.uuid : `"${item.name}"`;
      KnowledgeGraphAPI.log.error(`Error processing ${type} ${identifier}`, {error: error.message});
    }
  }

  // Send any remaining items
  if (batch.length > 0) {
    await sendBatchToBackend(type, batch, graphName);
  }
}

// Handle database changes
async function handleDBChanges(changes) {
  // Skip if no changes or empty changes array
  if (!changes || !Array.isArray(changes) || changes.length === 0) {
    return;
  }
  
  // Silent happy path - no log for routine changes
  
  // Check if backend is available before processing changes (light retry for real-time)
  try {
    const backendAvailable = await checkBackendAvailabilityWithRetry(1, 500);
    if (!backendAvailable) {
      KnowledgeGraphAPI.log.warn('Backend server not available. Real-time changes will not be processed.');
      return;
    }
    
    // Get current graph name
    const graph = await logseq.App.getCurrentGraph();
    if (!graph || !graph.name) {
      KnowledgeGraphAPI.log.error('Failed to get current graph name.');
      return;
    }
    
    const graphName = graph.name;
    
    // Process each change
    for (const change of changes) {
      // Process block changes
      if (change.blocks && change.blocks.length > 0) {
        await processBatch('block', change.blocks, graphName, 100);
      }
      
      // Process page changes  
      if (change.pages && change.pages.length > 0) {
        await processBatch('page', change.pages, graphName, 100);
      }
    }
  } catch (error) {
    KnowledgeGraphAPI.log.error('Error handling DB changes', {error: error.message, stack: error.stack});
  }
}

//=============================================================================
// FULL DATABASE SYNC
//=============================================================================

// Sync all pages and blocks in the database
async function syncFullDatabase() {
  const syncStartTime = performance.now();
  KnowledgeGraphAPI.log.info('Starting full database sync');
  
  // Check if backend is available with retry logic for critical full sync
  const backendAvailable = await checkBackendAvailabilityWithRetry(3, 2000);
  if (!backendAvailable) {
    KnowledgeGraphAPI.log.error('Backend server not available after retries. Sync aborted.');
    logseq.App.showMsg('Backend server not available after retries. Start the server first.', 'error');
    return false;
  }
  
  try {
    // Reset validation issues tracker
    validationIssues.reset();
    
    // Get current graph
    const graph = await logseq.App.getCurrentGraph();
    if (!graph) {
      KnowledgeGraphAPI.log.error('Failed to get current graph.');
      logseq.App.showMsg('Failed to get current graph.', 'error');
      return false;
    }
    
    const graphName = graph.name;
    
    // Get all pages
    const allPages = await logseq.Editor.getAllPages();
    if (!allPages || !Array.isArray(allPages)) {
      KnowledgeGraphAPI.log.error('Failed to fetch pages from database.');
      logseq.App.showMsg('Failed to fetch pages from database.', 'error');
      return false;
    }
    
    KnowledgeGraphAPI.log.debug(`Found ${allPages.length} pages to sync`);
    
    // Track progress
    let pagesProcessed = 0;
    let blocksProcessed = 0;
    
    // Shared block batch for efficient processing across all pages
    const globalBlockBatch = [];
    
    // Process pages in batches
    for (let i = 0; i < allPages.length; i += 100) {
      const pageBatch = allPages.slice(i, i + 100);
      
      await processBatch('page', pageBatch, graphName);
      pagesProcessed += pageBatch.length;
      
      // Silent progress - no UI spam
      
      // Process blocks for these pages
      for (const page of pageBatch) {
        const pageBlocksTree = await logseq.Editor.getPageBlocksTree(page.name);
        if (pageBlocksTree) {
          await processBlocksRecursively(pageBlocksTree, graphName, globalBlockBatch, 100);
          const pageBlockCount = countBlocksInTree(pageBlocksTree);
          blocksProcessed += pageBlockCount;
          
          // Silent progress - no UI spam
        }
      }
    }
    
    // Send any remaining blocks in the final batch
    if (globalBlockBatch.length > 0) {
      await sendBatchToBackend('block', globalBlockBatch.slice(), graphName);
      globalBlockBatch.splice(0); // Clear for consistency
    }

    // Display validation summary if there were issues
    const summary = validationIssues.getSummary();
    if (summary.totalBlockIssues > 0 || summary.totalPageIssues > 0) {
      KnowledgeGraphAPI.log.warn('Validation issues during sync', summary);
      
      // Show a user-friendly message with counts
      logseq.App.showMsg(
        `Sync completed with issues: ${summary.totalBlockIssues} block issues, ${summary.totalPageIssues} page issues.`, 
        'warning'
      );
    } else {
      // Show success message
      logseq.App.showMsg('Full database sync completed successfully!', 'success');
    }
    
    // Log summary at info level - this is one of our few info logs
    const syncDuration = (performance.now() - syncStartTime) / 1000;
    KnowledgeGraphAPI.log.info('Full sync completed', {
      pages: pagesProcessed,
      blocks: blocksProcessed,
      pageErrors: summary.totalPageIssues || 0,
      blockErrors: summary.totalBlockIssues || 0,
      durationSeconds: syncDuration.toFixed(3)
    });
    
    return true;
  } catch (error) {
    KnowledgeGraphAPI.log.error('Error during full database sync', {error: error.message, stack: error.stack});
    logseq.App.showMsg('Error during full database sync.', 'error');
    return false;
  }
}

// Process blocks recursively with batching
async function processBlocksRecursively(blocks, graphName, blockBatch, batchSize) {
  if (!blocks || !Array.isArray(blocks)) return;
  
  for (const block of blocks) {
    try {
      // Skip blocks without UUIDs
      if (!block.uuid) {
        KnowledgeGraphAPI.log.error('Block missing UUID in recursive processing', {block});
        continue;
      }
      
      // Process this block
      const blockData = await processBlockData(block);
      if (!blockData) {
        // Skip silently - empty blocks are normal
        continue;
      }
      
      const validation = validateBlockData(blockData);
      if (validation.valid) {
        // Add to block batch instead of sending immediately
        blockBatch.push(blockData);
        
        // Send batch if it reaches the batch size
        if (blockBatch.length >= batchSize) {
          await sendBatchToBackend('block', blockBatch.slice(), graphName);
          blockBatch.splice(0); // Clear array safely
        }
      } else {
        KnowledgeGraphAPI.log.warn(`Invalid block data for ${block.uuid}`, validation.errors);
        validationIssues.addBlockIssue(blockData.id, blockData.page, validation.errors);
      }
      
      // Process children recursively
      if (block.children && block.children.length > 0) {
        await processBlocksRecursively(block.children, graphName, blockBatch, batchSize);
      }
    } catch (blockError) {
      KnowledgeGraphAPI.log.error(`Error processing block ${block.uuid}`, {error: blockError.message});
      // Continue with other blocks even if one fails
    }
  }
}

// Send a batch of data to the backend
async function sendBatchToBackend(type, batch, graphName) {
  // Use the global KnowledgeGraphAPI object's sendBatchToBackend function
  return KnowledgeGraphAPI.sendBatchToBackend(type, batch, graphName);
}

// Count blocks in a tree (for progress reporting)
function countBlocksInTree(blocks) {
  if (!blocks || !Array.isArray(blocks)) return 0;
  
  let count = blocks.length;
  
  for (const block of blocks) {
    if (block.children && block.children.length > 0) {
      count += countBlocksInTree(block.children);
    }
  }
  
  return count;
}

//=============================================================================
// SYNC STATUS MANAGEMENT
//=============================================================================

// Check if a full sync is needed by querying the backend
async function checkIfFullSyncNeeded() {
  // Use the global KnowledgeGraphAPI object's checkIfFullSyncNeeded function
  return KnowledgeGraphAPI.checkIfFullSyncNeeded();
}

// Update the sync timestamp on the backend
async function updateSyncTimestamp() {
  // Use the global KnowledgeGraphAPI object's updateSyncTimestamp function
  return KnowledgeGraphAPI.updateSyncTimestamp();
}

//=============================================================================
// PLUGIN INITIALIZATION
//=============================================================================

// Main function for plugin logic
async function main() {
  // Silent startup - no need to log initialization
  
  // Check if required global objects are available
  if (typeof window.KnowledgeGraphAPI === 'undefined') {
    // Can't use our logging API if it doesn't exist!
    console.error('ERROR: KnowledgeGraphAPI not found! api.js may not have loaded properly.');
    logseq.App.showMsg('Plugin initialization failed: API module not loaded', 'error');
    return;
  }
  
  if (typeof window.KnowledgeGraphDataProcessor === 'undefined') {
    KnowledgeGraphAPI.log.error('KnowledgeGraphDataProcessor not found! data_processor.js may not have loaded properly.');
    logseq.App.showMsg('Plugin initialization failed: Data processor module not loaded', 'error');
    return;
  }

  // Register a command to check sync status
  logseq.Editor.registerSlashCommand('Check Sync Status', async () => {
    logseq.App.showMsg('Checking sync status...', 'info');
    
    // Test backend availability with retry for user-initiated check
    const backendAvailable = await checkBackendAvailabilityWithRetry(2, 1000);
    if (!backendAvailable) {
      logseq.App.showMsg('Backend server not available after retries. Start the server first.', 'error');
      return;
    }
    
    // Get sync status from backend
    try {
      const response = await fetch(await window.KnowledgeGraphAPI.getBackendUrl('/sync/status'), {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      
      if (!response.ok) {
        logseq.App.showMsg('Error getting sync status from backend', 'error');
        return;
      }
      
      const status = await response.json();
      
      // Display sync status
      let statusMessage = 'Sync Status:\n';
      
      if (status.last_full_sync) {
        const lastSync = new Date(status.last_full_sync);
        statusMessage += `- Last sync: ${lastSync.toLocaleString()}\n`;
        statusMessage += `- Hours since sync: ${status.hours_since_sync}\n`;
      } else {
        statusMessage += '- No previous sync detected\n';
      }
      
      statusMessage += `- Nodes: ${status.node_count}\n`;
      statusMessage += `- References: ${status.reference_count}\n`;
      statusMessage += `- Full sync needed: ${status.full_sync_needed ? 'Yes' : 'No'}`;
      
      logseq.App.showMsg(statusMessage, 'info');
    } catch (error) {
      KnowledgeGraphAPI.log.error('Error checking sync status', {error: error.message});
      logseq.App.showMsg('Error checking sync status.', 'error');
    }
  });

  // Set up DB change monitoring
  logseq.DB.onChanged(handleDBChanges);
  
  // Listen for page open events
  logseq.App.onRouteChanged(async ({ path }) => {
    if (path.startsWith('/page/')) {
      const pageName = decodeURIComponent(path.substring(6));
      // Silent - we don't need to log every page navigation
      
      // You could trigger a sync here if needed
    }
  });
  
  // Send initialization signal to backend first
  try {
    const result = await KnowledgeGraphAPI.sendToBackend({
      source: 'PKM Plugin Startup',
      timestamp: Date.now().toString(),
      type_: 'plugin_initialized',
      payload: JSON.stringify({ message: 'PKM Knowledge Graph Plugin initialized successfully' })
    });
    
    // Show single UI notification for successful plugin load
    if (result) {
      logseq.App.showMsg('Cyberorganism initialized', 'success');
    }
  } catch (error) {
    KnowledgeGraphAPI.log.error('Failed to send plugin initialization signal', {error: error.message});
  }
  
  // Check if we need to do a full sync immediately
  const needsFullSync = await checkIfFullSyncNeeded();
  
  if (needsFullSync) {
    const success = await syncFullDatabase();
    
    if (success) {
      await updateSyncTimestamp();
      // Success message already shown by syncFullDatabase
    } else {
      // Error message already shown by syncFullDatabase
    }
    
    // Signal sync completion regardless of success/failure
    await KnowledgeGraphAPI.sendToBackend({
      source: 'PKM Plugin Sync',
      timestamp: Date.now().toString(),
      type_: 'sync_complete',
      payload: JSON.stringify({ success })
    });
  } else {
    // No sync needed - signal completion immediately
    await KnowledgeGraphAPI.sendToBackend({
      source: 'PKM Plugin Sync',
      timestamp: Date.now().toString(),
      type_: 'sync_complete', 
      payload: JSON.stringify({ syncSkipped: true })
    });
  }
}

// Initialize the plugin
logseq.ready(main).catch((error) => {
  // Can't use our logging API here if initialization fails
  console.error('Plugin initialization failed:', error);
});
