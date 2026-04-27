# @soroban-scanner/ui-components

Reusable UI components and utilities for the Soroban Security Scanner project.

## Installation

```bash
npm install @soroban-scanner/ui-components
```

## Components

### DraggableList

A generic draggable list component with file drop support.

```tsx
import { DraggableList } from '@soroban-scanner/ui-components';

function MyComponent() {
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
```

#### Props

- `items: T[]` - Array of items to render
- `renderItem: (item: T, index: number) => ReactNode` - Function to render each item
- `onReorder?: (items: T[]) => void` - Callback when items are reordered
- `onFileDrop?: (files: File[]) => void` - Callback when files are dropped
- `className?: string` - Additional CSS classes

### LazyImage

An optimized image component with lazy loading and WebP support.

```tsx
import { LazyImage } from '@soroban-scanner/ui-components';

function MyComponent() {
  return (
    <LazyImage
      src="/path/to/image.jpg"
      webpSrc="/path/to/image.webp"
      alt="Description"
      width={300}
      height={200}
      className="my-image"
    />
  );
}
```

#### Props

- `src: string` - Image source URL
- `webpSrc?: string` - WebP image source URL (optional)
- `alt: string` - Alt text for accessibility
- `width?: number` - Image width
- `height?: number` - Image height
- `sizes?: string` - Image sizes attribute (default: '100vw')
- `className?: string` - Additional CSS classes

## Hooks

### useDragDrop

Hook for drag and drop functionality.

```tsx
import { useDragDrop } from '@soroban-scanner/ui-components';

function MyComponent() {
  const { items, isDragOver, onDragStart, onDragOver, onDragEnd, onFileDrop } = 
    useDragDrop(['Item 1', 'Item 2']);

  return (
    <div
      onDragOver={onDragOver}
      onDrop={(e) => onFileDrop(e, (files) => console.log(files))}
      style={{ border: isDragOver ? '2px dashed blue' : 'none' }}
    >
      {items.map((item, index) => (
        <div
          key={index}
          draggable
          onDragStart={() => onDragStart(index)}
          onDragEnd={onDragEnd}
        >
          {item}
        </div>
      ))}
    </div>
  );
}
```

### useWebSocket

WebSocket hook with automatic reconnection logic.

```tsx
import { useWebSocket } from '@soroban-scanner/ui-components';

function MyComponent() {
  const { status, send } = useWebSocket('ws://localhost:8080', {
    onMessage: (data) => console.log('Received:', data),
    onOpen: () => console.log('Connected'),
    onClose: () => console.log('Disconnected'),
    reconnectDelay: 2000,
    maxRetries: 3,
  });

  return (
    <div>
      <p>Status: {status}</p>
      <button onClick={() => send({ type: 'ping' })}>
        Send Ping
      </button>
    </div>
  );
}
```

#### Options

- `onMessage?: (data: unknown) => void` - Message handler
- `onOpen?: () => void` - Connection open handler
- `onClose?: () => void` - Connection close handler
- `reconnectDelay?: number` - Reconnection delay in ms (default: 3000)
- `maxRetries?: number` - Maximum reconnection attempts (default: 5)

## Utils

### cache

Memory and persistent caching utilities.

```tsx
import { cache } from '@soroban-scanner/ui-components';

// Memory cache
cache.set('key', { data: 'value' }, 60000); // 1 minute TTL
const data = cache.get('key'); // Returns cached data or null
cache.remove('key');
cache.clear();

// Persistent cache (localStorage)
cache.persist('user', { name: 'John' }, 3600000); // 1 hour TTL
const user = cache.load('user'); // Returns cached data or null

// Fetch with cache
const result = await cache.fetchWithCache(
  'api-data',
  () => fetch('/api/data').then(r => r.json()),
  300000 // 5 minutes TTL
);
```

#### Methods

- `set<T>(key: string, data: T, ttlMs?: number): void` - Set memory cache
- `get<T>(key: string): T | null` - Get from memory cache
- `remove(key: string): void` - Remove from memory cache
- `clear(): void` - Clear memory cache
- `persist<T>(key: string, data: T, ttlMs?: number): void` - Set persistent cache
- `load<T>(key: string): T | null` - Get from persistent cache
- `fetchWithCache<T>(key: string, fetcher: () => Promise<T>, ttlMs?: number): Promise<T>` - Fetch with caching

## Development

```bash
# Install dependencies
npm install

# Build the library
npm run build

# Watch mode for development
npm run dev

# Run tests
npm test

# Type checking
npm run type-check

# Linting
npm run lint
npm run lint:fix
```

## License

MIT
