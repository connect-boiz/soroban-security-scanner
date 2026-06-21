import { useCallback, useRef, useState } from 'react';

interface UseVirtualScrollOptions {
  itemHeight: number;
  overscan?: number;
}

interface UseVirtualScrollResult<T> {
  containerRef: React.RefObject<HTMLDivElement>;
  visibleItems: { item: T; index: number }[];
  totalHeight: number;
  offsetY: number;
}

export function useVirtualScroll<T>(
  items: T[],
  { itemHeight, overscan = 3 }: UseVirtualScrollOptions
): UseVirtualScrollResult<T> {
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [containerHeight, setContainerHeight] = useState(0);

  const onScroll = useCallback(() => {
    if (containerRef.current) {
      setScrollTop(containerRef.current.scrollTop);
      setContainerHeight(containerRef.current.clientHeight);
    }
  }, []);

  // Attach scroll listener via ref callback
  const setRef = useCallback(
    (node: HTMLDivElement | null) => {
      if ((containerRef as React.MutableRefObject<HTMLDivElement | null>).current) {
        (containerRef as React.MutableRefObject<HTMLDivElement | null>).current!.removeEventListener(
          'scroll',
          onScroll
        );
      }
      (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
      if (node) {
        node.addEventListener('scroll', onScroll, { passive: true });
        setContainerHeight(node.clientHeight);
      }
    },
    [onScroll]
  );

  // Expose setRef as the ref object shape consumers expect
  (containerRef as unknown as { current: HTMLDivElement | null; setRef: typeof setRef }).setRef =
    setRef;

  const totalHeight = items.length * itemHeight;
  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const visibleCount = Math.ceil(containerHeight / itemHeight) + overscan * 2;
  const endIndex = Math.min(items.length, startIndex + visibleCount);

  const visibleItems = items.slice(startIndex, endIndex).map((item, i) => ({
    item,
    index: startIndex + i,
  }));

  const offsetY = startIndex * itemHeight;

  return { containerRef, visibleItems, totalHeight, offsetY };
}
