import React, { useState } from 'react';
import { DraggableList, LazyImage, useWebSocket, cache } from '../src';

// Example usage of DraggableList
function DraggableListExample() {
  const [items, setItems] = useState(['Item 1', 'Item 2', 'Item 3']);

  return (
    <DraggableList
      items={items}
      renderItem={(item, index) => (
        <div style={{ padding: '10px', border: '1px solid #ccc', margin: '5px' }}>
          {item}
        </div>
      )}
      onReorder={setItems}
      onFileDrop={(files) => console.log('Dropped files:', files)}
    />
  );
}

// Example usage of LazyImage
function LazyImageExample() {
  return (
    <LazyImage
      src="/example.jpg"
      webpSrc="/example.webp"
      alt="Example image"
      width={300}
      height={200}
    />
  );
}

// Example usage of useWebSocket
function WebSocketExample() {
  const { status, send } = useWebSocket('ws://localhost:8080', {
    onMessage: (data) => console.log('Received:', data),
    onOpen: () => console.log('Connected'),
    onClose: () => console.log('Disconnected'),
  });

  return (
    <div>
      <p>Status: {status}</p>
      <button onClick={() => send({ type: 'ping' })}>Send Ping</button>
    </div>
  );
}

// Example usage of cache
async function CacheExample() {
  // Set cache
  cache.set('user', { name: 'John', age: 30 }, 60000);
  
  // Get from cache
  const user = cache.get('user');
  console.log(user); // { name: 'John', age: 30 }
  
  // Persistent cache
  cache.persist('settings', { theme: 'dark' }, 3600000);
  const settings = cache.load('settings');
  console.log(settings); // { theme: 'dark' }
  
  // Fetch with cache
  const data = await cache.fetchWithCache(
    'api-data',
    () => fetch('/api/data').then(r => r.json()),
    300000
  );
  console.log(data);
}

export { DraggableListExample, LazyImageExample, WebSocketExample, CacheExample };
