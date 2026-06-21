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

export default function RootLayout({ children }: { children: ReactNode }) {
  // Get the CSP nonce from middleware
  const headersList = headers();
  const nonce = headersList.get('x-nonce') || '';

  return (
    <html lang="en">
      <body>
        <script
          type="application/ld+json"
          nonce={nonce}
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
