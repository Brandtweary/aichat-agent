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
 * TODO: Consider freezing Logseq version to avoid breaking API changes
 * The onChanged API changed from accepting an array to an object structure,
 * breaking our real-time sync without warning. We should investigate:
 * - Pinning to a specific Logseq version
 * - Adding version detection and compatibility layers
 * - Monitoring Logseq release notes for API changes
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
 * - Managing custom block timestamps for incremental sync
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
 * 
 * INCREMENTAL SYNC SYSTEM:
 * =======================
 * The plugin implements an incremental sync system to dramatically improve performance for large
 * databases. Instead of syncing all content every 2 hours, it only syncs what has changed.
 * 
 * How it works:
 * 1. Pages use Logseq's built-in `updatedAt` field for change detection
 * 2. Blocks use custom `cyberorganism-updated-ms` properties managed by this plugin
 * 3. On each sync, only pages/blocks modified since the last sync are processed
 * 
 * Block Timestamp Management:
 * - Since Logseq blocks don't have reliable built-in timestamps, we add custom properties
 * - The property name is converted from kebab-case to camelCase by Logseq: `cyberorganismUpdatedMs`
 * - Timestamps are set when blocks are first synced or when changes are detected
 * - Empty blocks and blocks with only properties are filtered out to avoid clutter
 * 
 * Configuration Required:
 * Users must add the following to their Logseq config.edn to hide the timestamp property:
 * ```clojure
 * :block-hidden-properties #{:cyberorganism-updated-ms}
 * ```
 * TODO: Implement programmatic config.edn editing to automate this
 * 
 * Performance Impact:
 * - Full sync of 4000 pages/40000 blocks: ~20+ seconds
 * - Incremental sync with minimal changes: <1 second
 * - Bottleneck: Thousands of sequential `getPageBlocksTree()` API calls
 * 
 * Known Limitations:
 * - Properties are visible until user adds config and restarts Logseq
 * - Logseq may update page timestamps on startup (contents, favorites, card pages)
 * - Block property persistence depends on Logseq not re-indexing the graph
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
async function processBatch(type, items, graphName, batchSize = 100, source = 'Full Sync') {
  if (!items || items.length === 0) return;
  
  const batch = [];
  
  for (const item of items) {
    try {
      if (type === 'block') {
        // Skip file-level changes (they have path but no uuid)
        if (item.path && !item.uuid) {
          // This is a file change event, not a block change
          continue;
        }
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
        await sendBatchToBackend(type, batch, graphName, source);
        batch.length = 0;
      }
    } catch (error) {
      const identifier = type === 'block' ? item.uuid : `"${item.name}"`;
      KnowledgeGraphAPI.log.error(`Error processing ${type} ${identifier}`, {error: error.message});
    }
  }

  // Send any remaining items
  if (batch.length > 0) {
    await sendBatchToBackend(type, batch, graphName, source);
  }
}

// Global queue for timestamp updates to prevent race conditions
let timestampQueue = new Set();
let processingTimestamps = false;

// Process the timestamp queue in one batch
async function processTimestampQueue() {
  if (processingTimestamps || timestampQueue.size === 0) {
    return;
  }
  
  processingTimestamps = true;
  const currentTimestamp = Date.now();
  const blocksToUpdate = Array.from(timestampQueue);
  timestampQueue.clear();
  
  try {
    for (const blockUuid of blocksToUpdate) {
      try {
        await logseq.Editor.upsertBlockProperty(blockUuid, 'cyberorganism-updated-ms', currentTimestamp);
      } catch (error) {
        KnowledgeGraphAPI.log.error(`Failed to update timestamp for block ${blockUuid}`, {error: error.message});
      }
    }
  } finally {
    processingTimestamps = false;
  }
}

