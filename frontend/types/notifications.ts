// Notification system types for Soroban Security Scanner

export enum NotificationChannel {
  EMAIL = 'email',
  SMS = 'sms',
  PUSH = 'push',
  IN_APP = 'in_app',
}

export enum NotificationPriority {
  LOW = 'low',
  NORMAL = 'normal',
  HIGH = 'high',
  CRITICAL = 'critical',
}

export enum NotificationType {
  SCAN_COMPLETED = 'scan_completed',
  VULNERABILITY_FOUND = 'vulnerability_found',
  SCAN_FAILED = 'scan_failed',
  SECURITY_ALERT = 'security_alert',
  SYSTEM_MAINTENANCE = 'system_maintenance',
  ACCOUNT_UPDATE = 'account_update',
  BOUNTY_CLAIMED = 'bounty_claimed',
  ESCROW_RELEASED = 'escrow_released',
}

export enum DeliveryStatus {
  PENDING = 'pending',
  PROCESSING = 'processing',
  SENT = 'sent',
  DELIVERED = 'delivered',
  FAILED = 'failed',
  RETRYING = 'retrying',
}

export interface NotificationPreferences {
  email_enabled: boolean;
  sms_enabled: boolean;
  push_enabled: boolean;
  in_app_enabled: boolean;
  quiet_hours?: QuietHours;
  max_priority: NotificationPriority;
  notification_types: Record<NotificationType, boolean>;
}

export interface QuietHours {
  start_hour: number; // 0-23
  end_hour: number; // 0-23
  timezone: string;
}

export interface NotificationMessage {
  id: string;
  template_id?: string;
  subject?: string;
  body: string;
  data: Record<string, string>;
  priority: NotificationPriority;
  type: NotificationType;
  channels: NotificationChannel[];
  created_at: string;
  scheduled_for?: string;
  read_at?: string;
  expires_at?: string;
}

export interface DeliveryTracking {
  notification_id: string;
  recipient_id: string;
  channel: NotificationChannel;
  status: DeliveryStatus;
  attempts: number;
  last_attempt: string;
  delivered_at?: string;
  error_message?: string;
  external_id?: string;
}

export interface NotificationResult {
  notification_id: string;
  success: boolean;
  delivered_channels: NotificationChannel[];
  failed_channels: Array<{ channel: NotificationChannel; error: string }>;
  tracking_ids: string[];
}

export interface Recipient {
  id: string;
  email?: string;
  phone?: string;
  device_tokens: string[];
  user_id?: string;
  preferences: NotificationPreferences;
}

export interface ToastNotification {
  id: string;
  title: string;
  message?: string;
  type: NotificationType;
  priority: NotificationPriority;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
  dismissible?: boolean;
}

export interface InAppNotification {
  id: string;
  title: string;
  message: string;
  type: NotificationType;
  priority: NotificationPriority;
  created_at: string;
  read_at?: string;
  action?: {
    label: string;
    url?: string;
    onClick?: () => void;
  };
}

export interface NotificationStats {
  total_sent: number;
  total_delivered: number;
  total_failed: number;
  delivery_rate: number;
  average_delivery_time: number;
  by_type: Record<NotificationType, number>;
  by_priority: Record<NotificationPriority, number>;
  by_channel: Record<NotificationChannel, number>;
}

export interface EmailTemplate {
  id: string;
  name: string;
  subject: string;
  html_body: string;
  text_body: string;
  variables: string[];
  default_type: NotificationType;
  default_priority: NotificationPriority;
}

export interface NotificationContextType {
  // Toast notifications
  toasts: ToastNotification[];
  addToast: (toast: Omit<ToastNotification, 'id'>) => void;
  removeToast: (id: string) => void;
  clearToasts: () => void;

  // In-app notifications
  notifications: InAppNotification[];
  unreadCount: number;
  addNotification: (notification: Omit<InAppNotification, 'id'>) => void;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  deleteNotification: (id: string) => void;
  clearNotifications: () => void;

  // Preferences
  preferences: NotificationPreferences;
  updatePreferences: (preferences: Partial<NotificationPreferences>) => void;

  // Actions
  sendNotification: (
    message: Omit<NotificationMessage, 'id' | 'created_at'>,
    recipient?: Recipient
  ) => Promise<NotificationResult>;

  // Stats
  stats: NotificationStats | null;
  fetchStats: () => Promise<void>;
}

// API Response types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// Event types for real-time updates
export interface NotificationEvent {
  type: 'notification_added' | 'notification_read' | 'notification_deleted' | 'preferences_updated';
  data: any;
  timestamp: string;
}

// Email settings
export interface EmailSettings {
  smtp_host: string;
  smtp_port: number;
  smtp_username: string;
  smtp_password: string;
  from_email: string;
  from_name: string;
  use_tls: boolean;
  use_ssl: boolean;
}

// Push notification settings
export interface PushSettings {
  vapid_public_key: string;
  vapid_private_key: string;
  vapid_email: string;
  firebase_config?: {
    apiKey: string;
    authDomain: string;
    projectId: string;
    messagingSenderId: string;
    appId: string;
  };
}

// SMS settings
export interface SmsSettings {
  provider: 'twilio' | 'aws-sns' | 'sendgrid';
  account_sid?: string;
  auth_token?: string;
  from_number?: string;
  region?: string;
  access_key_id?: string;
  secret_access_key?: string;
}
