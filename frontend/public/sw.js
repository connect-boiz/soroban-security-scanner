const CACHE_NAME = 'soroban-scanner-v1';
const STATIC_CACHE = 'static-v1';
const DYNAMIC_CACHE = 'dynamic-v1';

const STATIC_ASSETS = [
  '/',
  '/app/page',
  '/app/layout',
  '/app/globals.css',
  '/scanner-icon.svg',
];

// Install event - cache static assets
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE)
      .then((cache) => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting())
  );
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((cacheNames) => {
        return Promise.all(
          cacheNames
            .filter((cacheName) => 
              cacheName !== STATIC_CACHE && cacheName !== DYNAMIC_CACHE
            )
            .map((cacheName) => caches.delete(cacheName))
        );
      })
      .then(() => self.clients.claim())
  );
});

// Fetch event - serve from cache with network fallback
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests and external URLs
  if (request.method !== 'GET' || url.origin !== self.location.origin) {
    return;
  }

  // Cache-first strategy for static assets
  if (STATIC_ASSETS.some(asset => url.pathname.includes(asset)) || 
      url.pathname.includes('/_next/static/') ||
      url.pathname.includes('/images/')) {
    event.respondWith(
      caches.match(request)
        .then((response) => {
          if (response) {
            return response;
          }
          
          return fetch(request)
            .then((response) => {
              if (!response || response.status !== 200) {
                return response;
              }

              const responseClone = response.clone();
              caches.open(STATIC_CACHE)
                .then((cache) => cache.put(request, responseClone));
              
              return response;
            });
        })
    );
    return;
  }

  // Network-first strategy for dynamic content
  event.respondWith(
    fetch(request)
      .then((response) => {
        if (!response || response.status !== 200) {
          return response;
        }

        const responseClone = response.clone();
        caches.open(DYNAMIC_CACHE)
          .then((cache) => cache.put(request, responseClone));
        
        return response;
      })
      .catch(() => {
        return caches.match(request);
      })
  );
});

// Background sync for failed requests
self.addEventListener('sync', (event) => {
  if (event.tag === 'background-sync') {
    event.waitUntil(
      // Handle background sync logic here
      console.log('Background sync triggered')
    );
  }
});

// Push notifications
self.addEventListener('push', (event) => {
  const options = {
    body: event.data ? event.data.text() : 'New scan completed',
    icon: '/scanner-icon.svg',
    badge: '/scanner-icon.svg',
    vibrate: [100, 50, 100],
    data: {
      dateOfArrival: Date.now(),
      primaryKey: 1
    }
  };

  event.waitUntil(
    self.registration.showNotification('Soroban Scanner', options)
  );
});
