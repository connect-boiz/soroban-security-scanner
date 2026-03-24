import { NotificationData } from '@/types/bounty';

export class NotificationService {
  private static instance: NotificationService;
  private subscribers: ((notifications: NotificationData[]) => void)[] = [];
  private notifications: NotificationData[] = [];
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;

  private constructor() {
    this.initializeWebSocket();
    this.loadStoredNotifications();
  }

  static getInstance(): NotificationService {
    if (!NotificationService.instance) {
      NotificationService.instance = new NotificationService();
    }
    return NotificationService.instance;
  }

  private initializeWebSocket() {
    try {
      // In production, this would connect to your actual WebSocket server
      const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3001/ws';
      
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;
      };

      this.ws.onmessage = (event) => {
        try {
          const notification: NotificationData = JSON.parse(event.data);
          this.addNotification(notification);
        } catch (error) {
          console.error('Failed to parse notification:', error);
        }
      };

      this.ws.onclose = () => {
        console.log('WebSocket disconnected');
        this.attemptReconnect();
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
      };
    } catch (error) {
      console.error('Failed to initialize WebSocket:', error);
      // Fallback to polling for demo purposes
      this.startPolling();
    }
  }

  private attemptReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.pow(2, this.reconnectAttempts) * 1000; // Exponential backoff
      
      setTimeout(() => {
        console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
        this.initializeWebSocket();
      }, delay);
    } else {
      console.log('Max reconnection attempts reached, falling back to polling');
      this.startPolling();
    }
  }

  private startPolling() {
    // Fallback polling mechanism
    setInterval(async () => {
      try {
        const response = await fetch('/api/notifications');
        const newNotifications: NotificationData[] = await response.json();
        
        newNotifications.forEach(notification => {
          if (!this.notifications.find(n => n.id === notification.id)) {
            this.addNotification(notification);
          }
        });
      } catch (error) {
        console.error('Polling error:', error);
      }
    }, 30000); // Poll every 30 seconds
  }

  private loadStoredNotifications() {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('bounty_notifications');
      if (stored) {
        this.notifications = JSON.parse(stored);
        this.notifySubscribers();
      }
    }
  }

  private saveNotifications() {
    if (typeof window !== 'undefined') {
      localStorage.setItem('bounty_notifications', JSON.stringify(this.notifications));
    }
  }

  private addNotification(notification: NotificationData) {
    this.notifications.unshift(notification);
    
    // Keep only last 50 notifications
    if (this.notifications.length > 50) {
      this.notifications = this.notifications.slice(0, 50);
    }

    this.saveNotifications();
    this.notifySubscribers();
    this.showBrowserNotification(notification);
  }

  private showBrowserNotification(notification: NotificationData) {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(notification.title, {
        body: notification.message,
        icon: '/favicon.ico',
        tag: notification.id
      });
    } else if ('Notification' in window && Notification.permission !== 'denied') {
      Notification.requestPermission().then(permission => {
        if (permission === 'granted') {
          new Notification(notification.title, {
            body: notification.message,
            icon: '/favicon.ico',
            tag: notification.id
          });
        }
      });
    }
  }

  private notifySubscribers() {
    this.subscribers.forEach(callback => callback(this.notifications));
  }

  subscribe(callback: (notifications: NotificationData[]) => void) {
    this.subscribers.push(callback);
    callback(this.notifications);
    
    // Return unsubscribe function
    return () => {
      this.subscribers = this.subscribers.filter(sub => sub !== callback);
    };
  }

  markAsRead(notificationId: string) {
    const notification = this.notifications.find(n => n.id === notificationId);
    if (notification) {
      notification.read = true;
      this.saveNotifications();
      this.notifySubscribers();
    }
  }

  markAllAsRead() {
    this.notifications.forEach(n => n.read = true);
    this.saveNotifications();
    this.notifySubscribers();
  }

  clearAll() {
    this.notifications = [];
    this.saveNotifications();
    this.notifySubscribers();
  }

  getUnreadCount(): number {
    return this.notifications.filter(n => !n.read).length;
  }

  // Simulate receiving notifications for demo purposes
  simulateNotification(type: NotificationData['type'], data?: any) {
    const notification: NotificationData = {
      id: `notif_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      type,
      title: this.getNotificationTitle(type, data),
      message: this.getNotificationMessage(type, data),
      timestamp: new Date(),
      read: false
    };

    this.addNotification(notification);
  }

  private getNotificationTitle(type: NotificationData['type'], data?: any): string {
    switch (type) {
      case 'new_bounty':
        return '🎯 New Bounty Posted';
      case 'submission_approved':
        return '✅ Submission Approved';
      case 'bounty_completed':
        return '🏆 Bounty Completed';
      case 'dispute_raised':
        return '⚠️ Dispute Raised';
      default:
        return '📢 Notification';
    }
  }

  private getNotificationMessage(type: NotificationData['type'], data?: any): string {
    switch (type) {
      case 'new_bounty':
        return `New ${data?.difficulty || 'Medium'} bounty "${data?.title || 'Security Audit'}" posted with ${data?.reward || '1000'} XLM reward`;
      case 'submission_approved':
        return `Your submission for "${data?.bountyTitle || 'Security Audit'}" has been approved!`;
      case 'bounty_completed':
        return `Bounty "${data?.bountyTitle || 'Security Audit'}" has been completed and rewards distributed`;
      case 'dispute_raised':
        return `A dispute has been raised for submission "${data?.submissionId || 'Unknown'}"`;
      default:
        return 'You have a new notification';
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}
