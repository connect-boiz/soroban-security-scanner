import React from 'react';
import { render, screen } from '@testing-library/react';
import { headers } from 'next/headers';
import RootLayout, {
  RTL_LOCALES,
  isRtlLocale,
  getTextDirection,
  resolveLocale,
} from '../app/layout';
import {
  SkipLink,
  AccessibleFormField,
  getDirectionalAria,
  DirectionalNavigationButton,
  DirectionalToolbar,
} from '../components/accessibility/A11yPrimitives';
import {
  useTextDirection,
  isRtlLocale as isRtlLocaleHook,
  getTextDirection as getTextDirectionHook,
} from '../hooks/useTextDirection';

jest.mock('next/headers', () => ({
  headers: jest.fn(),
}));

jest.mock('@/components/ui/ErrorBoundary', () => ({
  PageErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

describe('RTL Layout and Direction Support', () => {
  const mockHeadersGet = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    (headers as jest.Mock).mockReturnValue({
      get: mockHeadersGet,
    });
  });

  describe('Locale & Direction Helpers', () => {
    it('identifies RTL locales accurately', () => {
      expect(isRtlLocale('ar')).toBe(true);
      expect(isRtlLocale('ar-SA')).toBe(true);
      expect(isRtlLocale('he')).toBe(true);
      expect(isRtlLocale('he-IL')).toBe(true);
      expect(isRtlLocale('fa')).toBe(true);
      expect(isRtlLocale('ur')).toBe(true);

      expect(isRtlLocale('en')).toBe(false);
      expect(isRtlLocale('es')).toBe(false);
      expect(isRtlLocale('fr')).toBe(false);
      expect(isRtlLocale(null)).toBe(false);
      expect(isRtlLocale(undefined)).toBe(false);
    });

    it('returns correct direction strings', () => {
      expect(getTextDirection('ar')).toBe('rtl');
      expect(getTextDirection('he')).toBe('rtl');
      expect(getTextDirection('en')).toBe('ltr');
      expect(getTextDirection('es')).toBe('ltr');
    });

    it('resolves locale from explicit headers, cookies, and Accept-Language', () => {
      // 1. Explicit header
      expect(resolveLocale({ get: (k: string) => (k === 'x-locale' ? 'ar' : null) })).toBe('ar');
      expect(resolveLocale({ get: (k: string) => (k === 'x-lang' ? 'he' : null) })).toBe('he');

      // 2. Cookie
      expect(
        resolveLocale({
          get: (k: string) => (k === 'cookie' ? 'session=xyz; i18nextLng=ar-EG; theme=dark' : null),
        })
      ).toBe('ar-EG');
      expect(
        resolveLocale({
          get: (k: string) => (k === 'cookie' ? 'NEXT_LOCALE=he' : null),
        })
      ).toBe('he');

      // 3. Accept-Language
      expect(
        resolveLocale({
          get: (k: string) => (k === 'accept-language' ? 'ar-SA,ar;q=0.9,en;q=0.8' : null),
        })
      ).toBe('ar-SA');

      // Fallback
      expect(resolveLocale({ get: () => null })).toBe('en');
    });
  });

  describe('RootLayout Server Component', () => {
    it('renders with dir="rtl" and lang="ar" when x-locale is Arabic', () => {
      mockHeadersGet.mockImplementation((key: string) => {
        if (key === 'x-locale') return 'ar';
        return null;
      });

      const { container } = render(
        <RootLayout>
          <div data-testid="child">Content</div>
        </RootLayout>
      );

      const html = container.querySelector('html');
      expect(html).not.toBeNull();
      expect(html).toHaveAttribute('dir', 'rtl');
      expect(html).toHaveAttribute('lang', 'ar');
    });

    it('renders with dir="rtl" and lang="he" when x-locale is Hebrew', () => {
      mockHeadersGet.mockImplementation((key: string) => {
        if (key === 'x-locale') return 'he';
        return null;
      });

      const { container } = render(
        <RootLayout>
          <div>Hebrew layout</div>
        </RootLayout>
      );

      const html = container.querySelector('html');
      expect(html).not.toBeNull();
      expect(html).toHaveAttribute('dir', 'rtl');
      expect(html).toHaveAttribute('lang', 'he');
    });

    it('renders with dir="ltr" and lang="en" by default', () => {
      mockHeadersGet.mockImplementation(() => null);

      const { container } = render(
        <RootLayout>
          <div>Default layout</div>
        </RootLayout>
      );

      const html = container.querySelector('html');
      expect(html).not.toBeNull();
      expect(html).toHaveAttribute('dir', 'ltr');
      expect(html).toHaveAttribute('lang', 'en');
    });
  });

  describe('useTextDirection hook', () => {
    it('returns rtl for Arabic and Hebrew and ltr for English', () => {
      expect(isRtlLocaleHook('ar')).toBe(true);
      expect(isRtlLocaleHook('he')).toBe(true);
      expect(getTextDirectionHook('ar')).toBe('rtl');
      expect(getTextDirectionHook('en')).toBe('ltr');
    });
  });

  describe('A11yPrimitives RTL extensions', () => {
    it('SkipLink swaps placement to right in RTL mode', () => {
      const { container: ltrContainer } = render(
        <SkipLink targetId="main-content" dir="ltr" />
      );
      const ltrLink = ltrContainer.querySelector('a');
      expect(ltrLink?.className).toContain('focus:left-4');

      const { container: rtlContainer } = render(
        <SkipLink targetId="main-content" dir="rtl" />
      );
      const rtlLink = rtlContainer.querySelector('a');
      expect(rtlLink?.className).toContain('focus:right-4');
    });

    it('AccessibleFormField swaps asterisk margin in RTL mode', () => {
      const { container: ltrContainer } = render(
        <AccessibleFormField label="Address" required dir="ltr">
          {({ inputProps }) => <input {...inputProps} />}
        </AccessibleFormField>
      );
      const ltrAsterisk = ltrContainer.querySelector('span[aria-hidden="true"]');
      expect(ltrAsterisk?.className).toContain('ml-1');

      const { container: rtlContainer } = render(
        <AccessibleFormField label="العنوان" required dir="rtl">
          {({ inputProps }) => <input {...inputProps} />}
        </AccessibleFormField>
      );
      const rtlAsterisk = rtlContainer.querySelector('span[aria-hidden="true"]');
      expect(rtlAsterisk?.className).toContain('mr-1');
      expect(rtlContainer.firstChild).toHaveAttribute('dir', 'rtl');
    });

    it('getDirectionalAria swaps ArrowLeft / ArrowRight and start/end placements', () => {
      const ltr = getDirectionalAria({ direction: 'ltr' });
      expect(ltr.previousKey).toBe('ArrowLeft');
      expect(ltr.nextKey).toBe('ArrowRight');
      expect(ltr.startPlacement).toBe('left');
      expect(ltr.endPlacement).toBe('right');
      expect(ltr.prevArrow).toBe('←');
      expect(ltr.nextArrow).toBe('→');

      const rtl = getDirectionalAria({ direction: 'rtl' });
      expect(rtl.previousKey).toBe('ArrowRight');
      expect(rtl.nextKey).toBe('ArrowLeft');
      expect(rtl.startPlacement).toBe('right');
      expect(rtl.endPlacement).toBe('left');
      expect(rtl.prevArrow).toBe('→');
      expect(rtl.nextArrow).toBe('←');
    });

    it('DirectionalNavigationButton flips arrow symbol in RTL mode', () => {
      render(<DirectionalNavigationButton direction="ltr" action="next" />);
      expect(screen.getByRole('button', { name: 'Next' })).toHaveTextContent('→');

      render(<DirectionalNavigationButton direction="rtl" action="next" />);
      const buttons = screen.getAllByRole('button', { name: 'Next' });
      expect(buttons[1]).toHaveTextContent('←');
    });

    it('DirectionalToolbar renders with flex-row-reverse in RTL mode', () => {
      const { container } = render(
        <DirectionalToolbar direction="rtl" aria-label="Editor controls">
          <button>1</button>
          <button>2</button>
        </DirectionalToolbar>
      );
      const toolbar = container.querySelector('[role="toolbar"]');
      expect(toolbar).not.toBeNull();
      expect(toolbar).toHaveAttribute('dir', 'rtl');
      expect(toolbar?.className).toContain('flex-row-reverse');
    });
  });
});
