import React, { useState } from 'react';
import { NotificationProvider, useNotifications } from '../../hooks/notifications';
import EmailPreferencesPanel from '../../components/notifications/EmailPreferences';
import { NotificationDropdown } from '../../components/notifications/InAppNotification';
import { useInAppNotifications } from '../../hooks/notifications';

function NotificationsContent() {
  const { 
    notifications, 
    unreadCount, 
    markAsRead, 
    markAllAsRead, 
    deleteNotification, 
    clearNotifications,
    preferences,
    updatePreferences 
  } = useNotifications();

  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center">
              <h1 className="text-2xl font-bold text-gray-900">Notifications</h1>
              <div className="ml-6">
                <NotificationDropdown
                  notifications={notifications}
                  unreadCount={unreadCount}
                  isOpen={isDropdownOpen}
                  onToggle={() => setIsDropdownOpen(!isDropdownOpen)}
                  onRead={markAsRead}
                  onDelete={deleteNotification}
                  onMarkAllAsRead={markAllAsRead}
                  onClearAll={clearNotifications}
                />
              </div>
            </div>
            
            {unreadCount > 0 && (
              <button
                onClick={markAllAsRead}
                className="px-4 py-2 text-sm font-medium text-blue-600 hover:text-blue-800 bg-blue-50 rounded-md hover:bg-blue-100 transition-colors"
              >
                Mark all as read
              </button>
            )}
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Notifications List */}
          <div className="lg:col-span-2">
            <div className="bg-white rounded-lg shadow">
              <div className="px-6 py-4 border-b border-gray-200">
                <h2 className="text-lg font-medium text-gray-900">
                  Recent Notifications
                  {unreadCount > 0 && (
                    <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                      {unreadCount} new
                    </span>
                  )}
                </h2>
              </div>
              
              <div className="divide-y divide-gray-200">
                {notifications.length === 0 ? (
                  <div className="px-6 py-12 text-center">
                    <div className="text-gray-400 text-lg mb-2">📭</div>
                    <h3 className="text-lg font-medium text-gray-900 mb-1">No notifications</h3>
                    <p className="text-sm text-gray-500">You're all caught up! Check back later for updates.</p>
                  </div>
                ) : (
                  notifications.slice(0, 20).map((notification) => (
                    <div
                      key={notification.id}
                      className={`p-6 hover:bg-gray-50 transition-colors ${
                        !notification.read_at ? 'bg-blue-50' : ''
                      }`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-2">
                            <span className="text-lg">
                              {notification.type === 'scan_completed' && '✅'}
                              {notification.type === 'vulnerability_found' && '⚠️'}
                              {notification.type === 'scan_failed' && '❌'}
                              {notification.type === 'security_alert' && '🚨'}
                              {notification.type === 'system_maintenance' && '🔧'}
                              {notification.type === 'account_update' && '👤'}
                              {notification.type === 'bounty_claimed' && '💰'}
                              {notification.type === 'escrow_released' && '🔓'}
                            </span>
                            <h3 className="text-sm font-medium text-gray-900">
                              {notification.title}
                            </h3>
                            {!notification.read_at && (
                              <span className="inline-flex items-center px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 rounded-full">
                                New
                              </span>
                            )}
                          </div>
                          
                          <p className="text-sm text-gray-600 mb-2">
                            {notification.message}
                          </p>
                          
                          <div className="flex items-center gap-4 text-xs text-gray-500">
                            <span>{new Date(notification.created_at).toLocaleDateString()}</span>
                            <span>•</span>
                            <span className={`font-medium ${
                              notification.priority === 'critical' ? 'text-red-600' :
                              notification.priority === 'high' ? 'text-orange-600' :
                              notification.priority === 'normal' ? 'text-blue-600' :
                              'text-gray-600'
                            }`}>
                              {notification.priority.toUpperCase()}
                            </span>
                          </div>

                          {notification.action && (
                            <div className="mt-3">
                              <button
                                onClick={() => {
                                  if (notification.action?.url) {
                                    window.open(notification.action.url, '_blank');
                                  } else if (notification.action?.onClick) {
                                    notification.action.onClick();
                                  }
                                  if (!notification.read_at) {
                                    markAsRead(notification.id);
                                  }
                                }}
                                className="inline-flex items-center px-3 py-1.5 text-xs font-medium bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                              >
                                {notification.action.label}
                              </button>
                            </div>
                          )}
                        </div>

                        <div className="flex items-center gap-2 ml-4">
                          {!notification.read_at && (
                            <button
                              onClick={() => markAsRead(notification.id)}
                              className="p-1.5 text-blue-600 hover:text-blue-800 transition-colors"
                              title="Mark as read"
                            >
                              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                              </svg>
                            </button>
                          )}
                          <button
                            onClick={() => deleteNotification(notification.id)}
                            className="p-1.5 text-red-600 hover:text-red-800 transition-colors"
                            title="Delete notification"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                          </button>
                        </div>
                      </div>
                    </div>
                  ))
                )}
              </div>

              {notifications.length > 20 && (
                <div className="px-6 py-4 border-t border-gray-200 bg-gray-50">
                  <p className="text-sm text-gray-600 text-center">
                    Showing 20 of {notifications.length} notifications
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Settings Panel */}
          <div className="lg:col-span-1">
            <EmailPreferencesPanel
              preferences={preferences}
              onUpdate={updatePreferences}
              isLoading={false}
            />
          </div>
        </div>
      </main>
    </div>
  );
}

export default function NotificationsPage() {
  return (
    <NotificationProvider>
      <NotificationsContent />
    </NotificationProvider>
  );
}
