/**
 * Offline Integration Script for Soroban Security Scanner
 * Integrates service worker, offline storage, and sync functionality
 */

class SorobanSecurityScannerOffline {
  constructor() {
    this.serviceWorkerSupported = 'serviceWorker' in navigator;
    this.indexedDBSupported = 'indexedDB' in window;
    this.isOnline = navigator.onLine;
    this.syncManager = null;
    
    this.init();
  }

  /**
   * Initialize offline functionality
   */
  async init() {
    console.log('[SorobanSecurityScannerOffline] Initializing offline functionality');
    
    // Check browser support
    if (!this.serviceWorkerSupported) {
      console.warn('[SorobanSecurityScannerOffline] Service workers not supported');
    }
    
    if (!this.indexedDBSupported) {
      console.warn('[SorobanSecurityScannerOffline] IndexedDB not supported');
    }
    
    // Register service worker
    await this.registerServiceWorker();
    
    // Initialize sync manager
    if (window.SecurityScannerSync) {
      this.syncManager = window.SecurityScannerSync;
    }
    
    // Add offline detection to DOM
    this.addOfflineIndicators();
    
    // Setup API interceptors
    this.setupApiInterceptors();
    
    // Setup periodic cache maintenance
    this.setupCacheMaintenance();
    
    console.log('[SorobanSecurityScannerOffline] Offline functionality initialized');
  }

  /**
   * Register service worker
   */
  async registerServiceWorker() {
    if (!this.serviceWorkerSupported) return;
    
    try {
      const registration = await navigator.serviceWorker.register('/sw.js');
      console.log('[SorobanSecurityScannerOffline] Service worker registered:', registration.scope);
      
      // Listen for service worker updates
      registration.addEventListener('updatefound', () => {
        console.log('[SorobanSecurityScannerOffline] Service worker update found');
        const newWorker = registration.installing;
        
        newWorker.addEventListener('statechange', () => {
          if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
            // New service worker is available, show update notification
            this.showUpdateNotification();
          }
        });
      });
      
      // Listen for controller changes (service worker activated)
      navigator.serviceWorker.addEventListener('controllerchange', () => {
        console.log('[SorobanSecurityScannerOffline] Service worker controller changed');
        window.location.reload();
      });
      
    } catch (error) {
      console.error('[SorobanSecurityScannerOffline] Service worker registration failed:', error);
    }
  }

  /**
   * Add offline indicators to the page
   */
  addOfflineIndicators() {
    // Connection status is already in the HTML, just update it
    this.updateConnectionStatus(this.isOnline);
    
    // Listen for connection changes
    window.addEventListener('online', () => this.updateConnectionStatus(true));
    window.addEventListener('offline', () => this.updateConnectionStatus(false));
  }

  /**
   * Update connection status indicator
   */
  updateConnectionStatus(isOnline) {
    this.isOnline = isOnline;
    const indicator = document.getElementById('connection-status');
    
    if (indicator) {
      indicator.setAttribute('data-connection-status', isOnline ? 'Online' : 'Offline');
      indicator.className = isOnline ? 'offline-indicator online' : 'offline-indicator offline';
      indicator.querySelector('.status-text').textContent = isOnline ? 'Online' : 'Offline';
    }
  }

  /**
   * Setup API interceptors for offline functionality
   */
  setupApiInterceptors() {
    // Override fetch for offline functionality
    const originalFetch = window.fetch;
    
    window.fetch = async (url, options = {}) => {
      // Only intercept API calls
      if (typeof url === 'string' && url.includes('/api/')) {
        if (this.syncManager) {
          return this.syncManager.fetchWithFallback(url, options);
        }
      }
      
      return originalFetch(url, options);
    };
  }

  /**
   * Setup periodic cache maintenance
   */
  setupCacheMaintenance() {
    // Clean up expired cache every 30 minutes
    setInterval(async () => {
      if (window.securityScannerStorage) {
        await window.securityScannerStorage.clearExpiredCache();
        console.log('[SorobanSecurityScannerOffline] Cache maintenance completed');
      }
    }, 30 * 60 * 1000);
  }

  /**
   * Show update notification for service worker
   */
  showUpdateNotification() {
    const notification = document.createElement('div');
    notification.id = 'update-notification';
    notification.innerHTML = `
      <style>
        #update-notification {
          position: fixed;
          bottom: 20px;
          left: 20px;
          background: #2563eb;
          color: white;
          padding: 1rem 1.5rem;
          border-radius: 8px;
          box-shadow: 0 4px 12px rgba(0,0,0,0.15);
          z-index: 1002;
          max-width: 350px;
          display: flex;
          align-items: center;
          gap: 1rem;
        }
        #update-notification button {
          background: white;
          color: #2563eb;
          border: none;
          padding: 0.5rem 1rem;
          border-radius: 4px;
          cursor: pointer;
          font-weight: 600;
        }
      </style>
      <span>🔄 A new version is available!</span>
      <button onclick="window.location.reload()">Update Now</button>
      <button onclick="this.parentElement.remove()">Later</button>
    `;
    
    document.body.appendChild(notification);
    
    // Auto-hide after 15 seconds
    setTimeout(() => {
      if (notification.parentElement) {
        notification.remove();
      }
    }, 15000);
  }

  /**
   * Cache critical data for offline use
   */
  async cacheCriticalData() {
    if (!this.syncManager) return;
    
    try {
      console.log('[SorobanSecurityScannerOffline] Caching critical data...');
      
      // Cache security rules
      const rulesResponse = await fetch('/api/rules');
      if (rulesResponse.ok) {
        const rules = await rulesResponse.json();
        await window.securityScannerStorage.storeSecurityRules(rules.data);
        console.log('[SorobanSecurityScannerOffline] Security rules cached');
      }
      
      console.log('[SorobanSecurityScannerOffline] Critical data cached successfully');
      
    } catch (error) {
      console.warn('[SorobanSecurityScannerOffline] Failed to cache critical data:', error);
    }
  }

  /**
   * Get offline status information
   */
  getOfflineStatus() {
    return {
      isOnline: this.isOnline,
      serviceWorkerSupported: this.serviceWorkerSupported,
      indexedDBSupported: this.indexedDBSupported,
      syncStatus: this.syncManager ? this.syncManager.getSyncStatus() : null
    };
  }

  /**
   * Clear all offline data
   */
  async clearOfflineData() {
    if (this.syncManager && this.syncManager.clearAllData) {
      await this.syncManager.clearAllData();
      console.log('[SorobanSecurityScannerOffline] Cleared all offline data');
    }
    
    // Clear service worker caches
    if ('caches' in window) {
      const cacheNames = await caches.keys();
      await Promise.all(cacheNames.map(name => caches.delete(name)));
      console.log('[SorobanSecurityScannerOffline] Cleared service worker caches');
    }
  }

  /**
   * Force sync now
   */
  async forceSync() {
    if (this.syncManager && this.isOnline) {
      return await this.syncManager.forceSync();
    }
    return false;
  }

  /**
   * Get storage statistics
   */
  async getStorageStats() {
    if (window.securityScannerStorage) {
      return await window.securityScannerStorage.getStorageStats();
    }
    return null;
  }
}

// Auto-initialize when DOM is loaded
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    window.sorobanSecurityScannerOffline = new SorobanSecurityScannerOffline();
  });
} else {
  window.sorobanSecurityScannerOffline = new SorobanSecurityScannerOffline();
}

// Export for use in other scripts
window.SorobanSecurityScannerOffline = SorobanSecurityScannerOffline;
