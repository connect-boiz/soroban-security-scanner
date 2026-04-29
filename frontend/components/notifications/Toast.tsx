import React, { useEffect, useState } from 'react';
import { ToastNotification, NotificationPriority, NotificationType } from '../../types/notifications';
import { cn, getPriorityColor, getToastDuration, getNotificationIcon } from '../../lib/notifications/utils';

interface ToastProps {
  toast: ToastNotification;
  onClose: (id: string) => void;
}

export const Toast: React.FC<ToastProps> = ({ toast, onClose }) => {
  const [isVisible, setIsVisible] = useState(false);
  const [isLeaving, setIsLeaving] = useState(false);

  useEffect(() => {
    // Trigger entrance animation
    const timer = setTimeout(() => setIsVisible(true), 10);

    // Auto-dismiss if duration is set
    if (toast.duration && toast.duration > 0) {
      const dismissTimer = setTimeout(() => {
        handleClose();
      }, toast.duration);
      return () => {
        clearTimeout(timer);
        clearTimeout(dismissTimer);
      };
    }

    return () => clearTimeout(timer);
  }, [toast.duration]);

  const handleClose = () => {
    setIsLeaving(true);
    setTimeout(() => onClose(toast.id), 300);
  };

  const getIconElement = () => {
    const iconText = getNotificationIcon(toast.type);
    return <span className="text-lg">{iconText}</span>;
  };

  const priorityClasses = getPriorityColor(toast.priority);

  return (
    <div
      className={cn(
        'relative flex items-start gap-3 p-4 rounded-lg border shadow-lg max-w-md w-full',
        'transform transition-all duration-300 ease-in-out',
        'backdrop-blur-sm',
        priorityClasses,
        isVisible && !isLeaving ? 'translate-x-0 opacity-100 scale-100' : 'translate-x-full opacity-0 scale-95',
        isLeaving && 'translate-x-full opacity-0 scale-95'
      )}
      role="alert"
      aria-live={toast.priority === NotificationPriority.CRITICAL ? 'assertive' : 'polite'}
    >
      {/* Icon */}
      <div className="flex-shrink-0 mt-0.5">
        {getIconElement()}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1">
            <h4 className="text-sm font-semibold text-gray-900">
              {toast.title}
            </h4>
            {toast.message && (
              <p className="mt-1 text-sm text-gray-600 leading-relaxed">
                {toast.message}
              </p>
            )}
          </div>

          {/* Close button */}
          {(toast.dismissible !== false) && (
            <button
              onClick={handleClose}
              className="flex-shrink-0 p-1 rounded-md hover:bg-black/5 transition-colors"
              aria-label="Close notification"
            >
              <svg className="w-4 h-4 text-gray-400 hover:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>

        {/* Action button */}
        {toast.action && (
          <div className="mt-3">
            <button
              onClick={toast.action.onClick}
              className="inline-flex items-center px-3 py-1.5 text-xs font-medium rounded-md border border-current/20 hover:border-current/30 transition-colors"
            >
              {toast.action.label}
            </button>
          </div>
        )}

        {/* Progress indicator for auto-dismiss */}
        {toast.duration && toast.duration > 0 && (
          <div className="absolute bottom-0 left-0 h-1 bg-current/20 rounded-b-lg overflow-hidden">
            <div
              className="h-full bg-current/40 rounded-b-lg"
              style={{
                animation: `shrink ${toast.duration}ms linear forwards`,
              }}
            />
          </div>
        )}
      </div>

      {/* Priority indicator dot */}
      <div
        className={cn(
          'absolute top-4 right-4 w-2 h-2 rounded-full',
          toast.priority === NotificationPriority.CRITICAL && 'bg-red-500 animate-pulse',
          toast.priority === NotificationPriority.HIGH && 'bg-orange-500',
          toast.priority === NotificationPriority.NORMAL && 'bg-blue-500',
          toast.priority === NotificationPriority.LOW && 'bg-gray-400'
        )}
      />
    </div>
  );
};

interface ToastContainerProps {
  toasts: ToastNotification[];
  onClose: (id: string) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, onClose }) => {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-50 space-y-3 pointer-events-none">
      <div className="space-y-3 pointer-events-auto">
        {toasts.map((toast) => (
          <Toast key={toast.id} toast={toast} onClose={onClose} />
        ))}
      </div>
    </div>
  );
};

// Add the shrink animation to the global styles
export const ToastStyles = () => (
  <style jsx>{`
    @keyframes shrink {
      from {
        width: 100%;
      }
      to {
        width: 0%;
      }
    }
  `}</style>
);

export default ToastContainer;
