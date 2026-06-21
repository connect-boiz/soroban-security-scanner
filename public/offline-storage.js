/**
 * Offline Storage for Soroban Security Scanner
 * Provides IndexedDB storage for scan results, rules, and queued scans
 */

class SecurityScannerStorage {
  constructor() {
    this.dbName = 'SorobanSecurityScanner';
    this.dbVersion = 1;
    this.db = null;
    this.isInitialized = false;
  }

  /**
   * Initialize IndexedDB database
   */
  async init() {
    if (this.isInitialized) return;
    
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.dbName, this.dbVersion);
      
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        this.db = request.result;
        this.isInitialized = true;
        resolve(this.db);
      };
      
      request.onupgradeneeded = (event) => {
        const db = event.target.result;
        
        // Create object stores for different data types
        if (!db.objectStoreNames.contains('scan-results')) {
          const scanStore = db.createObjectStore('scan-results', { keyPath: 'scanId' });
          scanStore.createIndex('contractId', 'contractId', { unique: false });
          scanStore.createIndex('timestamp', 'timestamp', { unique: false });
          scanStore.createIndex('severity', 'severity', { unique: false });
        }
        
        if (!db.objectStoreNames.contains('security-rules')) {
          const rulesStore = db.createObjectStore('security-rules', { keyPath: 'id' });
          rulesStore.createIndex('category', 'category', { unique: false });
          rulesStore.createIndex('severity', 'severity', { unique: false });
        }
        
        if (!db.objectStoreNames.contains('api-cache')) {
          const cacheStore = db.createObjectStore('api-cache', { keyPath: 'url' });
          cacheStore.createIndex('timestamp', 'timestamp', { unique: false });
        }
        
        if (!db.objectStoreNames.contains('scan-queue')) {
          const queueStore = db.createObjectStore('scan-queue', { keyPath: 'id', autoIncrement: true });
          queueStore.createIndex('timestamp', 'timestamp', { unique: false });
          queueStore.createIndex('status', 'status', { unique: false });
        }
        
        if (!db.objectStoreNames.contains('favorites')) {
          const favStore = db.createObjectStore('favorites', { keyPath: 'contractId' });
          favStore.createIndex('timestamp', 'timestamp', { unique: false });
        }
      };
    });
  }

  /**
   * Store scan result
   */
  async storeScanResult(scanResult) {
    await this.init();
    const transaction = this.db.transaction(['scan-results'], 'readwrite');
    const store = transaction.objectStore('scan-results');
    
    // Add offline metadata
    const offlineResult = {
      ...scanResult,
      cachedAt: Date.now(),
      offline: true
    };
    
    return store.put(offlineResult);
  }

  /**
   * Get scan result by ID
   */
  async getScanResult(scanId) {
    await this.init();
    const transaction = this.db.transaction(['scan-results'], 'readonly');
    const store = transaction.objectStore('scan-results');
    
    return new Promise((resolve, reject) => {
      const request = store.get(scanId);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Get all scan results with pagination
   */
  async getAllScanResults(options = {}) {
    await this.init();
    const { page = 1, limit = 20, contractId, severity } = options;
    
    const transaction = this.db.transaction(['scan-results'], 'readonly');
    const store = transaction.objectStore('scan-results');
    
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        let results = request.result;
        
        // Apply filters
        if (contractId) {
          results = results.filter(result => result.contractId.includes(contractId));
        }
        
        if (severity) {
          results = results.filter(result => 
            result.vulnerabilities.some(vuln => vuln.severity === severity)
          );
        }
        
        // Sort by timestamp (newest first)
        results.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
        
        // Apply pagination
        const startIndex = (page - 1) * limit;
        const endIndex = startIndex + limit;
        const paginatedResults = results.slice(startIndex, endIndex);
        
        resolve({
          scans: paginatedResults,
          page,
          limit,
          total: results.length,
        });
      };
      
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Store security rules
   */
  async storeSecurityRules(rules) {
    await this.init();
    const transaction = this.db.transaction(['security-rules'], 'readwrite');
    const store = transaction.objectStore('security-rules');
    
    const offlineRules = rules.map(rule => ({
      ...rule,
      cachedAt: Date.now(),
      offline: true
    }));
    
    // Store all rules
    for (const rule of offlineRules) {
      await store.put(rule);
    }
  }

  /**
   * Get security rules
   */
  async getSecurityRules(filters = {}) {
    await this.init();
    const { category, severity } = filters;
    
    const transaction = this.db.transaction(['security-rules'], 'readonly');
    const store = transaction.objectStore('security-rules');
    
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        let rules = request.result;
        
        // Apply filters
        if (category) {
          rules = rules.filter(rule => rule.category === category);
        }
        
        if (severity) {
          rules = rules.filter(rule => rule.severity === severity);
        }
        
        resolve(rules);
      };
      
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Cache API response
   */
  async cacheApiResponse(url, response, ttl = 600000) { // 10 minutes default TTL
    await this.init();
    const transaction = this.db.transaction(['api-cache'], 'readwrite');
    const store = transaction.objectStore('api-cache');
    
    const cacheEntry = {
      url,
      response: JSON.stringify(response),
      timestamp: Date.now(),
      ttl,
      expiresAt: Date.now() + ttl
    };
    
    return store.put(cacheEntry);
  }

  /**
   * Get cached API response
   */
  async getCachedApiResponse(url) {
    await this.init();
    const transaction = this.db.transaction(['api-cache'], 'readonly');
    const store = transaction.objectStore('api-cache');
    
    return new Promise((resolve, reject) => {
      const request = store.get(url);
      request.onsuccess = () => {
        const cached = request.result;
        if (cached && cached.expiresAt > Date.now()) {
          resolve(JSON.parse(cached.response));
        } else {
          resolve(null);
        }
      };
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Add scan to queue
   */
  async queueScan(contractId, options = {}) {
    await this.init();
    const transaction = this.db.transaction(['scan-queue'], 'readwrite');
    const store = transaction.objectStore('scan-queue');
    
    const queuedScan = {
      contractId,
      options,
      timestamp: Date.now(),
      status: 'queued',
      attempts: 0
    };
    
    return store.add(queuedScan);
  }

  /**
   * Get queued scans
   */
  async getQueuedScans() {
    await this.init();
    const transaction = this.db.transaction(['scan-queue'], 'readonly');
    const store = transaction.objectStore('scan-queue');
    
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        const queued = request.result.filter(item => item.status === 'queued');
        resolve(queued);
      };
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Clear expired cache entries
   */
  async clearExpiredCache() {
    await this.init();
    const transaction = this.db.transaction(['api-cache'], 'readwrite');
    const store = transaction.objectStore('api-cache');
    const now = Date.now();
    
    return new Promise((resolve, reject) => {
      const request = store.openCursor();
      request.onsuccess = (event) => {
        const cursor = event.target.result;
        if (cursor) {
          if (cursor.value.expiresAt < now) {
            cursor.delete();
          }
          cursor.continue();
        } else {
          resolve();
        }
      };
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Get storage statistics
   */
  async getStorageStats() {
    await this.init();
    const stats = {};
    
    const stores = ['scan-results', 'security-rules', 'api-cache', 'scan-queue', 'favorites'];
    
    for (const storeName of stores) {
      const transaction = this.db.transaction([storeName], 'readonly');
      const store = transaction.objectStore(storeName);
      
      const count = await new Promise((resolve) => {
        const request = store.count();
        request.onsuccess = () => resolve(request.result);
      });
      
      stats[storeName] = count;
    }
    
    return stats;
  }

  /**
   * Clear all offline data
   */
  async clearAll() {
    await this.init();
    const stores = ['scan-results', 'security-rules', 'api-cache', 'scan-queue', 'favorites'];
    
    for (const storeName of stores) {
      const transaction = this.db.transaction([storeName], 'readwrite');
      const store = transaction.objectStore(storeName);
      store.clear();
    }
  }
}

// Export for use in browser
window.SecurityScannerStorage = SecurityScannerStorage;
