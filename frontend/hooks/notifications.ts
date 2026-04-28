import React, { createContext, useContext, useReducer, useEffect, useCallback } from 'react';
import { 
  NotificationContextType, 
  ToastNotification, 
  InAppNotification, 
  NotificationPreferences,
  NotificationMessage,
  NotificationResult,
  NotificationType,
  NotificationPriority,
  NotificationChannel,
  Recipient
} from '../types/notifications';
import { generateId, getToastDuration } from '../lib/notifications/utils';

// Default preferences
const defaultPreferences: NotificationPreferences = {
  email_enabled: true,
  sms_enabled: true,
  push_enabled: true,
  in_app_enabled: true,
  max_priority: NotificationPriority.NORMAL,
  notification_types: {
    [NotificationType.SCAN_COMPLETED]: true,
    [NotificationType.VULNERABILITY_FOUND]: true,
    [NotificationType.SCAN_FAILED]: true,
    [NotificationType.SECURITY_ALERT]: true,
    [NotificationType.SYSTEM_MAINTENANCE]: true,
    [NotificationType.ACCOUNT_UPDATE]: true,
    [NotificationType.BOUNTY_CLAIMED]: true,
    [NotificationType.ESCROW_RELEASED]: true,
  },
};

// State interface
interface NotificationState {
  toasts: ToastNotification[];
  notifications: InAppNotification[];
  preferences: NotificationPreferences;
  unreadCount: number;
  stats: any;
  isLoading: boolean;
}

// Action types
type NotificationAction =
  | { type: 'ADD_TOAST'; payload: Omit<ToastNotification, 'id'> }
  | { type: 'REMOVE_TOAST'; payload: string }
  | { type: 'CLEAR_TOASTS' }
  | { type: 'ADD_NOTIFICATION'; payload: Omit<InAppNotification, 'id'> }
  | { type: 'MARK_AS_READ'; payload: string }
  | { type: 'MARK_ALL_AS_READ' }
  | { type: 'DELETE_NOTIFICATION'; payload: string }
  | { type: 'CLEAR_NOTIFICATIONS' }
  | { type: 'UPDATE_PREFERENCES'; payload: Partial<NotificationPreferences> }
  | { type: 'SET_STATS'; payload: any }
  | { type: 'SET_LOADING'; payload: boolean };

// Reducer
function notificationReducer(state: NotificationState, action: NotificationAction): NotificationState {
  switch (action.type) {
    case 'ADD_TOAST': {
      const toast: ToastNotification = {
        id: generateId(),
        ...action.payload,
        duration: action.payload.duration ?? getToastDuration(action.payload.priority),
      };
      return {
        ...state,
        toasts: [...state.toasts, toast],
      };
    }

    case 'REMOVE_TOAST':
      return {
        ...state,
        toasts: state.toasts.filter((toast) => toast.id !== action.payload),
      };

    case 'CLEAR_TOASTS':
      return {
        ...state,
        toasts: [],
      };

    case 'ADD_NOTIFICATION': {
      const notification: InAppNotification = {
        id: generateId(),
        ...action.payload,
      };
      return {
        ...state,
        notifications: [notification, ...state.notifications],
        unreadCount: state.unreadCount + 1,
      };
    }

    case 'MARK_AS_READ': {
      const updatedNotifications = state.notifications.map((notif) =>
        notif.id === action.payload
          ? { ...notif, read_at: new Date().toISOString() }
          : notif
      );
      const readNotifications = updatedNotifications.filter((notif) => notif.read_at);
      const unreadNotifications = updatedNotifications.filter((notif) => !notif.read_at);
      
      return {
        ...state,
        notifications: updatedNotifications,
        unreadCount: unreadNotifications.length,
      };
    }

    case 'MARK_ALL_AS_READ': {
      const now = new Date().toISOString();
      const updatedNotifications = state.notifications.map((notif) => ({
        ...notif,
        read_at: notif.read_at || now,
      }));
      
      return {
        ...state,
        notifications: updatedNotifications,
        unreadCount: 0,
      };
    }

    case 'DELETE_NOTIFICATION':
      const updatedNotifications = state.notifications.filter(
        (notif) => notif.id !== action.payload
      );
      const unreadNotifications = updatedNotifications.filter((notif) => !notif.read_at);
      
      return {
        ...state,
        notifications: updatedNotifications,
        unreadCount: unreadNotifications.length,
      };

    case 'CLEAR_NOTIFICATIONS':
      return {
        ...state,
        notifications: [],
        unreadCount: 0,
      };

    case 'UPDATE_PREFERENCES':
      return {
        ...state,
        preferences: {
          ...state.preferences,
          ...action.payload,
        },
      };

    case 'SET_STATS':
      return {
        ...state,
        stats: action.payload,
      };

    case 'SET_LOADING':
      return {
        ...state,
        isLoading: action.payload,
      };

    default:
      return state;
  }
}

