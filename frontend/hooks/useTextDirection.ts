import { useState, useEffect } from 'react';

/**
 * List of known right-to-left language codes.
 */
export const RTL_LOCALES = ['ar', 'he', 'fa', 'ur'] as const;
export type RtlLocale = (typeof RTL_LOCALES)[number];
export type Direction = 'ltr' | 'rtl';

/**
 * Check if a language code corresponds to an RTL language.
 */
export function isRtlLocale(locale?: string | null): boolean {
  if (!locale) return false;
  const lang = locale.toLowerCase().split('-')[0].split('_')[0].trim();
  return (RTL_LOCALES as readonly string[]).includes(lang);
}

/**
 * Get the text direction ('rtl' or 'ltr') for a given locale string.
 */
export function getTextDirection(locale?: string | null): Direction {
  return isRtlLocale(locale) ? 'rtl' : 'ltr';
}

/**
 * React hook that returns 'ltr' or 'rtl' based on the active locale.
 *
 * It checks the provided override locale, or reads the direction from
 * document.documentElement.dir / document.documentElement.lang, or localStorage/cookies,
 * and reactively updates if the document direction or language changes.
 *
 * @param overrideLocale Optional locale string to resolve directly
 * @returns 'ltr' | 'rtl'
 */
export function useTextDirection(overrideLocale?: string): Direction {
  const [direction, setDirection] = useState<Direction>(() => {
    if (overrideLocale) {
      return getTextDirection(overrideLocale);
    }
    if (typeof document !== 'undefined') {
      const docDir = document.documentElement.dir;
      if (docDir === 'rtl' || docDir === 'ltr') {
        return docDir;
      }
      const docLang = document.documentElement.lang;
      if (docLang) {
        return getTextDirection(docLang);
      }
    }
    return 'ltr';
  });

  useEffect(() => {
    if (overrideLocale) {
      setDirection(getTextDirection(overrideLocale));
      return;
    }

    if (typeof document === 'undefined') return;

    const resolveCurrentDirection = (): Direction => {
      const docDir = document.documentElement.dir;
      if (docDir === 'rtl' || docDir === 'ltr') {
        return docDir;
      }
      const docLang = document.documentElement.lang;
      if (docLang) {
        return getTextDirection(docLang);
      }
      try {
        const storedLang = localStorage.getItem('i18nextLng');
        if (storedLang) {
          return getTextDirection(storedLang);
        }
      } catch {
        // Ignore localStorage restrictions
      }
      return 'ltr';
    };

    setDirection(resolveCurrentDirection());

    // Observe changes to html dir or lang attributes
    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        if (m.type === 'attributes' && (m.attributeName === 'dir' || m.attributeName === 'lang')) {
          setDirection(resolveCurrentDirection());
          break;
        }
      }
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['dir', 'lang'],
    });

    const handleStorage = (e: StorageEvent) => {
      if (e.key === 'i18nextLng') {
        setDirection(resolveCurrentDirection());
      }
    };
    window.addEventListener('storage', handleStorage);

    return () => {
      observer.disconnect();
      window.removeEventListener('storage', handleStorage);
    };
  }, [overrideLocale]);

  return direction;
}

export default useTextDirection;
