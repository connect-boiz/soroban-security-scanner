import { NotificationPriority, NotificationType } from '../../types/notifications';

export function cn(...inputs: (string | undefined | null | boolean)[]) {
  return inputs.filter(Boolean).join(' ');
}

export function generateId(): string {
  return Math.random().toString(36).substr(2, 9);
}

export function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins} minute${diffMins > 1 ? 's' : ''} ago`;
  if (diffHours < 24) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
  if (diffDays < 7) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
  
  return date.toLocaleDateString();
}

export function getNotificationIcon(type: NotificationType): string {
  switch (type) {
    case NotificationType.SCAN_COMPLETED:
      return '✅';
    case NotificationType.VULNERABILITY_FOUND:
      return '⚠️';
    case NotificationType.SCAN_FAILED:
      return '❌';
    case NotificationType.SECURITY_ALERT:
      return '🚨';
    case NotificationType.SYSTEM_MAINTENANCE:
      return '🔧';
    case NotificationType.ACCOUNT_UPDATE:
      return '👤';
    case NotificationType.BOUNTY_CLAIMED:
      return '💰';
    case NotificationType.ESCROW_RELEASED:
      return '🔓';
    default:
      return '📢';
  }
}

export function getPriorityColor(priority: NotificationPriority): string {
  switch (priority) {
    case NotificationPriority.LOW:
      return 'text-gray-500 border-gray-200 bg-gray-50';
    case NotificationPriority.NORMAL:
      return 'text-blue-600 border-blue-200 bg-blue-50';
    case NotificationPriority.HIGH:
      return 'text-orange-600 border-orange-200 bg-orange-50';
    case NotificationPriority.CRITICAL:
      return 'text-red-600 border-red-200 bg-red-50';
    default:
      return 'text-gray-600 border-gray-200 bg-gray-50';
  }
}

export function getToastDuration(priority: NotificationPriority): number {
  switch (priority) {
    case NotificationPriority.LOW:
      return 3000;
    case NotificationPriority.NORMAL:
      return 5000;
    case NotificationPriority.HIGH:
      return 8000;
    case NotificationPriority.CRITICAL:
      return 0; // Auto-dismiss disabled for critical
    default:
      return 5000;
  }
}

export function getNotificationTypeLabel(type: NotificationType): string {
  switch (type) {
    case NotificationType.SCAN_COMPLETED:
      return 'Scan Completed';
    case NotificationType.VULNERABILITY_FOUND:
      return 'Vulnerability Found';
    case NotificationType.SCAN_FAILED:
      return 'Scan Failed';
    case NotificationType.SECURITY_ALERT:
      return 'Security Alert';
    case NotificationType.SYSTEM_MAINTENANCE:
      return 'System Maintenance';
    case NotificationType.ACCOUNT_UPDATE:
      return 'Account Update';
    case NotificationType.BOUNTY_CLAIMED:
      return 'Bounty Claimed';
    case NotificationType.ESCROW_RELEASED:
      return 'Escrow Released';
    default:
      return 'Notification';
  }
}

export function getPriorityLabel(priority: NotificationPriority): string {
  switch (priority) {
    case NotificationPriority.LOW:
      return 'Low';
    case NotificationPriority.NORMAL:
      return 'Normal';
    case NotificationPriority.HIGH:
      return 'High';
    case NotificationPriority.CRITICAL:
      return 'Critical';
    default:
      return 'Normal';
  }
}

export function validateEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

export function validatePhone(phone: string): boolean {
  const phoneRegex = /^\+?[\d\s\-\(\)]+$/;
  return phoneRegex.test(phone) && phone.replace(/\D/g, '').length >= 10;
}

export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.substr(0, maxLength - 3) + '...';
}

export function debounce<T extends (...args: any[]) => void>(
  func: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: number;
  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = window.setTimeout(() => func(...args), delay);
  };
}

export function throttle<T extends (...args: any[]) => void>(
  func: T,
  limit: number
): (...args: Parameters<T>) => void {
  let inThrottle: boolean;
  return (...args: Parameters<T>) => {
    if (!inThrottle) {
      func(...args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
}

export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function copyToClipboard(text: string): Promise<boolean> {
  if (navigator.clipboard && window.isSecureContext) {
    return navigator.clipboard.writeText(text).then(() => true).catch(() => false);
  } else {
    // Fallback for older browsers
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.style.position = 'fixed';
    textArea.style.left = '-999999px';
    textArea.style.top = '-999999px';
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
    return new Promise((resolve) => {
      document.execCommand('copy') ? resolve(true) : resolve(false);
      textArea.remove();
    });
  }
}

export function downloadFile(content: string, filename: string, contentType: string = 'text/plain') {
  const blob = new Blob([content], { type: contentType });
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  window.URL.revokeObjectURL(url);
}

export function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'An unknown error occurred';
}

export function parseApiResponse<T>(response: Response): Promise<T> {
  return response.json().then((data) => {
    if (!response.ok) {
      throw new Error(data.error || data.message || `HTTP error! status: ${response.status}`);
    }
    return data;
  });
}

export function buildQueryString(params: Record<string, string | number | boolean | undefined>): string {
  const searchParams = new URLSearchParams();
  for (const key in params) {
    const value = params[key];
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  return searchParams.toString();
}
