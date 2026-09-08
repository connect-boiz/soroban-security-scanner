import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { EmailPreferencesPanel } from '../components/notifications/EmailPreferences';
import {
  NotificationPreferences,
  NotificationPriority,
  NotificationType,
  NotificationChannel,
} from '../types/notifications';

const basePrefs: NotificationPreferences = {
  email_enabled: true,
  sms_enabled: false,
  push_enabled: true,
  in_app_enabled: false,
  max_priority: NotificationPriority.NORMAL,
  notification_types: {
    [NotificationType.SCAN_COMPLETED]: true,
    [NotificationType.VULNERABILITY_FOUND]: false,
    [NotificationType.SCAN_FAILED]: true,
    [NotificationType.SECURITY_ALERT]: false,
    [NotificationType.SYSTEM_MAINTENANCE]: true,
    [NotificationType.ACCOUNT_UPDATE]: false,
    [NotificationType.BOUNTY_CLAIMED]: true,
    [NotificationType.ESCROW_RELEASED]: false,
  },
};

describe('EmailPreferencesPanel', () => {
  it('renders all channel toggles', () => {
    render(<EmailPreferencesPanel preferences={basePrefs} onUpdate={jest.fn()} />);
    expect(screen.getByText('Notification Channels')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: /toggle email notifications/i })).toHaveAttribute(
      'aria-checked',
      'true'
    );
    expect(screen.getByRole('switch', { name: /toggle sms notifications/i })).toHaveAttribute(
      'aria-checked',
      'false'
    );
    expect(screen.getByRole('switch', { name: /toggle push notifications/i })).toBeInTheDocument();
    expect(
      screen.getByRole('switch', { name: /toggle in-app notifications/i })
    ).toBeInTheDocument();
  });

  it('toggles a channel and reports the update', () => {
    const onUpdate = jest.fn();
    render(<EmailPreferencesPanel preferences={basePrefs} onUpdate={onUpdate} />);
    fireEvent.click(screen.getByRole('switch', { name: /toggle sms notifications/i }));
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ sms_enabled: true, email_enabled: true })
    );
  });

  it('toggles individual notification types', () => {
    const onUpdate = jest.fn();
    render(<EmailPreferencesPanel preferences={basePrefs} onUpdate={onUpdate} />);
    fireEvent.click(screen.getByRole('switch', { name: /toggle vulnerability found/i }));
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        notification_types: expect.objectContaining({
          [NotificationType.VULNERABILITY_FOUND]: true,
        }),
      })
    );
  });

  it('changes the minimum priority level', () => {
    const onUpdate = jest.fn();
    render(<EmailPreferencesPanel preferences={basePrefs} onUpdate={onUpdate} />);
    fireEvent.change(screen.getByDisplayValue('Normal'), { target: { value: 'critical' } });
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ max_priority: NotificationPriority.CRITICAL })
    );
  });

  it('enables quiet hours when they are off', () => {
    const onUpdate = jest.fn();
    render(
      <EmailPreferencesPanel
        preferences={{ ...basePrefs, quiet_hours: undefined }}
        onUpdate={onUpdate}
      />
    );
    fireEvent.click(screen.getByRole('switch', { name: /toggle enable quiet hours/i }));
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        quiet_hours: expect.objectContaining({ start_hour: 22, end_hour: 8 }),
      })
    );
  });

  it('disables quiet hours when they are on', () => {
    const onUpdate = jest.fn();
    render(
      <EmailPreferencesPanel
        preferences={{
          ...basePrefs,
          quiet_hours: { start_hour: 22, end_hour: 8, timezone: 'UTC' },
        }}
        onUpdate={onUpdate}
      />
    );
    expect(screen.getByText('Start Time')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: /toggle enable quiet hours/i }));
    expect(onUpdate).toHaveBeenCalledWith(expect.objectContaining({ quiet_hours: undefined }));
  });

  it('updates the quiet hours start time', () => {
    const onUpdate = jest.fn();
    render(
      <EmailPreferencesPanel
        preferences={{
          ...basePrefs,
          quiet_hours: { start_hour: 22, end_hour: 8, timezone: 'UTC' },
        }}
        onUpdate={onUpdate}
      />
    );
    fireEvent.change(screen.getByDisplayValue('22:00'), { target: { value: '23' } });
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ quiet_hours: expect.objectContaining({ start_hour: 23 }) })
    );
  });

  it('updates the quiet hours timezone', () => {
    const onUpdate = jest.fn();
    render(
      <EmailPreferencesPanel
        preferences={{
          ...basePrefs,
          quiet_hours: { start_hour: 22, end_hour: 8, timezone: 'UTC' },
        }}
        onUpdate={onUpdate}
      />
    );
    fireEvent.change(screen.getByDisplayValue('UTC'), { target: { value: 'Europe/Paris' } });
    expect(onUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        quiet_hours: expect.objectContaining({ timezone: 'Europe/Paris' }),
      })
    );
  });

  it('disables all controls and shows saving state while loading', () => {
    render(<EmailPreferencesPanel preferences={basePrefs} onUpdate={jest.fn()} isLoading />);
    expect(screen.getByText('Saving preferences...')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: /toggle email notifications/i })).toBeDisabled();
    expect(screen.getByDisplayValue('Normal')).toBeDisabled();
  });
});
