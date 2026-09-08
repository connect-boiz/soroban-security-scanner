import {
  cn,
  generateId,
  formatTimestamp,
  getNotificationIcon,
  getPriorityColor,
  getToastDuration,
  getNotificationTypeLabel,
  getPriorityLabel,
  validateEmail,
  validatePhone,
  truncateText,
  debounce,
  throttle,
  formatFileSize,
  copyToClipboard,
  downloadFile,
  isValidUrl,
  getErrorMessage,
  parseApiResponse,
  buildQueryString,
} from '../lib/notifications/utils';
import { NotificationPriority, NotificationType } from '../types/notifications';

describe('cn', () => {
  it('joins truthy values and skips falsy ones', () => {
    expect(cn('a', '', null, undefined, false, 'b')).toBe('a b');
    expect(cn()).toBe('');
  });
});

describe('generateId', () => {
  it('returns a 9-char string', () => {
    expect(generateId()).toHaveLength(9);
    expect(generateId()).not.toBe(generateId());
  });
});

describe('formatTimestamp', () => {
  const now = Date.now();
  it('returns "Just now" for < 1 minute', () => {
    expect(formatTimestamp(new Date(now - 30_000).toISOString())).toBe('Just now');
  });
  it('returns minutes ago (singular and plural)', () => {
    expect(formatTimestamp(new Date(now - 60_000).toISOString())).toBe('1 minute ago');
    expect(formatTimestamp(new Date(now - 5 * 60_000).toISOString())).toBe('5 minutes ago');
  });
  it('returns hours ago', () => {
    expect(formatTimestamp(new Date(now - 3 * 3_600_000).toISOString())).toBe('3 hours ago');
  });
  it('returns days ago for under a week', () => {
    expect(formatTimestamp(new Date(now - 2 * 86_400_000).toISOString())).toBe('2 days ago');
  });
  it('returns a locale date for older timestamps', () => {
    const date = new Date(now - 30 * 86_400_000);
    expect(formatTimestamp(date.toISOString())).toBe(date.toLocaleDateString());
  });
});

describe('notification icon/label/priority helpers', () => {
  it('maps every notification type to an icon', () => {
    expect(getNotificationIcon(NotificationType.SCAN_COMPLETED)).toBe('✅');
    expect(getNotificationIcon(NotificationType.VULNERABILITY_FOUND)).toBe('⚠️');
    expect(getNotificationIcon(NotificationType.SCAN_FAILED)).toBe('❌');
    expect(getNotificationIcon(NotificationType.SECURITY_ALERT)).toBe('🚨');
    expect(getNotificationIcon(NotificationType.SYSTEM_MAINTENANCE)).toBe('🔧');
    expect(getNotificationIcon(NotificationType.ACCOUNT_UPDATE)).toBe('👤');
    expect(getNotificationIcon(NotificationType.BOUNTY_CLAIMED)).toBe('💰');
    expect(getNotificationIcon(NotificationType.ESCROW_RELEASED)).toBe('🔓');
  });
  it('falls back for unknown types', () => {
    expect(getNotificationIcon('unknown' as NotificationType)).toBe('📢');
  });
  it('maps every priority to a color class', () => {
    expect(getPriorityColor(NotificationPriority.LOW)).toContain('gray');
    expect(getPriorityColor(NotificationPriority.NORMAL)).toContain('blue');
    expect(getPriorityColor(NotificationPriority.HIGH)).toContain('orange');
    expect(getPriorityColor(NotificationPriority.CRITICAL)).toContain('red');
    expect(getPriorityColor('unknown' as NotificationPriority)).toContain('gray');
  });
  it('maps every priority to a toast duration', () => {
    expect(getToastDuration(NotificationPriority.LOW)).toBe(3000);
    expect(getToastDuration(NotificationPriority.NORMAL)).toBe(5000);
    expect(getToastDuration(NotificationPriority.HIGH)).toBe(8000);
    expect(getToastDuration(NotificationPriority.CRITICAL)).toBe(0);
    expect(getToastDuration('unknown' as NotificationPriority)).toBe(5000);
  });
  it('labels notification types', () => {
    expect(getNotificationTypeLabel(NotificationType.SCAN_COMPLETED)).toBe('Scan Completed');
    expect(getNotificationTypeLabel(NotificationType.VULNERABILITY_FOUND)).toBe(
      'Vulnerability Found'
    );
    expect(getNotificationTypeLabel(NotificationType.SCAN_FAILED)).toBe('Scan Failed');
    expect(getNotificationTypeLabel(NotificationType.SECURITY_ALERT)).toBe('Security Alert');
    expect(getNotificationTypeLabel(NotificationType.SYSTEM_MAINTENANCE)).toBe(
      'System Maintenance'
    );
    expect(getNotificationTypeLabel(NotificationType.ACCOUNT_UPDATE)).toBe('Account Update');
    expect(getNotificationTypeLabel(NotificationType.BOUNTY_CLAIMED)).toBe('Bounty Claimed');
    expect(getNotificationTypeLabel(NotificationType.ESCROW_RELEASED)).toBe('Escrow Released');
    expect(getNotificationTypeLabel('unknown' as NotificationType)).toBe('Notification');
  });
  it('labels priorities', () => {
    expect(getPriorityLabel(NotificationPriority.LOW)).toBe('Low');
    expect(getPriorityLabel(NotificationPriority.NORMAL)).toBe('Normal');
    expect(getPriorityLabel(NotificationPriority.HIGH)).toBe('High');
    expect(getPriorityLabel(NotificationPriority.CRITICAL)).toBe('Critical');
    expect(getPriorityLabel('unknown' as NotificationPriority)).toBe('Normal');
  });
});