// Handle database changes
async function handleDBChanges(changesData) {
  // Prevent infinite loops from our own timestamp property additions
  if (processingTimestamps) {
    return;
  }
  
  // The changes parameter is an object with blocks array, not an array itself
  if (!changesData || typeof changesData !== 'object') {
    return;
  }
  
  // Extract the blocks and pages from the changes object
  const changes = [{
    blocks: changesData.blocks || [],
    pages: changesData.pages || []
  }];
  
  // Only log if we have actual changes
  const hasChanges = (changesData.blocks && changesData.blocks.length > 0) || 
                    (changesData.pages && changesData.pages.length > 0);
  
  if (!hasChanges) {
    return;
  }
  
  
  // Queue blocks for timestamp updates (avoids race conditions)
  for (const change of changes) {
    if (change.blocks && change.blocks.length > 0) {
      for (const block of change.blocks) {
        if (block.uuid) {
          // Check if this change is just from our timestamp property update
          // If the block has our timestamp property and no other meaningful changes, skip it
          try {
            const fullBlock = await logseq.Editor.getBlock(block.uuid);
            if (fullBlock && fullBlock.properties && fullBlock.properties['cyberorganismUpdatedMs']) {
              // Block already has our timestamp - this might be a change from our own timestamp update
              // Skip adding to queue to prevent infinite loops
              continue;
            } else {
              // This block doesn't have our timestamp yet
            }
          } catch (error) {
            // If we can't check, err on the side of processing
            KnowledgeGraphAPI.log.warn(`Could not check timestamp property for ${block.uuid}, processing anyway`);
          }
          
          timestampQueue.add(block.uuid);
        }
      }
    }
  }
  
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
        // Process blocks silently
        await processBatch('block', change.blocks, graphName, 100, 'Real-time Sync');
      }
      
      // Process page changes  
      if (change.pages && change.pages.length > 0) {
        // Process pages silently
        await processBatch('page', change.pages, graphName, 100, 'Real-time Sync');
      }
    }
    
    // Process any queued timestamp updates after handling the changes
    await processTimestampQueue();
  } catch (error) {
    KnowledgeGraphAPI.log.error('Error handling DB changes', {error: error.message, stack: error.stack});
  }
}

//=============================================================================
// FULL DATABASE SYNC
//=============================================================================

// Sync all pages and blocks in the database
// syncType: 'incremental' (default) or 'full'
async function syncDatabase(syncType = 'incremental') {
  const syncTypeDisplay = syncType === 'full' ? 'full database' : 'incremental';
  KnowledgeGraphAPI.log.info(`Starting ${syncTypeDisplay} sync`);
  
  
  // Check if backend is available with retry logic for critical full sync
  const backendAvailable = await checkBackendAvailabilityWithRetry(3, 2000);
  if (!backendAvailable) {
    KnowledgeGraphAPI.log.error('Backend server not available after retries. Sync aborted.');
    logseq.App.showMsg('Backend server not available after retries. Start the server first.', 'error');
    return false;
  }
  
  try {
    // Get last sync timestamp from backend
    let lastSyncDate = null;
    if (syncType === 'incremental') {
      try {
        const response = await fetch(await window.KnowledgeGraphAPI.getBackendUrl('/sync/status'), {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        });
        
        if (response.ok) {
          const status = await response.json();
          if (status.last_incremental_sync_iso) {
            lastSyncDate = new Date(status.last_incremental_sync_iso);
            KnowledgeGraphAPI.log.debug(`Last incremental sync: ${status.last_incremental_sync_iso}`);
          } else {
            KnowledgeGraphAPI.log.debug('No previous incremental sync found');
          }
        }
      } catch (error) {
        KnowledgeGraphAPI.log.warn('Failed to get sync status, treating as first sync', {error: error.message});
      }
    } else {
      KnowledgeGraphAPI.log.info('Performing full database sync - no timestamp filtering');
    }
    
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
    
    
    // Filter pages based on last sync timestamp if doing incremental sync
    let pagesToSync = allPages;
    if (syncType === 'incremental' && lastSyncDate) {
      pagesToSync = allPages.filter(page => {
        // If page has updated timestamp, check if it's newer than last sync
        if (page.updatedAt) {
          const pageUpdated = new Date(page.updatedAt);
          return pageUpdated > lastSyncDate;
        }
        // If no updated timestamp, include it to be safe
        return true;
      });
      
      KnowledgeGraphAPI.log.info(`Incremental sync: ${pagesToSync.length} of ${allPages.length} pages modified`);
    } else {
      KnowledgeGraphAPI.log.info(`Full sync: processing all ${allPages.length} pages`);
    }
    
    // Track progress
    let pagesProcessed = 0;
    let blocksProcessed = 0;
    
    // Track block sync stats for debugging
    let blocksSkipped = 0;
    let blocksModified = 0;
    let blocksWithoutTimestamp = 0;
    
    // Track all PKM IDs for deletion detection
    const allPkmIds = {
      blocks: [],
      pages: []
    };
    
    // Shared block batch for efficient processing across all pages
    const globalBlockBatch = [];
    
    // Collect ALL page names for deletion detection (not just modified ones)
    for (const page of allPages) {
      if (page.name) {
        allPkmIds.pages.push(page.name);
      }
    }
    
    // Process pages in batches
    for (let i = 0; i < pagesToSync.length; i += 100) {
      const pageBatch = pagesToSync.slice(i, i + 100);
      
      await processBatch('page', pageBatch, graphName);
      pagesProcessed += pageBatch.length;
      
      // Process blocks for these pages
      for (const page of pageBatch) {
        const pageBlocksTree = await logseq.Editor.getPageBlocksTree(page.name);
        if (pageBlocksTree) {
          
          const blockStats = { skipped: 0, modified: 0, noTimestamp: 0 };
          const syncDateForBlocks = syncType === 'incremental' ? lastSyncDate : null;
          await processBlocksRecursively(pageBlocksTree, graphName, globalBlockBatch, 100, syncDateForBlocks, blockStats);
          const pageBlockCount = countBlocksInTree(pageBlocksTree);
          blocksProcessed += pageBlockCount;
          blocksSkipped += blockStats.skipped;
          blocksModified += blockStats.modified;
          blocksWithoutTimestamp += blockStats.noTimestamp;
          
          // Silent progress - no UI spam
        }
      }
    }
    
    // Now collect ALL block IDs for deletion detection (separate pass)
    for (const page of allPages) {
      const pageBlocksTree = await logseq.Editor.getPageBlocksTree(page.name);
      if (pageBlocksTree) {
        collectBlockIds(pageBlocksTree, allPkmIds.blocks);
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
      const displayType = syncType === 'full' ? 'Full database' : 'Incremental';
      logseq.App.showMsg(`${displayType} sync completed successfully!`, 'success');
    }
    
    // Log summary at info level - this is one of our few info logs
    KnowledgeGraphAPI.log.info(`${syncType} sync completed`, {
      pages: pagesProcessed,
      blocks: blocksProcessed,
      pageErrors: summary.totalPageIssues || 0,
      blockErrors: summary.totalBlockIssues || 0,
      syncType: syncType
    });
    
    // Process any queued timestamp updates before finishing sync
    await processTimestampQueue();
    
    // Send all PKM IDs to backend for deletion detection
    try {
      const response = await fetch(await window.KnowledgeGraphAPI.getBackendUrl('/sync/verify'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          pages: allPkmIds.pages,
          blocks: allPkmIds.blocks
        })
      });
      
      if (!response.ok) {
        KnowledgeGraphAPI.log.warn('Failed to verify PKM IDs with backend');
      }
    } catch (error) {
      KnowledgeGraphAPI.log.warn('Failed to send PKM IDs for deletion detection', {error: error.message});
    }
    
    return true;
  } catch (error) {
    KnowledgeGraphAPI.log.error('Error during full database sync', {error: error.message, stack: error.stack});
    logseq.App.showMsg('Error during full database sync.', 'error');
    return false;
  }
}

