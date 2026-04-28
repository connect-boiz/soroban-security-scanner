/**
 * Offline Sync Manager for Soroban Security Scanner
 * Handles network status detection and data synchronization
 */

class SecurityScannerSync {
  constructor() {
    this.isOnline = navigator.onLine;
    this.storage = new SecurityScannerStorage();
    this.syncInProgress = false;
    this.retryAttempts = 3;
    this.retryDelay = 5000; // 5 seconds
    
    this.init();
  }

  /**
   * Initialize offline sync manager
   */
  async init() {
    // Listen for network changes
    window.addEventListener('online', () => this.handleOnline());
    window.addEventListener('offline', () => this.handleOffline());
    
    // Initialize storage
    await this.storage.init();
    
    // Load and cache security rules
    await this.cacheSecurityRules();
    
    // Try to sync if we're online
    if (this.isOnline) {
      this.attemptSync();
    }
    
    console.log('[SecurityScannerSync] Initialized, online:', this.isOnline);
  }

  /**
   * Handle coming back online
   */
  async handleOnline() {
    console.log('[SecurityScannerSync] Connection restored');
    this.isOnline = true;
    
    // Update UI
    this.updateConnectionStatus(true);
    
    // Start sync process
    this.attemptSync();
    
    // Clear expired cache
    await this.storage.clearExpiredCache();
  }

  /**
   * Handle going offline
   */
  handleOffline() {
    console.log('[SecurityScannerSync] Connection lost');
    this.isOnline = false;
    
    // Update UI
    this.updateConnectionStatus(false);
    
    // Show offline notification
    this.showOfflineNotification();
  }

  /**
   * Update connection status in UI
   */
  updateConnectionStatus(isOnline) {
    const statusElements = document.querySelectorAll('[data-connection-status]');
    
    statusElements.forEach(element => {
      if (isOnline) {
        element.textContent = 'Online';
        element.className = element.className.replace(/offline/g, 'online');
      } else {
        element.textContent = 'Offline';
        element.className = element.className.replace(/online/g, 'offline');
      }
    });
  }