// Initial state
const initialState: NotificationState = {
  toasts: [],
  notifications: [],
  preferences: defaultPreferences,
  unreadCount: 0,
  stats: null,
  isLoading: false,
};

// Context
const NotificationContext = createContext<NotificationContextType | undefined>(undefined);

// Provider
export const NotificationProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [state, dispatch] = useReducer(notificationReducer, initialState);

  // Load preferences from localStorage on mount
  useEffect(() => {
    try {
      const savedPreferences = localStorage.getItem('notification-preferences');
      if (savedPreferences) {
        const parsed = JSON.parse(savedPreferences);
        dispatch({ type: 'UPDATE_PREFERENCES', payload: parsed });
      }
    } catch (error) {
      console.error('Failed to load notification preferences:', error);
    }
  }, []);

  // Save preferences to localStorage when they change
  useEffect(() => {
    try {
      localStorage.setItem('notification-preferences', JSON.stringify(state.preferences));
    } catch (error) {
      console.error('Failed to save notification preferences:', error);
    }
  }, [state.preferences]);

  // Toast actions
  const addToast = useCallback((toast: Omit<ToastNotification, 'id'>) => {
    dispatch({ type: 'ADD_TOAST', payload: toast });
  }, []);

  const removeToast = useCallback((id: string) => {
    dispatch({ type: 'REMOVE_TOAST', payload: id });
  }, []);

  const clearToasts = useCallback(() => {
    dispatch({ type: 'CLEAR_TOASTS' });
  }, []);

  // In-app notification actions
  const addNotification = useCallback((notification: Omit<InAppNotification, 'id'>) => {
    dispatch({ type: 'ADD_NOTIFICATION', payload: notification });
  }, []);

  const markAsRead = useCallback((id: string) => {
    dispatch({ type: 'MARK_AS_READ', payload: id });
  }, []);

  const markAllAsRead = useCallback(() => {
    dispatch({ type: 'MARK_ALL_AS_READ' });
  }, []);

  const deleteNotification = useCallback((id: string) => {
    dispatch({ type: 'DELETE_NOTIFICATION', payload: id });
  }, []);

  const clearNotifications = useCallback(() => {
    dispatch({ type: 'CLEAR_NOTIFICATIONS' });
  }, []);

  // Preferences actions
  const updatePreferences = useCallback((preferences: Partial<NotificationPreferences>) => {
    dispatch({ type: 'UPDATE_PREFERENCES', payload: preferences });
  }, []);

  // Send notification action
  const sendNotification = useCallback(async (
    message: Omit<NotificationMessage, 'id' | 'created_at'>,
    recipient?: Recipient
  ): Promise<NotificationResult> => {
    dispatch({ type: 'SET_LOADING', payload: true });

    try {
      const notificationMessage: NotificationMessage = {
        id: generateId(),
        created_at: new Date().toISOString(),
        ...message,
      };

      // Create toast for immediate feedback
      if (state.preferences.in_app_enabled) {
        addToast({
          title: message.subject || 'Notification',
          message: message.body.length > 100 ? message.body.substring(0, 100) + '...' : message.body,
          type: message.type,
          priority: message.priority,
          action: message.data?.actionUrl ? {
            label: 'View Details',
            onClick: () => window.open(message.data?.actionUrl, '_blank'),
          } : undefined,
        });
      }

      // Add to in-app notifications
      if (state.preferences.in_app_enabled && state.preferences.notification_types[message.type]) {
        addNotification({
          title: message.subject || 'Notification',
          message: message.body,
          type: message.type,
          priority: message.priority,
          created_at: notificationMessage.created_at,
          action: message.data?.actionUrl ? {
            label: 'View Details',
            url: message.data.actionUrl,
          } : undefined,
        });
      }

      // In a real implementation, this would call the backend API
      // For now, we'll simulate a successful response
      const result: NotificationResult = {
        notification_id: notificationMessage.id,
        success: true,
        delivered_channels: message.channels,
        failed_channels: [],
        tracking_ids: [],
      };

      dispatch({ type: 'SET_LOADING', payload: false });
      return result;
    } catch (error) {
      dispatch({ type: 'SET_LOADING', payload: false });
      throw error;
    }
  }, [state.preferences, addToast, addNotification]);

  // Stats actions
  const fetchStats = useCallback(async () => {
    dispatch({ type: 'SET_LOADING', payload: true });
    try {
      // In a real implementation, this would fetch from the API
      const mockStats = {
        total_sent: 1250,
        total_delivered: 1180,
        total_failed: 70,
        delivery_rate: 94.4,
        average_delivery_time: 250,
        by_type: {
          [NotificationType.SCAN_COMPLETED]: 450,
          [NotificationType.VULNERABILITY_FOUND]: 180,
          [NotificationType.SCAN_FAILED]: 45,
          [NotificationType.SECURITY_ALERT]: 120,
          [NotificationType.SYSTEM_MAINTENANCE]: 85,
          [NotificationType.ACCOUNT_UPDATE]: 200,
          [NotificationType.BOUNTY_CLAIMED]: 95,
          [NotificationType.ESCROW_RELEASED]: 75,
        },
        by_priority: {
          [NotificationPriority.LOW]: 400,
          [NotificationPriority.NORMAL]: 600,
          [NotificationPriority.HIGH]: 200,
          [NotificationPriority.CRITICAL]: 50,
        },
        by_channel: {
          [NotificationChannel.EMAIL]: 800,
          [NotificationChannel.SMS]: 150,
          [NotificationChannel.PUSH]: 200,
          [NotificationChannel.IN_APP]: 100,
        },
      };
      
      dispatch({ type: 'SET_STATS', payload: mockStats });
    } catch (error) {
      console.error('Failed to fetch notification stats:', error);
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  }, []);

  const contextValue: NotificationContextType = {
    toasts: state.toasts,
    addToast,
    removeToast,
    clearToasts,
    notifications: state.notifications,
    unreadCount: state.unreadCount,
    addNotification,
    markAsRead,
    markAllAsRead,
    deleteNotification,
    clearNotifications,
    preferences: state.preferences,
    updatePreferences,
    sendNotification,
    stats: state.stats,
    fetchStats,
  };

  return React.createElement(
    NotificationContext.Provider,
    { value: contextValue },
    children
  );
};

// Hook
export const useNotifications = (): NotificationContextType => {
  const context = useContext(NotificationContext);
  if (context === undefined) {
    throw new Error('useNotifications must be used within a NotificationProvider');
  }
  return context;
};

// Convenience hooks
export const useToasts = () => {
  const { toasts, addToast, removeToast, clearToasts } = useNotifications();
  return { toasts, addToast, removeToast, clearToasts };
};

export const useInAppNotifications = () => {
  const { 
    notifications, 
    unreadCount, 
    addNotification, 
    markAsRead, 
    markAllAsRead, 
    deleteNotification, 
    clearNotifications 
  } = useNotifications();
  return { 
    notifications, 
    unreadCount, 
    addNotification, 
    markAsRead, 
    markAllAsRead, 
    deleteNotification, 
    clearNotifications 
  };
};

export const useNotificationPreferences = () => {
  const { preferences, updatePreferences } = useNotifications();
  return { preferences, updatePreferences };
};

export const useNotificationSender = () => {
  const { sendNotification } = useNotifications();
  return { sendNotification };
};