describe('validation helpers', () => {
  it('validates emails', () => {
    expect(validateEmail('a@b.com')).toBe(true);
    expect(validateEmail('not-an-email')).toBe(false);
  });
  it('validates phones', () => {
    expect(validatePhone('+1 555 123 4567')).toBe(true);
    expect(validatePhone('(555) 123-4567')).toBe(true);
    expect(validatePhone('123')).toBe(false);
  });
});

describe('text helpers', () => {
  it('truncates text', () => {
    expect(truncateText('short', 10)).toBe('short');
    expect(truncateText('a very long string', 10)).toBe('a very ...');
  });
  it('formats file sizes', () => {
    expect(formatFileSize(0)).toBe('0 Bytes');
    expect(formatFileSize(500)).toBe('500 Bytes');
    expect(formatFileSize(2048)).toBe('2 KB');
    expect(formatFileSize(5 * 1024 * 1024)).toBe('5 MB');
    expect(formatFileSize(3 * 1024 * 1024 * 1024)).toBe('3 GB');
  });
});

describe('debounce & throttle', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it('debounces trailing calls', () => {
    const fn = jest.fn();
    const debounced = debounce(fn, 100);
    debounced('a');
    debounced('b');
    jest.advanceTimersByTime(99);
    expect(fn).not.toHaveBeenCalled();
    jest.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('b');
  });

  it('throttles leading calls', () => {
    const fn = jest.fn();
    const throttled = throttle(fn, 100);
    throttled(1);
    throttled(2);
    expect(fn).toHaveBeenCalledTimes(1);
    jest.advanceTimersByTime(100);
    throttled(3);
    expect(fn).toHaveBeenCalledTimes(2);
  });
});

describe('clipboard & downloads', () => {
  it('copies via the async clipboard API', async () => {
    const writeText = jest.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    Object.defineProperty(window, 'isSecureContext', { value: true, configurable: true });
    await expect(copyToClipboard('hi')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('hi');
  });

  it('falls back when the async API fails', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: jest.fn().mockRejectedValue(new Error('denied')) },
      configurable: true,
    });
    Object.defineProperty(window, 'isSecureContext', { value: true, configurable: true });
    await expect(copyToClipboard('hi')).resolves.toBe(false);
  });

  it('downloads a file', () => {
    const click = jest.fn();
    const createObjectURL = jest.fn(() => 'blob:url');
    const revokeObjectURL = jest.fn();
    window.URL.createObjectURL = createObjectURL;
    window.URL.revokeObjectURL = revokeObjectURL;
    document.createElement = jest.fn((tag: string) => {
      const el = Object.assign(document.createElementNS('http://www.w3.org/1999/xhtml', tag), {
        click,
        href: '',
        download: '',
        style: {},
      });
      return el as unknown as HTMLElement;
    }) as unknown as typeof document.createElement;
    downloadFile('content', 'file.txt', 'text/plain');
    expect(click).toHaveBeenCalled();
    expect(createObjectURL).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalled();
  });
});

describe('url & error helpers', () => {
  it('validates URLs', () => {
    expect(isValidUrl('https://example.com')).toBe(true);
    expect(isValidUrl('not a url')).toBe(false);
  });
  it('extracts error messages', () => {
    expect(getErrorMessage(new Error('boom'))).toBe('boom');
    expect(getErrorMessage('string error')).toBe('string error');
    expect(getErrorMessage({})).toBe('An unknown error occurred');
  });
});

describe('parseApiResponse', () => {
  it('resolves data for ok responses', async () => {
    const response = { ok: true, json: async () => ({ id: 1 }) } as unknown as Response;
    await expect(parseApiResponse(response)).resolves.toEqual({ id: 1 });
  });
  it('throws with a server message for failed responses', async () => {
    const response = {
      ok: false,
      status: 500,
      json: async () => ({ error: 'server exploded' }),
    } as unknown as Response;
    await expect(parseApiResponse(response)).rejects.toThrow('server exploded');
  });
  it('throws with a status-based message when no body message exists', async () => {
    const response = {
      ok: false,
      status: 404,
      json: async () => ({}),
    } as unknown as Response;
    await expect(parseApiResponse(response)).rejects.toThrow('HTTP error! status: 404');
  });
});

describe('buildQueryString', () => {
  it('builds query strings, skipping undefined/null', () => {
    expect(buildQueryString({ a: 'x', b: 1, c: true, d: undefined })).toBe('a=x&b=1&c=true');
    expect(buildQueryString({})).toBe('');
  });
});
