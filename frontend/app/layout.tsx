import { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';
import { NotificationProvider } from '../hooks/notifications';
import ToastContainer from '../components/notifications/Toast';

const inter = Inter({ 
  subsets: ['latin'],
  display: 'swap',
  preload: true,
});

export const metadata: Metadata = {
  title: 'Soroban Security Scanner',
  description: 'Automated security scanner for Soroban smart contracts',
  keywords: ['soroban', 'stellar', 'security', 'smart-contracts', 'blockchain', 'audit'],
  authors: [{ name: 'connect-boiz' }],
  viewport: 'width=device-width, initial-scale=1',
  robots: 'index, follow',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={inter.className}>
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      </head>
      <body className="antialiased">
        <NotificationProvider>
          <div id="root">
            {children}
          </div>
          <ToastContainer />
        </NotificationProvider>
      </body>
    </html>
  );
}