// Process blocks recursively with batching
async function processBlocksRecursively(blocks, graphName, blockBatch, batchSize, lastSyncDate = null, stats = null) {
  if (!blocks || !Array.isArray(blocks)) return;
  
  for (const block of blocks) {
    try {
      // Skip blocks without UUIDs
      if (!block.uuid) {
        KnowledgeGraphAPI.log.error('Block missing UUID in recursive processing', {block});
        continue;
      }
      
      // Get the full block data to check for our custom timestamp property
      const fullBlock = await logseq.Editor.getBlock(block.uuid);
      if (!fullBlock) {
        KnowledgeGraphAPI.log.error(`Could not fetch full block data for ${block.uuid}`);
        continue;
      }
      
      // Check for our custom timestamp property
      let blockUpdatedMs = fullBlock.properties?.['cyberorganismUpdatedMs'];
      let shouldSync = true;
      
      if (lastSyncDate) {
        const lastSyncMs = lastSyncDate.getTime();
        
        if (blockUpdatedMs) {
          // Block has timestamp - compare with last sync
          const blockUpdatedTime = parseInt(blockUpdatedMs);
          if (blockUpdatedTime <= lastSyncMs) {
            shouldSync = false;
            if (stats) stats.skipped++;
          } else {
            if (stats) stats.modified++;
          }
        } else {
          // Block missing timestamp - initialize it and treat as modified
          // Queue block for timestamp initialization
          timestampQueue.add(block.uuid);
          if (stats) stats.noTimestamp++;
        }
      } else {
        // Full sync - ensure all blocks have timestamps
        if (!blockUpdatedMs) {
          // Queue block for timestamp initialization
          timestampQueue.add(block.uuid);
          if (stats) stats.noTimestamp++;
        } else {
          if (stats) stats.modified++;
        }
      }
      
      // Only process if we should sync this block
      if (shouldSync) {
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
      }
      
      // Process children recursively
      if (block.children && block.children.length > 0) {
        await processBlocksRecursively(block.children, graphName, blockBatch, batchSize, lastSyncDate, stats);
      }
    } catch (blockError) {
      KnowledgeGraphAPI.log.error(`Error processing block ${block.uuid}`, {error: blockError.message});
      // Continue with other blocks even if one fails
    }
  }
}

