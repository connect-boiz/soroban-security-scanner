import type { Metadata, Viewport } from 'next';
import type { ReactNode } from 'react';
import { headers } from 'next/headers';
import './globals.css';
import { PageErrorBoundary } from '@/components/ui/ErrorBoundary';
import {
  getSiteUrl,
  siteDescription,
  siteKeywords,
  siteName,
  softwareApplicationJsonLd,
} from '@/lib/seo';

const siteUrl = getSiteUrl();

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: siteName,
    template: `%s | ${siteName}`,
  },
  description: siteDescription,
  applicationName: siteName,
  keywords: siteKeywords,
  authors: [{ name: 'Soroban Security Scanner contributors' }],
  creator: 'Soroban Security Scanner contributors',
  category: 'Security',
  alternates: {
    canonical: '/',
  },
  openGraph: {
    type: 'website',
    url: '/',
    title: siteName,
    description: siteDescription,
    siteName,
  },
  twitter: {
    card: 'summary_large_image',
    title: siteName,
    description: siteDescription,
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  minimumScale: 1,
  viewportFit: 'cover', // support iPhone notch / safe areas
};

// Known RTL locales
export const RTL_LOCALES = ['ar', 'he', 'fa', 'ur'] as const;
export type RtlLocale = (typeof RTL_LOCALES)[number];

export function isRtlLocale(locale?: string | null): boolean {
  if (!locale) return false;
  const lang = locale.toLowerCase().split('-')[0].split('_')[0].trim();
  return (RTL_LOCALES as readonly string[]).includes(lang);
}

export function getTextDirection(locale?: string | null): 'rtl' | 'ltr' {
  return isRtlLocale(locale) ? 'rtl' : 'ltr';
}

export function resolveLocale(headersList: { get(name: string): string | null }): string {
  // 1. Explicit headers (from proxy, API or middleware)
  const explicit = headersList.get('x-locale') || headersList.get('x-lang');
  if (explicit) return explicit;

  // 2. Cookie header (i18nextLng, NEXT_LOCALE, locale)
  const cookieHeader = headersList.get('cookie') || '';
  const match = cookieHeader.match(/(?:i18nextLng|NEXT_LOCALE|locale)=([^;]+)/);
  if (match) {
    try {
      return decodeURIComponent(match[1]);
    } catch {
      return match[1];
    }
  }

  // 3. Accept-Language header
  const acceptLang = headersList.get('accept-language');
  if (acceptLang) {
    const primary = acceptLang.split(',')[0].split(';')[0].trim();
    if (primary) return primary;
  }

  return 'en';
}

// Nonces are generated per request in middleware; opt out of static rendering.
export const dynamic = 'force-dynamic';

export default function RootLayout({ children }: { children: ReactNode }) {
  // Get the CSP nonce and locale from middleware / headers
  const headersList = headers();
  const nonce = headersList.get('x-nonce') || '';
  const resolvedLocale = resolveLocale(headersList);
  const lang = resolvedLocale.toLowerCase().split('-')[0].split('_')[0].trim() || 'en';
  const dir = isRtlLocale(lang) ? 'rtl' : 'ltr';

  return (
    <html lang={lang} dir={dir}>
      <body className={dir === 'rtl' ? 'rtl' : 'ltr'}>
        <script
          type="application/ld+json"
          nonce={nonce || undefined}
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              ...softwareApplicationJsonLd,
              url: siteUrl,
            }).replace(/</g, '\\u003c'),
          }}
        />
        <PageErrorBoundary>{children}</PageErrorBoundary>
      </body>
    </html>
  );
}