  /**
   * Show offline notification
   */
  showOfflineNotification() {
    // Create notification if it doesn't exist
    if (!document.getElementById('offline-notification')) {
      const notification = document.createElement('div');
      notification.id = 'offline-notification';
      notification.innerHTML = `
        <div class="offline-notification-content">
          <span class="offline-icon">🔍</span>
          <span>You're offline. Security scanner will use cached data.</span>
          <button onclick="this.parentElement.parentElement.remove()">×</button>
        </div>
      `;
      
      // Add styles
      notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: #fed7d7;
        color: #c53030;
        padding: 1rem;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        z-index: 1000;
        max-width: 350px;
      `;
      
      document.body.appendChild(notification);
      
      // Auto-hide after 10 seconds
      setTimeout(() => {
        if (notification.parentElement) {
          notification.remove();
        }
      }, 10000);
    }
  }

  /**
   * Cache security rules for offline access
   */
  async cacheSecurityRules() {
    try {
      const response = await fetch('/api/rules');
      if (response.ok) {
        const rules = await response.json();
        await this.storage.storeSecurityRules(rules.data);
        console.log('[SecurityScannerSync] Security rules cached for offline use');
      }
    } catch (error) {
      console.warn('[SecurityScannerSync] Failed to cache security rules:', error);
    }
  }

  /**
   * Queue contract scan for offline processing
   */
  async queueScan(contractId, options = {}) {
    const queuedScan = {
      contractId,
      options,
      timestamp: Date.now(),
      status: 'queued',
      attempts: 0
    };
    
    await this.storage.queueScan(contractId, options);
    console.log('[SecurityScannerSync] Scan queued:', contractId);
    
    // Try to sync immediately if online
    if (this.isOnline && !this.syncInProgress) {
      this.attemptSync();
    }
    
    return queuedScan;
  }

  /**
   * Attempt to sync queued scans
   */
  async attemptSync() {
    if (this.syncInProgress || !this.isOnline) {
      return;
    }
    
    this.syncInProgress = true;
    console.log('[SecurityScannerSync] Starting sync process');
    
    try {
      // Get queued scans
      const queuedScans = await this.storage.getQueuedScans();
      
      // Process each queued scan
      for (const queuedScan of queuedScans) {
        await this.processQueuedScan(queuedScan);
      }
      
      console.log('[SecurityScannerSync] Sync completed');
    } catch (error) {
      console.error('[SecurityScannerSync] Sync failed:', error);
    } finally {
      this.syncInProgress = false;
    }
  }

  /**
   * Process individual queued scan
   */
  async processQueuedScan(queuedScan) {
    const { contractId, options, id } = queuedScan;
    
    try {
      const response = await fetch('/api/scan', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ contractId, options })
      });
      
      if (response.ok) {
        const result = await response.json();
        
        // Store result locally
        await this.storage.storeScanResult(result.data);
        
        console.log('[SecurityScannerSync] Scan synced:', contractId);
        
        // Notify user of completion
        this.showScanCompletedNotification(contractId, result.data);
        
      } else {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
    } catch (error) {
      queuedScan.attempts++;
      
      if (queuedScan.attempts >= this.retryAttempts) {
        queuedScan.status = 'failed';
        console.error('[SecurityScannerSync] Scan failed permanently:', contractId, error);
      } else {
        console.warn('[SecurityScannerSync] Scan failed, will retry:', contractId, error);
        // Exponential backoff for retry
        await this.delay(this.retryDelay * Math.pow(2, queuedScan.attempts - 1));
      }
    }
  }

  /**
   * Enhanced fetch with offline fallback
   */
  async fetchWithFallback(url, options = {}) {
    const cacheKey = `${options.method || 'GET'}:${url}`;
    
    try {
      // Try network first
      const response = await fetch(url, options);
      
      if (response.ok) {
        // Cache successful response
        const responseData = await response.clone().json();
        await this.storage.cacheApiResponse(url, responseData);
        return response;
      }
      
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      
    } catch (error) {
      console.log('[SecurityScannerSync] Network failed, trying cache:', url);
      
      // Try to get from cache
      const cachedData = await this.storage.getCachedApiResponse(cacheKey);
      if (cachedData) {
        console.log('[SecurityScannerSync] Serving from cache:', url);
        return new Response(JSON.stringify(cachedData), {
          status: 200,
          headers: { 'Content-Type': 'application/json' }
        });
      }
      
      // If it's a scan request, queue it for later sync
      if (url.includes('/api/scan') && options.method === 'POST') {
        const body = JSON.parse(options.body);
        await this.queueScan(body.contractId, body.options);
        
        return new Response(JSON.stringify({
          error: 'offline',
          message: 'Scan queued for execution when connection is restored',
          offline: true
        }), {
          status: 202, // Accepted
          headers: { 'Content-Type': 'application/json' }
        });
      }
      
      // Return offline error
      return new Response(JSON.stringify({
        error: 'offline',
        message: 'No cached data available',
        offline: true
      }), {
        status: 503,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  }

  /**
   * Show notification when scan completes
   */
  showScanCompletedNotification(contractId, result) {
    const notification = document.createElement('div');
    notification.className = 'scan-completed-notification';
    notification.innerHTML = `
      <div class="notification-content">
        <span class="notification-icon">✅</span>
        <div>
          <strong>Scan Completed</strong><br>
          <small>Contract: ${contractId}</small><br>
          <small>Score: ${result.score}/100</small>
        </div>
        <button onclick="this.parentElement.parentElement.remove()">×</button>
      </div>
    `;
    
    // Add styles
    notification.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: #c6f6d5;
      color: #22543d;
      padding: 1rem;
      border-radius: 8px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.15);
      z-index: 1001;
      max-width: 300px;
    `;
    
    document.body.appendChild(notification);
    
    // Auto-hide after 5 seconds
    setTimeout(() => {
      if (notification.parentElement) {
        notification.remove();
      }
    }, 5000);
  }

  /**
   * Get sync status
   */
  getSyncStatus() {
    return {
      isOnline: this.isOnline,
      syncInProgress: this.syncInProgress,
      storageStats: this.storage.getStorageStats()
    };
  }

  /**
   * Delay utility
   */
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Clear all offline data
   */
  async clearAllData() {
    await this.storage.clearAll();
    console.log('[SecurityScannerSync] All offline data cleared');
  }

  /**
   * Force sync now
   */
  async forceSync() {
    if (this.isOnline && !this.syncInProgress) {
      await this.attemptSync();
      return true;
    }
    return false;
  }
}

// Initialize the sync manager
window.securityScannerSync = new SecurityScannerSync();

// Export for use in other scripts
window.SecurityScannerSync = SecurityScannerSync;
