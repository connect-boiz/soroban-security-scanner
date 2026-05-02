import type { Metadata, Viewport } from 'next';
import { headers } from 'next/headers';
import './globals.css';
import { PageErrorBoundary } from '@/components/ui/ErrorBoundary';

export const metadata: Metadata = {
  title: 'Soroban Security Scanner',
  description: 'Smart contract security analysis for the Soroban ecosystem',
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  minimumScale: 1,
  viewportFit: 'cover', // support iPhone notch / safe areas
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  // Get the CSP nonce from middleware
  const headersList = headers();
  const nonce = headersList.get('x-nonce') || '';

  return (
    <html lang="en">
      <body>
        <PageErrorBoundary>{children}</PageErrorBoundary>
      </body>
    </html>
  );
}
