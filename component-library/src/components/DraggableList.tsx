import { CSSProperties, ReactNode } from 'react';
import { useTouchDragDrop } from '../hooks/useTouchDragDrop';
import { useIsTouchDevice } from '../hooks/useMediaQuery';
import { MIN_TOUCH_TARGET, spacing } from '../utils/breakpoints';

export interface DraggableListProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  onReorder?: (items: T[]) => void;
  onFileDrop?: (files: File[]) => void;
  className?: string;
  /** Override the minimum item height (px). Defaults to MIN_TOUCH_TARGET on touch devices. */
  itemMinHeight?: number;
}

export function DraggableList<T>({
  items: initialItems,
  renderItem,
  onReorder,
  onFileDrop,
  className,
  itemMinHeight,
}: DraggableListProps<T>) {
  const isTouch = useIsTouchDevice();

  const {
    items,
    dragIndex,
    isDragOver,
    onDragStart,
    onDragOver,
    onDragEnd,
    onFileDrop: handleFileDrop,
    onTouchStart,
    onTouchMove,
    onTouchEnd,
  } = useTouchDragDrop(initialItems);

  const minHeight = itemMinHeight ?? (isTouch ? MIN_TOUCH_TARGET : 0);

  const listStyle: CSSProperties = {
    listStyle: 'none',
    padding: 0,
    margin: 0,
    border: isDragOver ? '2px dashed #888' : '2px solid transparent',
    borderRadius: 8,
    transition: 'border-color 0.2s',
    // Prevent text selection during drag on mobile
    WebkitUserSelect: 'none',
    userSelect: 'none',
  };

  const getItemStyle = (index: number): CSSProperties => ({
    cursor: isTouch ? 'grab' : 'grab',
    minHeight,
    padding: `${spacing.sm}px ${spacing.md}px`,
    display: 'flex',
    alignItems: 'center',
    borderRadius: 6,
    marginBottom: spacing.xs,
    background: dragIndex === index ? 'rgba(0,0,0,0.06)' : 'transparent',
    transition: 'background 0.15s, transform 0.15s',
    transform: dragIndex === index ? 'scale(1.02)' : 'scale(1)',
    // Ensure touch targets are large enough
    boxSizing: 'border-box',
    touchAction: 'none', // let our handlers manage touch, not the browser
  });

  return (
    <ul
      className={className}
      style={listStyle}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onFileDrop ? (e) => handleFileDrop(e, onFileDrop) : undefined}
      // Propagate touch move at the list level so we capture moves outside the item
      onTouchMove={isTouch ? onTouchMove : undefined}
      onTouchEnd={isTouch ? onTouchEnd : undefined}
      aria-label="Reorderable list"
    >
      {items.map((item, index) => (
        <li
          key={index}
          draggable={!isTouch}
          style={getItemStyle(index)}
          onDragStart={!isTouch ? () => onDragStart(index) : undefined}
          onDragOver={
            !isTouch
              ? (e) => {
                  onDragOver(e, index);
                  onReorder?.(items);
                }
              : undefined
          }
          onDragEnd={!isTouch ? onDragEnd : undefined}
          onTouchStart={isTouch ? onTouchStart(index) : undefined}
          aria-grabbed={dragIndex === index}
          role="listitem"
        >
          {/* Drag handle — visible on touch, subtle on desktop */}
          <span
            aria-hidden="true"
            style={{
              marginRight: spacing.sm,
              opacity: 0.4,
              fontSize: isTouch ? 20 : 14,
              lineHeight: 1,
              flexShrink: 0,
              minWidth: isTouch ? MIN_TOUCH_TARGET : 16,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            ⠿
          </span>
          <span style={{ flex: 1 }}>{renderItem(item, index)}</span>
        </li>
      ))}
    </ul>
  );
}

export default DraggableList;
