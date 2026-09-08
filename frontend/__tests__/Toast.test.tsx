import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { Toast, ToastContainer } from '../components/notifications/Toast';
import { NotificationPriority, NotificationType, ToastNotification } from '../types/notifications';

const baseToast: ToastNotification = {
  id: 'toast-1',
  type: NotificationType.VULNERABILITY_FOUND,
  title: 'Vulnerability found',
  message: 'A critical issue was detected',
  priority: NotificationPriority.CRITICAL,
  timestamp: '2026-01-01T00:00:00Z',
  read: false,
};

describe('Toast', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders title, message and icon', () => {
    render(<Toast toast={baseToast} onClose={jest.fn()} />);
    expect(screen.getByText('Vulnerability found')).toBeInTheDocument();
    expect(screen.getByText('A critical issue was detected')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('alert').getAttribute('aria-live')).toBe('assertive');
  });

  it('uses polite live region for non-critical priorities', () => {
    render(
      <Toast toast={{ ...baseToast, priority: NotificationPriority.NORMAL }} onClose={jest.fn()} />
    );
    expect(screen.getByRole('alert').getAttribute('aria-live')).toBe('polite');
  });

  it('calls onClose after the dismissible duration elapses', () => {
    const onClose = jest.fn();
    render(<Toast toast={{ ...baseToast, duration: 1000 }} onClose={onClose} />);
    act(() => {
      jest.advanceTimersByTime(1300);
    });
    expect(onClose).toHaveBeenCalledWith('toast-1');
  });

  it('does not auto-dismiss when duration is not set', () => {
    const onClose = jest.fn();
    render(<Toast toast={baseToast} onClose={onClose} />);
    act(() => {
      jest.advanceTimersByTime(5000);
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes when the close button is clicked', () => {
    const onClose = jest.fn();
    render(<Toast toast={baseToast} onClose={onClose} />);
    fireEvent.click(screen.getByRole('button', { name: /close notification/i }));
    act(() => {
      jest.advanceTimersByTime(300);
    });
    expect(onClose).toHaveBeenCalledWith('toast-1');
  });

  it('hides the close button when dismissible is false', () => {
    render(<Toast toast={{ ...baseToast, dismissible: false }} onClose={jest.fn()} />);
    expect(screen.queryByRole('button', { name: /close notification/i })).not.toBeInTheDocument();
  });

  it('renders the action button and fires its handler', () => {
    const onAction = jest.fn();
    render(
      <Toast
        toast={{
          ...baseToast,
          action: { label: 'View details', onClick: onAction },
        }}
        onClose={jest.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /view details/i }));
    expect(onAction).toHaveBeenCalled();
  });

  it('renders without a message', () => {
    const { message: _omitted, ...withoutMessage } = baseToast;
    render(<Toast toast={withoutMessage} onClose={jest.fn()} />);
    expect(screen.getByText('Vulnerability found')).toBeInTheDocument();
  });
});

describe('ToastContainer', () => {
  it('returns null when there are no toasts', () => {
    const { container } = render(<ToastContainer toasts={[]} onClose={jest.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders one Toast per entry', () => {
    render(
      <ToastContainer
        toasts={[baseToast, { ...baseToast, id: 'toast-2', title: 'Second toast' }]}
        onClose={jest.fn()}
      />
    );
    expect(screen.getByText('Vulnerability found')).toBeInTheDocument();
    expect(screen.getByText('Second toast')).toBeInTheDocument();
  });
});
