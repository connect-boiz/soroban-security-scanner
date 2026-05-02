/**
 * Service Worker for Soroban Security Scanner
 * Provides offline functionality for contract scanning and security analysis
 */

const CACHE_NAME = 'soroban-scanner-v1';
const STATIC_CACHE_NAME = 'soroban-static-v1';

// Critical API endpoints to cache
const CRITICAL_API_PATTERNS = [
  '/api/scan/',
  '/api/history',
  '/api/rules',
  '/health'
];

// Static assets to cache for offline functionality
const STATIC_ASSETS = [
  '/',
  '/offline.html',
  '/manifest.json',
  '/styles/main.css',
  '/scripts/main.js'
];

// Cache TTL settings (in milliseconds)
const CACHE_TTL = {
  scanResults: 30 * 60 * 1000,    // 30 minutes for scan results
  history: 15 * 60 * 1000,        // 15 minutes for scan history
  rules: 60 * 60 * 1000,          // 1 hour for security rules
  health: 5 * 60 * 1000,          // 5 minutes for health check
  default: 10 * 60 * 1000         // 10 minutes default
};

/**
 * Install event - cache static assets
 */
self.addEventListener('install', (event) => {
  console.log('[SW] Installing Soroban Security Scanner service worker');
  
  event.waitUntil(
    caches.open(STATIC_CACHE_NAME)
      .then((cache) => {
        console.log('[SW] Caching static assets');
        return cache.addAll(STATIC_ASSETS);
      })
      .then(() => {
        console.log('[SW] Static assets cached successfully');
        return self.skipWaiting();
      })
  );
});

/**
 * Activate event - clean up old caches
 */
self.addEventListener('activate', (event) => {
  console.log('[SW] Activating service worker');
  
  event.waitUntil(
    caches.keys()
      .then((cacheNames) => {
        return Promise.all(
          cacheNames.map((cacheName) => {
            if (cacheName !== CACHE_NAME && cacheName !== STATIC_CACHE_NAME) {
              console.log('[SW] Deleting old cache:', cacheName);
              return caches.delete(cacheName);
            }
          })
        );
      })
      .then(() => {
        console.log('[SW] Service worker activated');
        return self.clients.claim();
      })
  );
});

/**
 * Fetch event - handle requests with caching strategy
 */
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);
  
  // Skip non-HTTP requests
  if (!request.url.startsWith('http')) {
    return;
  }
  
  // Handle API requests
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(handleApiRequest(request));
    return;
  }
  
  // Handle static requests
  if (STATIC_ASSETS.includes(url.pathname) || url.pathname === '/') {
    event.respondWith(handleStaticRequest(request));
    return;
  }
});

/**
 * Handle API requests with network-first strategy
 */
async function handleApiRequest(request) {
  const url = new URL(request.url);
  const cacheKey = `${request.method}:${request.url}`;
  
  try {
    // Try network first
    const networkResponse = await fetch(request);
    
    if (networkResponse.ok) {
      // Cache successful responses
      const cache = await caches.open(CACHE_NAME);
      const responseClone = networkResponse.clone();
      
      // Add cache metadata
      const cacheEntry = {
        response: responseClone,
        timestamp: Date.now(),
        ttl: getTTLForEndpoint(url.pathname)
      };
      
      cache.put(cacheKey, cacheEntry);
      console.log('[SW] API response cached:', request.url);
    }
    
    return networkResponse;
  } catch (error) {
    console.log('[SW] Network failed, trying cache:', request.url);
    
    // Fallback to cache
    const cachedResponse = await getCachedResponse(cacheKey);
    if (cachedResponse) {
      return cachedResponse;
    }
    
    // Return offline fallback
    return createOfflineResponse(request);
  }
}

/**
 * Handle static requests with cache-first strategy
 */
async function handleStaticRequest(request) {
  const cache = await caches.open(STATIC_CACHE_NAME);
  const cachedResponse = await cache.match(request);
  
  if (cachedResponse) {
    return cachedResponse;
  }
  
  try {
    const networkResponse = await fetch(request);
    if (networkResponse.ok) {
      cache.put(request, networkResponse.clone());
    }
    return networkResponse;
  } catch (error) {
    console.log('[SW] Static request failed, serving offline page');
    return caches.match('/offline.html');
  }
}

