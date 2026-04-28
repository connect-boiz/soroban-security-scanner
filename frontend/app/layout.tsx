import type { Metadata, Viewport } from 'next';
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
  return (
    <html lang="en">
      <body>
        <PageErrorBoundary>{children}</PageErrorBoundary>
      </body>
    </html>
  );
}
