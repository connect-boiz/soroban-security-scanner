import { useState, useCallback, DragEvent } from 'react';

export function useDragDrop<T>(initialItems: T[]) {
  const [items, setItems] = useState<T[]>(initialItems);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);

  const onDragStart = useCallback((index: number) => {
    setDragIndex(index);
  }, []);

  const onDragOver = useCallback((e: DragEvent, index: number) => {
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
  }, [dragIndex]);

  const onDragEnd = useCallback(() => {
    setDragIndex(null);
    setIsDragOver(false);
  }, []);

  const onFileDrop = useCallback((e: DragEvent, onFiles: (files: File[]) => void) => {
    e.preventDefault();
    setIsDragOver(false);
    const files = Array.from(e.dataTransfer.files);
    if (files.length) onFiles(files);
  }, []);

  return { items, setItems, dragIndex, isDragOver, onDragStart, onDragOver, onDragEnd, onFileDrop };
}
