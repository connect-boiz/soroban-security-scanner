# Modal & Dialog Components Guide

## Overview
Standardized modal system with proper focus management, accessibility, and responsive behavior.

## Components

### Modal
- **Focus Management**: Automatic focus trapping and restoration
- **Keyboard Navigation**: Escape key closes, Tab cycles through elements
- **Accessibility**: Proper ARIA attributes and semantic markup
- **Responsive**: Mobile-friendly sizing and behavior

### Dialog
- **Built-in Actions**: Confirm/cancel buttons with loading states
- **Variants**: Default, danger, warning, info styles
- **Async Support**: Handles async confirm actions

## Usage Examples

### Basic Modal
```tsx
import { Modal } from '@soroban-scanner/ui-components';

const [isOpen, setIsOpen] = useState(false);

<Modal
  isOpen={isOpen}
  onClose={() => setIsOpen(false)}
  title="Modal Title"
  size="md"
>
  <p>Modal content goes here</p>
</Modal>
```

### Confirmation Dialog
```tsx
import { Dialog } from '@soroban-scanner/ui-components';

<Dialog
  isOpen={showDialog}
  onClose={() => setShowDialog(false)}
  title="Confirm Action"
  message="Are you sure you want to proceed?"
  onConfirm={handleConfirm}
  variant="danger"
  confirmText="Delete"
  cancelText="Cancel"
/>
```

## Props

### Modal Props
- `isOpen`: boolean - Controls modal visibility
- `onClose`: function - Called when modal closes
- `title`: string - Modal header title
- `size`: 'sm' | 'md' | 'lg' | 'xl' | 'full' - Modal size
- `closeOnBackdrop`: boolean - Click backdrop to close
- `closeOnEscape`: boolean - Escape key closes modal
- `showCloseButton`: boolean - Show X button in header

### Dialog Props
- Extends all Modal props
- `message`: string - Dialog message content
- `confirmText`: string - Confirm button text
- `cancelText`: string - Cancel button text
- `onConfirm`: function - Confirm action handler
- `onCancel`: function - Cancel action handler
- `variant`: 'default' | 'danger' | 'warning' | 'info'
- `isConfirmLoading`: boolean - Show loading state
- `isConfirmDisabled`: boolean - Disable confirm button

## Migration

Replace existing inline modals:
```tsx
// Old
{selectedToken && (
  <div className="fixed inset-0 bg-black bg-opacity-50...">
    <div className="bg-white rounded-lg...">
      {/* content */}
    </div>
  </div>
)}

// New
<Modal isOpen={!!selectedToken} onClose={() => setSelectedToken(null)}>
  {/* content */}
</Modal>
```

## Accessibility Features
- Focus trapping within modal
- Automatic focus restoration on close
- Proper ARIA attributes
- Keyboard navigation support
- Screen reader compatibility
