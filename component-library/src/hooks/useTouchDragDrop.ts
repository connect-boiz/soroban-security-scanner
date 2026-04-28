import { useState, useCallback, useRef, TouchEvent } from 'react';

interface TouchDragDropState<T> {
  items: T[];
  dragIndex: number | null;
  isDragOver: boolean;
}

interface TouchDragDropHandlers {
  /** Attach to the drag handle / list item for mouse drag */
  onDragStart: (index: number) => void;
  onDragOver: (e: React.DragEvent, index: number) => void;
  onDragEnd: () => void;
  /** Attach to the drop zone for file drops */
  onFileDrop: (e: React.DragEvent, onFiles: (files: File[]) => void) => void;
  /** Touch equivalents — attach to each list item */
  onTouchStart: (index: number) => (e: TouchEvent) => void;
  onTouchMove: (e: TouchEvent) => void;
  onTouchEnd: () => void;
}

/**
 * Unified drag-and-drop hook that supports both mouse (HTML5 DnD API)
 * and touch events, making lists reorderable on mobile devices.
 */
export function useTouchDragDrop<T>(initialItems: T[]): TouchDragDropState<T> & TouchDragDropHandlers & { setItems: React.Dispatch<React.SetStateAction<T[]>> } {
  const [items, setItems] = useState<T[]>(initialItems);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);

  // Touch tracking refs — avoids re-renders during active drag
  const touchDragIndex = useRef<number | null>(null);
  const touchStartY = useRef<number>(0);
  const itemHeightRef = useRef<number>(0);

  // ── Mouse / HTML5 DnD ──────────────────────────────────────────────────────

  const onDragStart = useCallback((index: number) => {
    setDragIndex(index);
  }, []);

  const onDragOver = useCallback(
    (e: React.DragEvent, index: number) => {
      e.preventDefault();
      setIsDragOver(true);
      if (dragIndex === null || dragIndex === index) return;
      setItems((prev) => {
        const next = [...prev];
        const [moved] = next.splice(dragIndex, 1);
        next.splice(index, 0, moved);
        return next;
      });
      setDragIndex(index);
    },
    [dragIndex],
  );

  const onDragEnd = useCallback(() => {
    setDragIndex(null);
    setIsDragOver(false);
  }, []);

  const onFileDrop = useCallback((e: React.DragEvent, onFiles: (files: File[]) => void) => {
    e.preventDefault();
    setIsDragOver(false);
    const files = Array.from(e.dataTransfer.files);
    if (files.length) onFiles(files);
  }, []);

  // ── Touch ──────────────────────────────────────────────────────────────────

  const onTouchStart = useCallback(
    (index: number) => (e: TouchEvent) => {
      touchDragIndex.current = index;
      touchStartY.current = e.touches[0].clientY;
      // Estimate item height from the touched element
      const el = e.currentTarget as HTMLElement;
      itemHeightRef.current = el.getBoundingClientRect().height || 48;
      setDragIndex(index);
    },
    [],
  );

  const onTouchMove = useCallback((e: TouchEvent) => {
    e.preventDefault(); // prevent page scroll while dragging
    if (touchDragIndex.current === null) return;

    const deltaY = e.touches[0].clientY - touchStartY.current;
    const steps = Math.round(deltaY / itemHeightRef.current);
    if (steps === 0) return;

    const from = touchDragIndex.current;
    const to = Math.max(0, from + steps);

    setItems((prev) => {
      if (to >= prev.length) return prev;
      const next = [...prev];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      return next;
    });

    touchDragIndex.current = to;
    touchStartY.current = e.touches[0].clientY;
    setDragIndex(to);
  }, []);

  const onTouchEnd = useCallback(() => {
    touchDragIndex.current = null;
    setDragIndex(null);
    setIsDragOver(false);
  }, []);

  return {
    items,
    setItems,
    dragIndex,
    isDragOver,
    onDragStart,
    onDragOver,
    onDragEnd,
    onFileDrop,
    onTouchStart,
    onTouchMove,
    onTouchEnd,
  };
}