// Send a batch of data to the backend
async function sendBatchToBackend(type, batch, graphName, source = 'Full Sync') {
  // Use the global KnowledgeGraphAPI object's sendBatchToBackend function
  return KnowledgeGraphAPI.sendBatchToBackend(type, batch, graphName, source);
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

// Collect all block IDs from a tree recursively
function collectBlockIds(blocks, idArray) {
  if (!blocks || !Array.isArray(blocks)) return;
  
  for (const block of blocks) {
    if (block.uuid) {
      idArray.push(block.uuid);
    }
    
    if (block.children && block.children.length > 0) {
      collectBlockIds(block.children, idArray);
    }
  }
}

//=============================================================================
// SYNC STATUS MANAGEMENT
//=============================================================================

// Check what type of sync is needed by querying the backend
async function checkSyncStatus() {
  try {
    const response = await fetch(await window.KnowledgeGraphAPI.getBackendUrl('/sync/status'), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    if (!response.ok) {
      KnowledgeGraphAPI.log.error('Failed to get sync status from backend');
      return { needsSync: false };
    }
    
    const status = await response.json();
    
    // Check sync needs in priority order
    // Force flags override config settings
    if (status.force_full_sync || (status.true_full_sync_needed && status.sync_config?.enable_full_sync)) {
      return { needsSync: true, syncType: 'full' };
    } else if (status.force_incremental_sync || status.incremental_sync_needed) {
      return { needsSync: true, syncType: 'incremental' };
    } else {
      return { needsSync: false };
    }
  } catch (error) {
    KnowledgeGraphAPI.log.error('Error checking sync status', {error: error.message});
    return { needsSync: false };
  }
}

// Update the sync timestamp on the backend
async function updateSyncTimestamp(syncType = 'incremental') {
  // Use the global KnowledgeGraphAPI object's updateSyncTimestamp function
  return KnowledgeGraphAPI.updateSyncTimestamp(syncType);
}

//=============================================================================
// PLUGIN INITIALIZATION
//=============================================================================

// Main function for plugin logic
async function main() {
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
      let statusMessage = 'Sync Status:\n\n';
      
      // Incremental sync info
      statusMessage += 'Incremental Sync:\n';
      if (status.last_incremental_sync) {
        const lastSync = new Date(status.last_incremental_sync);
        statusMessage += `- Last sync: ${lastSync.toLocaleString()}\n`;
        statusMessage += `- Hours since: ${status.hours_since_incremental?.toFixed(1) || 'N/A'}\n`;
      } else {
        statusMessage += '- No previous sync\n';
      }
      statusMessage += `- Needed: ${status.incremental_sync_needed ? 'Yes' : 'No'}\n\n`;
      
      // Full sync info
      statusMessage += 'Full Database Sync:\n';
      statusMessage += `- Enabled: ${status.sync_config?.enable_full_sync ? 'Yes' : 'No'}\n`;
      if (status.sync_config?.enable_full_sync) {
        if (status.last_true_full_sync) {
          const lastSync = new Date(status.last_true_full_sync);
          statusMessage += `- Last sync: ${lastSync.toLocaleString()}\n`;
          statusMessage += `- Hours since: ${status.hours_since_full?.toFixed(1) || 'N/A'}\n`;
        } else {
          statusMessage += '- No previous sync\n';
        }
        statusMessage += `- Needed: ${status.true_full_sync_needed ? 'Yes' : 'No'}\n`;
      }
      
      statusMessage += `\nGraph Stats:\n`;
      statusMessage += `- Nodes: ${status.node_count}\n`;
      statusMessage += `- References: ${status.edge_count}`;
      
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
  
  // Check if we need to do any sync immediately
  const syncStatus = await checkSyncStatus();
  
  if (syncStatus.needsSync) {
    const success = await syncDatabase(syncStatus.syncType);
    
    if (success) {
      await updateSyncTimestamp(syncStatus.syncType);
      // Success message already shown by syncDatabase
    } else {
      // Error message already shown by syncDatabase
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
