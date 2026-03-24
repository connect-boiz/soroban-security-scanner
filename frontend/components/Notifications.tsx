'use client';

import { useState, useEffect } from 'react';
import { NotificationService } from '@/services/notificationService';
import { NotificationData } from '@/types/bounty';
import { 
  Bell, 
  X, 
  Check, 
  CheckCircle, 
  AlertTriangle, 
  Trophy,
  FileText,
  Settings,
  Trash2
} from 'lucide-react';

export const NotificationCenter: React.FC = () => {
  const [notifications, setNotifications] = useState<NotificationData[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [notificationService] = useState(() => NotificationService.getInstance());

  useEffect(() => {
    const unsubscribe = notificationService.subscribe(setNotifications);
    return unsubscribe;
  }, [notificationService]);

  const unreadCount = notifications.filter(n => !n.read).length;

  const markAsRead = (id: string) => {
    notificationService.markAsRead(id);
  };

  const markAllAsRead = () => {
    notificationService.markAllAsRead();
  };

  const clearAll = () => {
    notificationService.clearAll();
    setIsOpen(false);
  };

  const getNotificationIcon = (type: NotificationData['type']) => {
    switch (type) {
      case 'new_bounty':
        return <Trophy className="h-5 w-5 text-yellow-500" />;
      case 'submission_approved':
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case 'bounty_completed':
        return <Trophy className="h-5 w-5 text-blue-500" />;
      case 'dispute_raised':
        return <AlertTriangle className="h-5 w-5 text-red-500" />;
      default:
        return <Bell className="h-5 w-5 text-gray-500" />;
    }
  };

  const formatTimeAgo = (date: Date) => {
    const now = new Date();
    const diffInMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60));
    
    if (diffInMinutes < 1) return 'just now';
    if (diffInMinutes < 60) return `${diffInMinutes}m ago`;
    
    const diffInHours = Math.floor(diffInMinutes / 60);
    if (diffInHours < 24) return `${diffInHours}h ago`;
    
    const diffInDays = Math.floor(diffInHours / 24);
    return `${diffInDays}d ago`;
  };

  return (
    <div className="relative">
      {/* Bell Button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="relative p-2 text-gray-600 hover:text-gray-900 transition-colors"
      >
        <Bell className="h-6 w-6" />
        {unreadCount > 0 && (
          <span className="absolute -top-1 -right-1 h-5 w-5 bg-red-500 text-white text-xs rounded-full flex items-center justify-center">
            {unreadCount > 9 ? '9+' : unreadCount}
          </span>
        )}
      </button>

      {/* Notification Dropdown */}
      {isOpen && (
        <div className="absolute right-0 mt-2 w-96 bg-white rounded-lg shadow-xl border border-gray-200 z-50">
          {/* Header */}
          <div className="flex items-center justify-between p-4 border-b border-gray-200">
            <h3 className="font-semibold text-gray-900">Notifications</h3>
            <div className="flex items-center space-x-2">
              {unreadCount > 0 && (
                <button
                  onClick={markAllAsRead}
                  className="text-sm text-blue-600 hover:text-blue-800 flex items-center"
                >
                  <Check className="h-4 w-4 mr-1" />
                  Mark all read
                </button>
              )}
              <button
                onClick={clearAll}
                className="text-sm text-red-600 hover:text-red-800 flex items-center"
              >
                <Trash2 className="h-4 w-4 mr-1" />
                Clear all
              </button>
              <button
                onClick={() => setIsOpen(false)}
                className="text-gray-500 hover:text-gray-700"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
          </div>

          {/* Notifications List */}
          <div className="max-h-96 overflow-y-auto">
            {notifications.length === 0 ? (
              <div className="p-8 text-center text-gray-500">
                <Bell className="h-12 w-12 mx-auto mb-3 text-gray-300" />
                <p>No notifications yet</p>
                <p className="text-sm">We'll notify you when new bounties are posted</p>
              </div>
            ) : (
              <div className="divide-y divide-gray-100">
                {notifications.map((notification) => (
                  <div
                    key={notification.id}
                    className={`p-4 hover:bg-gray-50 transition-colors cursor-pointer ${
                      !notification.read ? 'bg-blue-50' : ''
                    }`}
                    onClick={() => markAsRead(notification.id)}
                  >
                    <div className="flex items-start">
                      <div className="flex-shrink-0 mr-3">
                        {getNotificationIcon(notification.type)}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between mb-1">
                          <p className="text-sm font-medium text-gray-900 truncate">
                            {notification.title}
                          </p>
                          {!notification.read && (
                            <span className="h-2 w-2 bg-blue-600 rounded-full flex-shrink-0 ml-2"></span>
                          )}
                        </div>
                        <p className="text-sm text-gray-600 mb-2">
                          {notification.message}
                        </p>
                        <p className="text-xs text-gray-500">
                          {formatTimeAgo(notification.timestamp)}
                        </p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="p-3 border-t border-gray-200 bg-gray-50">
            <button className="w-full text-sm text-blue-600 hover:text-blue-800 flex items-center justify-center">
              <Settings className="h-4 w-4 mr-1" />
              Notification Settings
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export const NotificationToast: React.FC<{ 
  notification: NotificationData;
  onClose: () => void;
}> = ({ notification, onClose }) => {
  const getNotificationIcon = (type: NotificationData['type']) => {
    switch (type) {
      case 'new_bounty':
        return <Trophy className="h-5 w-5 text-yellow-500" />;
      case 'submission_approved':
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case 'bounty_completed':
        return <Trophy className="h-5 w-5 text-blue-500" />;
      case 'dispute_raised':
        return <AlertTriangle className="h-5 w-5 text-red-500" />;
      default:
        return <Bell className="h-5 w-5 text-gray-500" />;
    }
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      onClose();
    }, 5000);

    return () => clearTimeout(timer);
  }, [onClose]);

  return (
    <div className="fixed top-4 right-4 w-80 bg-white rounded-lg shadow-lg border border-gray-200 p-4 z-50 animate-pulse">
      <div className="flex items-start">
        <div className="flex-shrink-0 mr-3">
          {getNotificationIcon(notification.type)}
        </div>
        <div className="flex-1">
          <div className="flex items-center justify-between mb-1">
            <p className="text-sm font-medium text-gray-900">
              {notification.title}
            </p>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
          <p className="text-sm text-gray-600">
            {notification.message}
          </p>
        </div>
      </div>
    </div>
  );
};

export const NotificationSettings: React.FC = () => {
  const [settings, setSettings] = useState({
    newBounty: true,
    submissionApproved: true,
    bountyCompleted: true,
    disputeRaised: true,
    browserNotifications: true,
    emailNotifications: false
  });

  const handleSettingChange = (key: string, value: boolean) => {
    setSettings(prev => ({ ...prev, [key]: value }));
  };

  const requestBrowserPermission = async () => {
    if ('Notification' in window) {
      const permission = await Notification.requestPermission();
      if (permission === 'granted') {
        setSettings(prev => ({ ...prev, browserNotifications: true }));
      }
    }
  };

  return (
    <div className="max-w-2xl mx-auto p-6">
      <div className="card">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">Notification Settings</h2>
        
        <div className="space-y-6">
          {/* Notification Types */}
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Notification Types</h3>
            <div className="space-y-3">
              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">New Bounties</span>
                  <p className="text-sm text-gray-600">Get notified when new bounties are posted</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.newBounty}
                  onChange={(e) => handleSettingChange('newBounty', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>

              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">Submission Approved</span>
                  <p className="text-sm text-gray-600">Get notified when your submissions are approved</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.submissionApproved}
                  onChange={(e) => handleSettingChange('submissionApproved', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>

              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">Bounty Completed</span>
                  <p className="text-sm text-gray-600">Get notified when bounties are completed</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.bountyCompleted}
                  onChange={(e) => handleSettingChange('bountyCompleted', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>

              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">Dispute Raised</span>
                  <p className="text-sm text-gray-600">Get notified when disputes are raised</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.disputeRaised}
                  onChange={(e) => handleSettingChange('disputeRaised', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>
            </div>
          </div>

          {/* Delivery Methods */}
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Delivery Methods</h3>
            <div className="space-y-3">
              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">Browser Notifications</span>
                  <p className="text-sm text-gray-600">Show desktop notifications</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.browserNotifications}
                  onChange={(e) => handleSettingChange('browserNotifications', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>

              <label className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-gray-900">Email Notifications</span>
                  <p className="text-sm text-gray-600">Receive notifications via email</p>
                </div>
                <input
                  type="checkbox"
                  checked={settings.emailNotifications}
                  onChange={(e) => handleSettingChange('emailNotifications', e.target.checked)}
                  className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                />
              </label>
            </div>
          </div>

          {/* Browser Permission */}
          {settings.browserNotifications && 'Notification' in window && Notification.permission === 'default' && (
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <div className="flex items-center">
                <Bell className="h-5 w-5 text-blue-600 mr-2" />
                <span className="text-blue-800">Enable browser notifications for real-time alerts</span>
              </div>
              <button
                onClick={requestBrowserPermission}
                className="mt-3 btn-primary"
              >
                Enable Browser Notifications
              </button>
            </div>
          )}

          {/* Save Button */}
          <div className="flex justify-end pt-6 border-t border-gray-200">
            <button className="btn-primary">
              Save Settings
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
