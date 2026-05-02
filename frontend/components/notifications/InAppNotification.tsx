import React, { useState } from 'react';
import { InAppNotification, NotificationPriority, NotificationType } from '../../types/notifications';
import { cn, getPriorityColor, formatTimestamp, getNotificationIcon, getNotificationTypeLabel } from '../../lib/notifications/utils';

interface InAppNotificationProps {
  notification: InAppNotification;
  onRead: (id: string) => void;
  onDelete: (id: string) => void;
  onAction?: (id: string, action: any) => void;
}

export const InAppNotificationItem: React.FC<InAppNotificationProps> = ({
  notification,
  onRead,
  onDelete,
  onAction,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const isUnread = !notification.read_at;

  const handleRead = () => {
    if (isUnread) {
      onRead(notification.id);
    }
  };

  const handleAction = () => {
    if (notification.action) {
      if (onAction) {
        onAction(notification.id, notification.action);
      } else if (notification.action.onClick) {
        notification.action.onClick();
      } else if (notification.action.url) {
        window.open(notification.action.url, '_blank');
      }
    }
  };

  const priorityClasses = getPriorityColor(notification.priority);

  return (
    <div
      className={cn(
        'p-4 border rounded-lg transition-all duration-200 cursor-pointer',
        'hover:shadow-md hover:border-gray-300',
        isUnread && 'bg-blue-50 border-blue-200',
        !isUnread && 'bg-white border-gray-200',
        priorityClasses
      )}
      onClick={handleRead}
    >
      <div className="flex items-start gap-3">
        {/* Icon */}
        <div className="flex-shrink-0 mt-0.5">
          <span className="text-lg">{getNotificationIcon(notification.type)}</span>
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-1">
                <h4 className="text-sm font-semibold text-gray-900 truncate">
                  {notification.title}
                </h4>
                {isUnread && (
                  <span className="inline-flex items-center px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 rounded-full">
                    New
                  </span>
                )}
              </div>
              
              <p className="text-sm text-gray-600 leading-relaxed">
                {isExpanded ? notification.message : notification.message.substring(0, 100)}
                {notification.message.length > 100 && !isExpanded && '...'}
              </p>

              {notification.message.length > 100 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsExpanded(!isExpanded);
                  }}
                  className="text-xs text-blue-600 hover:text-blue-800 mt-1"
                >
                  {isExpanded ? 'Show less' : 'Show more'}
                </button>
              )}

              <div className="flex items-center gap-3 mt-2 text-xs text-gray-500">
                <span>{getNotificationTypeLabel(notification.type)}</span>
                <span>•</span>
                <span>{formatTimestamp(notification.created_at)}</span>
                <span>•</span>
                <span className={cn(
                  'font-medium',
                  notification.priority === NotificationPriority.CRITICAL && 'text-red-600',
                  notification.priority === NotificationPriority.HIGH && 'text-orange-600',
                  notification.priority === NotificationPriority.NORMAL && 'text-blue-600',
                  notification.priority === NotificationPriority.LOW && 'text-gray-600'
                )}>
                  {notification.priority.toUpperCase()}
                </span>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center gap-1 flex-shrink-0">
              {notification.action && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleAction();
                  }}
                  className="p-1.5 text-xs font-medium bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                >
                  {notification.action.label}
                </button>
              )}
              
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(notification.id);
                }}
                className="p-1.5 text-gray-400 hover:text-red-600 transition-colors"
                aria-label="Delete notification"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

interface InAppNotificationListProps {
  notifications: InAppNotification[];
  onRead: (id: string) => void;
  onDelete: (id: string) => void;
  onMarkAllAsRead: () => void;
  onClearAll: () => void;
  onAction?: (id: string, action: any) => void;
}

export const InAppNotificationList: React.FC<InAppNotificationListProps> = ({
  notifications,
  onRead,
  onDelete,
  onMarkAllAsRead,
  onClearAll,
  onAction,
}) => {
  const unreadCount = notifications.filter(n => !n.read_at).length;

  if (notifications.length === 0) {
    return (
      <div className="text-center py-12">
        <div className="text-gray-400 text-lg mb-2">📭</div>
        <h3 className="text-lg font-medium text-gray-900 mb-1">No notifications</h3>
        <p className="text-sm text-gray-500">You're all caught up! Check back later for updates.</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between pb-3 border-b">
        <div className="flex items-center gap-2">
          <h2 className="text-lg font-semibold text-gray-900">Notifications</h2>
          {unreadCount > 0 && (
            <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
              {unreadCount} unread
            </span>
          )}
        </div>
        
        <div className="flex items-center gap-2">
          {unreadCount > 0 && (
            <button
              onClick={onMarkAllAsRead}
              className="text-sm text-blue-600 hover:text-blue-800 font-medium"
            >
              Mark all as read
            </button>
          )}
          <button
            onClick={onClearAll}
            className="text-sm text-red-600 hover:text-red-800 font-medium"
          >
            Clear all
          </button>
        </div>
      </div>

      {/* Notification List */}
      <div className="space-y-3">
        {notifications.map((notification) => (
          <InAppNotificationItem
            key={notification.id}
            notification={notification}
            onRead={onRead}
            onDelete={onDelete}
            onAction={onAction}
          />
        ))}
      </div>
    </div>
  );
};

interface NotificationDropdownProps {
  notifications: InAppNotification[];
  unreadCount: number;
  isOpen: boolean;
  onToggle: () => void;
  onRead: (id: string) => void;
  onDelete: (id: string) => void;
  onMarkAllAsRead: () => void;
  onClearAll: () => void;
  onAction?: (id: string, action: any) => void;
}

export const NotificationDropdown: React.FC<NotificationDropdownProps> = ({
  notifications,
  unreadCount,
  isOpen,
  onToggle,
  onRead,
  onDelete,
  onMarkAllAsRead,
  onClearAll,
  onAction,
}) => {
  return (
    <div className="relative">
      {/* Notification Bell Button */}
      <button
        onClick={onToggle}
        className="relative p-2 text-gray-600 hover:text-gray-900 transition-colors"
        aria-label={`Notifications ${unreadCount > 0 ? `(${unreadCount} unread)` : ''}`}
      >
        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
        </svg>
        
        {unreadCount > 0 && (
          <span className="absolute -top-1 -right-1 inline-flex items-center justify-center px-2 py-1 text-xs font-bold text-white bg-red-500 rounded-full min-w-[1.25rem] h-5">
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {/* Dropdown */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-40"
            onClick={onToggle}
          />
          
          {/* Dropdown Content */}
          <div className="absolute right-0 mt-2 w-96 bg-white rounded-lg shadow-lg border border-gray-200 z-50 max-h-96 overflow-hidden">
            <div className="p-4 max-h-96 overflow-y-auto">
              <InAppNotificationList
                notifications={notifications.slice(0, 10)} // Show max 10 in dropdown
                onRead={onRead}
                onDelete={onDelete}
                onMarkAllAsRead={onMarkAllAsRead}
                onClearAll={onClearAll}
                onAction={onAction}
              />
            </div>
            
            {notifications.length > 10 && (
              <div className="p-3 border-t bg-gray-50">
                <button className="w-full text-sm text-blue-600 hover:text-blue-800 font-medium text-center">
                  View all notifications ({notifications.length})
                </button>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};

export default InAppNotificationList;
