'use client';

import React, { useRef, useCallback } from 'react';
import Modal, { ModalProps } from './Modal';

export interface DialogProps extends Omit<ModalProps, 'size' | 'children'> {
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm?: () => void | Promise<void>;
  onCancel?: () => void;
  variant?: 'default' | 'danger' | 'warning' | 'info';
  confirmButtonClassName?: string;
  cancelButtonClassName?: string;
  isConfirmLoading?: boolean;
  isConfirmDisabled?: boolean;
}

const Dialog: React.FC<DialogProps> = ({
  title,
  message,
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  onConfirm,
  onCancel,
  variant = 'default',
  confirmButtonClassName = '',
  cancelButtonClassName = '',
  isConfirmLoading = false,
  isConfirmDisabled = false,
  ...modalProps
}) => {
  const confirmButtonRef = useRef<HTMLButtonElement>(null);

  const handleConfirm = useCallback(async () => {
    if (onConfirm && !isConfirmLoading && !isConfirmDisabled) {
      await onConfirm();
      if (!isConfirmLoading) {
        modalProps.onClose();
      }
    }
  }, [onConfirm, isConfirmLoading, isConfirmDisabled, modalProps.onClose]);

  const handleCancel = useCallback(() => {
    if (onCancel) {
      onCancel();
    }
    modalProps.onClose();
  }, [onCancel, modalProps.onClose]);

  // Variant styles
  const variantStyles: Record<string, { confirm: string; cancel: string }> = {
    default: {
      confirm: 'bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 text-white',
      cancel: 'bg-gray-200 hover:bg-gray-300 focus:ring-gray-500 text-gray-900'
    },
    danger: {
      confirm: 'bg-red-600 hover:bg-red-700 focus:ring-red-500 text-white',
      cancel: 'bg-gray-200 hover:bg-gray-300 focus:ring-gray-500 text-gray-900'
    },
    warning: {
      confirm: 'bg-yellow-600 hover:bg-yellow-700 focus:ring-yellow-500 text-white',
      cancel: 'bg-gray-200 hover:bg-gray-300 focus:ring-gray-500 text-gray-900'
    },
    info: {
      confirm: 'bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 text-white',
      cancel: 'bg-gray-200 hover:bg-gray-300 focus:ring-gray-500 text-gray-900'
    }
  };

  const styles = variantStyles[variant];

  return (
    <Modal
      {...modalProps}
      size="sm"
      initialFocusRef={confirmButtonRef}
      className="dialog-modal"
    >
      <div className="space-y-4">
        {/* Message */}
        <p className="text-gray-700 leading-relaxed">
          {message}
        </p>

        {/* Actions */}
        <div className="flex justify-end space-x-3 pt-4">
          <button
            type="button"
            ref={confirmButtonRef}
            onClick={handleConfirm}
            disabled={isConfirmDisabled || isConfirmLoading}
            className={`
              px-4 py-2 rounded-md font-medium transition-colors
              focus:outline-none focus:ring-2 focus:ring-offset-2
              disabled:opacity-50 disabled:cursor-not-allowed
              ${styles.confirm}
              ${confirmButtonClassName}
            `}
          >
            {isConfirmLoading ? (
              <span className="flex items-center space-x-2">
                <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
                <span>Loading...</span>
              </span>
            ) : (
              confirmText
            )}
          </button>

          <button
            type="button"
            onClick={handleCancel}
            disabled={isConfirmLoading}
            className={`
              px-4 py-2 rounded-md font-medium transition-colors
              focus:outline-none focus:ring-2 focus:ring-offset-2
              disabled:opacity-50 disabled:cursor-not-allowed
              ${styles.cancel}
              ${cancelButtonClassName}
            `}
          >
            {cancelText}
          </button>
        </div>
      </div>
    </Modal>
  );
};

export default Dialog;
