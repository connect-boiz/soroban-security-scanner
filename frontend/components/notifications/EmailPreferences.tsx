import React, { useState } from 'react';
import { 
  NotificationPreferences, 
  NotificationType, 
  NotificationPriority, 
  QuietHours,
  NotificationChannel 
} from '../../types/notifications';
import { cn, getNotificationTypeLabel, getPriorityLabel } from '../../lib/notifications/utils';

interface EmailPreferencesProps {
  preferences: NotificationPreferences;
  onUpdate: (preferences: Partial<NotificationPreferences>) => void;
  isLoading?: boolean;
}

export const EmailPreferencesPanel: React.FC<EmailPreferencesProps> = ({
  preferences,
  onUpdate,
  isLoading = false,
}) => {
  const [localPreferences, setLocalPreferences] = useState(preferences);

  const handleChannelToggle = (channel: NotificationChannel) => {
    const updated = {
      ...localPreferences,
      [`${channel}_enabled`]: !localPreferences[`${channel}_enabled` as keyof NotificationPreferences],
    };
    setLocalPreferences(updated);
    onUpdate(updated);
  };

  const handleNotificationTypeToggle = (type: NotificationType) => {
    const updated = {
      ...localPreferences,
      notification_types: {
        ...localPreferences.notification_types,
        [type]: !localPreferences.notification_types[type],
      },
    };
    setLocalPreferences(updated);
    onUpdate(updated);
  };

  const handleMaxPriorityChange = (priority: NotificationPriority) => {
    const updated = {
      ...localPreferences,
      max_priority: priority,
    };
    setLocalPreferences(updated);
    onUpdate(updated);
  };

  const handleQuietHoursChange = (quietHours: QuietHours | undefined) => {
    const updated = {
      ...localPreferences,
      quiet_hours: quietHours,
    };
    setLocalPreferences(updated);
    onUpdate(updated);
  };

  const handleQuietHoursToggle = () => {
    if (localPreferences.quiet_hours) {
      handleQuietHoursChange(undefined);
    } else {
      handleQuietHoursChange({
        start_hour: 22,
        end_hour: 8,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      });
    }
  };

  const handleQuietHoursTimeChange = (field: 'start_hour' | 'end_hour', value: number) => {
    if (!localPreferences.quiet_hours) return;
    
    const updated = {
      ...localPreferences.quiet_hours,
      [field]: value,
    };
    handleQuietHoursChange(updated);
  };

  return (
    <div className="space-y-6">
      {/* Channel Preferences */}
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Notification Channels</h3>
        <div className="space-y-3">
          <ChannelToggle
            label="Email Notifications"
            description="Receive notifications via email"
            enabled={localPreferences.email_enabled}
            onToggle={() => handleChannelToggle(NotificationChannel.EMAIL)}
            disabled={isLoading}
          />
          <ChannelToggle
            label="SMS Notifications"
            description="Receive notifications via SMS"
            enabled={localPreferences.sms_enabled}
            onToggle={() => handleChannelToggle(NotificationChannel.SMS)}
            disabled={isLoading}
          />
          <ChannelToggle
            label="Push Notifications"
            description="Receive browser push notifications"
            enabled={localPreferences.push_enabled}
            onToggle={() => handleChannelToggle(NotificationChannel.PUSH)}
            disabled={isLoading}
          />
          <ChannelToggle
            label="In-App Notifications"
            description="See notifications in the app"
            enabled={localPreferences.in_app_enabled}
            onToggle={() => handleChannelToggle(NotificationChannel.IN_APP)}
            disabled={isLoading}
          />
        </div>
      </div>

      {/* Notification Types */}
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Notification Types</h3>
        <div className="space-y-3">
          {Object.values(NotificationType).map((type) => (
            <ChannelToggle
              key={type}
              label={getNotificationTypeLabel(type)}
              description={`Receive ${getNotificationTypeLabel(type).toLowerCase()} notifications`}
              enabled={localPreferences.notification_types[type]}
              onToggle={() => handleNotificationTypeToggle(type)}
              disabled={isLoading}
            />
          ))}
        </div>
      </div>

      {/* Priority Settings */}
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Priority Settings</h3>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Minimum Priority Level
          </label>
          <p className="text-sm text-gray-500 mb-3">
            Only receive notifications at or above this priority level
          </p>
          <select
            value={localPreferences.max_priority}
            onChange={(e) => handleMaxPriorityChange(e.target.value as NotificationPriority)}
            disabled={isLoading}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            {Object.values(NotificationPriority).map((priority) => (
              <option key={priority} value={priority}>
                {getPriorityLabel(priority)}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Quiet Hours */}
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-gray-900">Quiet Hours</h3>
          <ChannelToggle
            label="Enable Quiet Hours"
            description="Limit notifications during specific hours"
            enabled={!!localPreferences.quiet_hours}
            onToggle={handleQuietHoursToggle}
            disabled={isLoading}
            compact
          />
        </div>

        {localPreferences.quiet_hours && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Start Time
                </label>
                <select
                  value={localPreferences.quiet_hours.start_hour}
                  onChange={(e) => handleQuietHoursTimeChange('start_hour', parseInt(e.target.value))}
                  disabled={isLoading}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  {Array.from({ length: 24 }, (_, i) => (
                    <option key={i} value={i}>
                      {i.toString().padStart(2, '0')}:00
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  End Time
                </label>
                <select
                  value={localPreferences.quiet_hours.end_hour}
                  onChange={(e) => handleQuietHoursTimeChange('end_hour', parseInt(e.target.value))}
                  disabled={isLoading}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  {Array.from({ length: 24 }, (_, i) => (
                    <option key={i} value={i}>
                      {i.toString().padStart(2, '0')}:00
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Timezone
              </label>
              <select
                value={localPreferences.quiet_hours.timezone}
                onChange={(e) => handleQuietHoursChange({
                  ...localPreferences.quiet_hours,
                  timezone: e.target.value,
                })}
                disabled={isLoading}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="UTC">UTC</option>
                <option value="America/New_York">Eastern Time</option>
                <option value="America/Chicago">Central Time</option>
                <option value="America/Denver">Mountain Time</option>
                <option value="America/Los_Angeles">Pacific Time</option>
                <option value="Europe/London">London</option>
                <option value="Europe/Paris">Paris</option>
                <option value="Asia/Tokyo">Tokyo</option>
                <option value="Asia/Shanghai">Shanghai</option>
              </select>
            </div>

            <div className="bg-blue-50 border border-blue-200 rounded-md p-3">
              <p className="text-sm text-blue-800">
                <strong>Note:</strong> Critical priority notifications will always be delivered, even during quiet hours.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Save Status */}
      {isLoading && (
        <div className="flex items-center justify-center py-4">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2"></div>
          <span className="text-sm text-gray-600">Saving preferences...</span>
        </div>
      )}
    </div>
  );
};

interface ChannelToggleProps {
  label: string;
  description?: string;
  enabled: boolean;
  onToggle: () => void;
  disabled?: boolean;
  compact?: boolean;
}

const ChannelToggle: React.FC<ChannelToggleProps> = ({
  label,
  description,
  enabled,
  onToggle,
  disabled = false,
  compact = false,
}) => {
  return (
    <div className={cn(
      'flex items-center',
      compact ? 'justify-between' : 'justify-between gap-3'
    )}>
      <div className={cn(compact ? 'flex-1' : 'flex-1')}>
        <label className={cn(
          'text-sm font-medium text-gray-900',
          compact ? 'cursor-pointer' : 'block'
        )}>
          {label}
        </label>
        {description && !compact && (
          <p className="text-sm text-gray-500 mt-1">{description}</p>
        )}
      </div>
      <button
        type="button"
        onClick={onToggle}
        disabled={disabled}
        className={cn(
          'relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2',
          enabled ? 'bg-blue-600' : 'bg-gray-200',
          disabled && 'opacity-50 cursor-not-allowed'
        )}
        role="switch"
        aria-checked={enabled}
      >
        <span className="sr-only">Toggle {label}</span>
        <span
          className={cn(
            'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
            enabled ? 'translate-x-5' : 'translate-x-0'
          )}
        />
      </button>
    </div>
  );
};

export default EmailPreferencesPanel;
