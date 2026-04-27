import { ReactNode } from 'react';
import { useDragDrop } from '../hooks/useDragDrop';

interface DraggableListProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  onReorder?: (items: T[]) => void;
  onFileDrop?: (files: File[]) => void;
  className?: string;
}

export function DraggableList<T>({
  items: initialItems,
  renderItem,
  onReorder,
  onFileDrop,
  className,
}: DraggableListProps<T>) {
  const { items, isDragOver, onDragStart, onDragOver, onDragEnd, onFileDrop: handleFileDrop } =
    useDragDrop(initialItems);

  return (
    <ul
      className={className}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onFileDrop ? (e) => handleFileDrop(e, onFileDrop) : undefined}
      style={{ listStyle: 'none', padding: 0, border: isDragOver ? '2px dashed #888' : undefined }}
    >
      {items.map((item, index) => (
        <li
          key={index}
          draggable
          onDragStart={() => onDragStart(index)}
          onDragOver={(e) => {
            onDragOver(e, index);
            onReorder?.(items);
          }}
          onDragEnd={onDragEnd}
          style={{ cursor: 'grab' }}
        >
          {renderItem(item, index)}
        </li>
      ))}
    </ul>
  );
}

export default DraggableList;
