/**
 * Root layout CSP nonce consumption tests
 *
 * Verifies that the layout reads the middleware-provided nonce and applies it
 * to inline scripts rendered in the document.
 */

import { render } from '@testing-library/react';
import { headers } from 'next/headers';
import RootLayout from '../app/layout';

jest.mock('next/headers', () => ({
  headers: jest.fn(),
}));

jest.mock('@/components/ui/ErrorBoundary', () => ({
  PageErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

describe('RootLayout CSP nonce consumption', () => {
  const mockHeadersGet = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    (headers as jest.Mock).mockReturnValue({
      get: mockHeadersGet,
    });
  });

  it('should apply nonce from x-nonce request header to inline scripts', () => {
    mockHeadersGet.mockImplementation((name: string) =>
      name === 'x-nonce' ? 'layout-test-nonce' : null
    );

    const { container } = render(
      <RootLayout>
        <div>child content</div>
      </RootLayout>
    );

    expect(mockHeadersGet).toHaveBeenCalledWith('x-nonce');

    const inlineScript = container.querySelector('script[type="application/ld+json"]');
    expect(inlineScript).not.toBeNull();
    expect(inlineScript).toHaveAttribute('nonce', 'layout-test-nonce');
  });

  it('should render inline script without nonce when x-nonce header is missing', () => {
    mockHeadersGet.mockImplementation(() => null);

    const { container } = render(
      <RootLayout>
        <div>child content</div>
      </RootLayout>
    );

    const inlineScript = container.querySelector('script[type="application/ld+json"]');
    expect(inlineScript).not.toBeNull();
    expect(inlineScript).not.toHaveAttribute('nonce');
  });

  it('should render inline script without nonce when x-nonce header is empty', () => {
    mockHeadersGet.mockImplementation((name: string) => (name === 'x-nonce' ? '' : null));

    const { container } = render(
      <RootLayout>
        <div>child content</div>
      </RootLayout>
    );

    const inlineScript = container.querySelector('script[type="application/ld+json"]');
    expect(inlineScript).not.toBeNull();
    expect(inlineScript).not.toHaveAttribute('nonce');
  });
});
