import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  InAppNotificationItem,
  InAppNotificationList,
  NotificationDropdown,
} from '../components/notifications/InAppNotification';
import { InAppNotification, NotificationPriority, NotificationType } from '../types/notifications';

const longMessage = 'This is a very long notification message that goes on and on. '.repeat(8);

const baseNotification: InAppNotification = {
  id: 'n1',
  title: 'Scan completed',
  message: 'Your scan finished successfully',
  type: NotificationType.SCAN_COMPLETED,
  priority: NotificationPriority.NORMAL,
  created_at: '2026-01-01T10:00:00Z',
};

const makeNotification = (overrides: Partial<InAppNotification> = {}): InAppNotification => ({
  ...baseNotification,
  ...overrides,
});

describe('InAppNotificationItem', () => {
  it('marks an unread notification as read on click', () => {
    const onRead = jest.fn();
    render(
      <InAppNotificationItem
        notification={makeNotification()}
        onRead={onRead}
        onDelete={jest.fn()}
      />
    );
    expect(screen.getByText('New')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Scan completed'));
    expect(onRead).toHaveBeenCalledWith('n1');
  });

  it('does not call onRead for already-read notifications', () => {
    const onRead = jest.fn();
    render(
      <InAppNotificationItem
        notification={makeNotification({ read_at: '2026-01-02T00:00:00Z' })}
        onRead={onRead}
        onDelete={jest.fn()}
      />
    );
    expect(screen.queryByText('New')).not.toBeInTheDocument();
    fireEvent.click(screen.getByText('Scan completed'));
    expect(onRead).not.toHaveBeenCalled();
  });

  it('truncates long messages and expands on demand', () => {
    render(
      <InAppNotificationItem
        notification={makeNotification({ message: longMessage })}
        onRead={jest.fn()}
        onDelete={jest.fn()}
      />
    );
    expect(screen.getByText('Show more')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Show more'));
    expect(screen.getByText('Show less')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Show less'));
    expect(screen.getByText('Show more')).toBeInTheDocument();
  });

  it('triggers the action via onAction callback', () => {
    const onAction = jest.fn();
    const action = { label: 'View details' };
    render(
      <InAppNotificationItem
        notification={makeNotification({ action })}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onAction={onAction}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /view details/i }));
    expect(onAction).toHaveBeenCalledWith('n1', action);
  });

  it('calls the action onClick when no onAction is provided', () => {
    const onClick = jest.fn();
    render(
      <InAppNotificationItem
        notification={makeNotification({ action: { label: 'Act', onClick } })}
        onRead={jest.fn()}
        onDelete={jest.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /act/i }));
    expect(onClick).toHaveBeenCalled();
  });

  it('opens the action url when neither onAction nor onClick exist', () => {
    const openSpy = jest.spyOn(window, 'open').mockImplementation(() => null);
    render(
      <InAppNotificationItem
        notification={makeNotification({ action: { label: 'Link', url: 'https://example.com' } })}
        onRead={jest.fn()}
        onDelete={jest.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /link/i }));
    expect(openSpy).toHaveBeenCalledWith('https://example.com', '_blank');
    openSpy.mockRestore();
  });

  it('deletes the notification', () => {
    const onDelete = jest.fn();
    render(
      <InAppNotificationItem
        notification={makeNotification()}
        onRead={jest.fn()}
        onDelete={onDelete}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /delete notification/i }));
    expect(onDelete).toHaveBeenCalledWith('n1');
  });

  it('shows the priority in uppercase', () => {
    render(
      <InAppNotificationItem
        notification={makeNotification({ priority: NotificationPriority.CRITICAL })}
        onRead={jest.fn()}
        onDelete={jest.fn()}
      />
    );
    expect(screen.getByText('CRITICAL')).toBeInTheDocument();
  });
});

describe('InAppNotificationList', () => {
  it('renders the empty state', () => {
    render(
      <InAppNotificationList
        notifications={[]}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    expect(screen.getByText('No notifications')).toBeInTheDocument();
  });

  it('shows the unread count and header actions', () => {
    const onMarkAllAsRead = jest.fn();
    const onClearAll = jest.fn();
    render(
      <InAppNotificationList
        notifications={[
          makeNotification({ id: 'a' }),
          makeNotification({ id: 'b' }),
          makeNotification({ id: 'c', read_at: '2026-01-02T00:00:00Z' }),
        ]}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={onMarkAllAsRead}
        onClearAll={onClearAll}
      />
    );
    expect(screen.getByText('2 unread')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /mark all as read/i }));
    expect(onMarkAllAsRead).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /clear all/i }));
    expect(onClearAll).toHaveBeenCalled();
  });
});

describe('NotificationDropdown', () => {
  const notifications = Array.from({ length: 12 }, (_, i) => makeNotification({ id: `n${i}` }));

  it('renders the bell with an unread badge', () => {
    render(
      <NotificationDropdown
        notifications={notifications}
        unreadCount={12}
        isOpen={false}
        onToggle={jest.fn()}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    expect(
      screen.getByRole('button', { name: /notifications \(12 unread\)/i })
    ).toBeInTheDocument();
  });

  it('caps the unread badge at 99+', () => {
    render(
      <NotificationDropdown
        notifications={notifications}
        unreadCount={150}
        isOpen={false}
        onToggle={jest.fn()}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    expect(screen.getByText('99+')).toBeInTheDocument();
  });

  it('opens and closes the dropdown on toggle', () => {
    const onToggle = jest.fn();
    const { rerender } = render(
      <NotificationDropdown
        notifications={notifications}
        unreadCount={12}
        isOpen={false}
        onToggle={onToggle}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /notifications/i }));
    expect(onToggle).toHaveBeenCalled();
    rerender(
      <NotificationDropdown
        notifications={notifications}
        unreadCount={12}
        isOpen={true}
        onToggle={onToggle}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    expect(screen.getByText('Notifications')).toBeInTheDocument();
    expect(screen.getByText('View all notifications (12)')).toBeInTheDocument();
    // Slice to max 10 in the dropdown
    expect(screen.getAllByText('Scan completed').length).toBeLessThanOrEqual(10);
  });

  it('shows no badge when unread count is zero', () => {
    render(
      <NotificationDropdown
        notifications={[]}
        unreadCount={0}
        isOpen={false}
        onToggle={jest.fn()}
        onRead={jest.fn()}
        onDelete={jest.fn()}
        onMarkAllAsRead={jest.fn()}
        onClearAll={jest.fn()}
      />
    );
    expect(screen.getByRole('button', { name: 'Notifications' })).toBeInTheDocument();
  });
});