/**
 * Get cached response if not expired
 */
async function getCachedResponse(cacheKey) {
  const cache = await caches.open(CACHE_NAME);
  const cached = await cache.match(cacheKey);
  
  if (!cached) {
    return null;
  }
  
  // Check if cache is expired
  const cacheData = await getCachedData(cached);
  if (cacheData && Date.now() - cacheData.timestamp < cacheData.ttl) {
    console.log('[SW] Serving from cache:', cacheKey);
    return cacheData.response;
  }
  
  // Cache expired, remove it
  cache.delete(cacheKey);
  return null;
}

/**
 * Extract cached data from Response object
 */
async function getCachedData(response) {
  try {
    const data = await response.json();
    return data;
  } catch (error) {
    return null;
  }
}

/**
 * Get TTL for specific API endpoint
 */
function getTTLForEndpoint(pathname) {
  if (pathname.includes('/scan')) return CACHE_TTL.scanResults;
  if (pathname.includes('/history')) return CACHE_TTL.history;
  if (pathname.includes('/rules')) return CACHE_TTL.rules;
  if (pathname.includes('/health')) return CACHE_TTL.health;
  return CACHE_TTL.default;
}

/**
 * Create offline fallback response
 */
function createOfflineResponse(request) {
  const url = new URL(request.url);
  
  if (request.method === 'GET') {
    // Return cached data or offline message
    if (url.pathname.includes('/scan')) {
      return new Response(JSON.stringify({
        error: 'offline',
        message: 'Currently offline. Showing cached scan results.',
        data: [],
        offline: true
      }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' }
      });
    }
    
    if (url.pathname.includes('/history')) {
      return new Response(JSON.stringify({
        error: 'offline',
        message: 'Currently offline. Showing cached scan history.',
        data: { scans: [], pagination: { page: 1, limit: 20, total: 0, pages: 0 } },
        offline: true
      }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' }
      });
    }
    
    if (url.pathname.includes('/rules')) {
      return new Response(JSON.stringify({
        error: 'offline',
        message: 'Currently offline. Showing cached security rules.',
        data: [],
        offline: true
      }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' }
      });
    }
    
    if (url.pathname.includes('/health')) {
      return new Response(JSON.stringify({
        status: 'offline',
        message: 'Security scanner is currently offline',
        offline: true
      }), {
        status: 503,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  }
  
  // Default offline response
  return new Response(JSON.stringify({
    error: 'offline',
    message: 'Currently offline. Please check your connection.',
    offline: true
  }), {
    status: 503,
    headers: { 'Content-Type': 'application/json' }
  });
}

/**
 * Message event - handle communication from main thread
 */
self.addEventListener('message', (event) => {
  const { type, payload } = event.data;
  
  switch (type) {
    case 'SKIP_WAITING':
      self.skipWaiting();
      break;
      
    case 'CACHE_CLEAR':
      clearCache();
      break;
      
    case 'CACHE_STATUS':
      getCacheStatus().then(status => {
        event.ports[0].postMessage(status);
      });
      break;
      
    case 'SYNC_SCAN':
      handleQueuedScan(payload);
      break;
      
    default:
      console.log('[SW] Unknown message type:', type);
  }
});

/**
 * Handle queued scan when online
 */
async function handleQueuedScan(scanData) {
  try {
    const response = await fetch('/api/scan', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(scanData)
    });
    
    if (response.ok) {
      const result = await response.json();
      console.log('[SW] Queued scan completed:', result.data.scanId);
      return result;
    }
  } catch (error) {
    console.error('[SW] Failed to process queued scan:', error);
  }
}

/**
 * Clear all caches
 */
async function clearCache() {
  const cacheNames = await caches.keys();
  await Promise.all(cacheNames.map(name => caches.delete(name)));
  console.log('[SW] All caches cleared');
}

/**
 * Get cache status information
 */
async function getCacheStatus() {
  const cacheNames = await caches.keys();
  const status = {};
  
  for (const name of cacheNames) {
    const cache = await caches.open(name);
    const keys = await cache.keys();
    status[name] = keys.length;
  }
  
  return status;
}
