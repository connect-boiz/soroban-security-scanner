interface CacheEntry<T> {
  data: T;
  expiresAt: number;
}

const memoryCache = new Map<string, CacheEntry<unknown>>();

function set<T>(key: string, data: T, ttlMs = 5 * 60 * 1000): void {
  memoryCache.set(key, { data, expiresAt: Date.now() + ttlMs });
}

function get<T>(key: string): T | null {
  const entry = memoryCache.get(key) as CacheEntry<T> | undefined;
  if (!entry) return null;
  if (Date.now() > entry.expiresAt) {
    memoryCache.delete(key);
    return null;
  }
  return entry.data;
}

function remove(key: string): void {
  memoryCache.delete(key);
}

function clear(): void {
  memoryCache.clear();
}

// Persistent cache via localStorage
function persist<T>(key: string, data: T, ttlMs = 60 * 60 * 1000): void {
  try {
    localStorage.setItem(key, JSON.stringify({ data, expiresAt: Date.now() + ttlMs }));
  } catch {
    // storage quota exceeded — fail silently
  }
}

function load<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const entry: CacheEntry<T> = JSON.parse(raw);
    if (Date.now() > entry.expiresAt) {
      localStorage.removeItem(key);
      return null;
    }
    return entry.data;
  } catch {
    return null;
  }
}

async function fetchWithCache<T>(
  key: string,
  fetcher: () => Promise<T>,
  ttlMs = 5 * 60 * 1000
): Promise<T> {
  const cached = get<T>(key);
  if (cached !== null) return cached;
  const data = await fetcher();
  set(key, data, ttlMs);
  return data;
}

export const cache = { set, get, remove, clear, persist, load, fetchWithCache };
