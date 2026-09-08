import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  ErrorBoundary,
  PageErrorBoundary,
  SectionErrorBoundary,
  InlineErrorBoundary,
} from '../components/ui/ErrorBoundary';
import { reportError } from '../lib/errorReporting';

jest.mock('../lib/errorReporting', () => ({
  reportError: jest.fn(),
}));

const reportErrorMock = reportError as jest.Mock;

/** Component that throws while `shouldThrow` is true. */
const createBoom = (getShouldThrow: () => boolean) =>
  function Boom() {
    if (getShouldThrow()) throw new Error('boom');
    return <div>recovered content</div>;
  };

describe('ErrorBoundary', () => {
  beforeEach(() => {
    reportErrorMock.mockClear();
  });

  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <div>healthy content</div>
      </ErrorBoundary>
    );
    expect(screen.getByText('healthy content')).toBeInTheDocument();
  });

  it('renders the section fallback and reports the error', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>
    );
    expect(screen.getByText('This section failed to load')).toBeInTheDocument();
    expect(reportErrorMock).toHaveBeenCalled();
  });

  it('resets the error state', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <ErrorBoundary variant="section">
        <Boom />
      </ErrorBoundary>
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
    shouldThrow = false;
    fireEvent.click(screen.getByRole('button', { name: /retry/i }));
    expect(screen.getByText('recovered content')).toBeInTheDocument();
  });

  it('renders the page fallback', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <PageErrorBoundary>
        <Boom />
      </PageErrorBoundary>
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument();
  });

  it('renders the inline fallback', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <InlineErrorBoundary>
        <Boom />
      </InlineErrorBoundary>
    );
    expect(screen.getByText(/Failed to render/i)).toBeInTheDocument();
  });

  it('supports a custom fallback render prop', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <ErrorBoundary
        fallback={({ error, resetError }) => (
          <div>
            <p>Custom fallback: {error.message}</p>
            <button onClick={resetError}>Reset me</button>
          </div>
        )}
      >
        <Boom />
      </ErrorBoundary>
    );
    expect(screen.getByText('Custom fallback: boom')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /reset me/i }));
  });

  it('calls the onError callback with error details', () => {
    const onError = jest.fn();
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <ErrorBoundary onError={onError} context={{ page: 'test' }}>
        <Boom />
      </ErrorBoundary>
    );
    expect(onError).toHaveBeenCalled();
    expect(reportErrorMock).toHaveBeenCalledWith(expect.any(Error), expect.any(String), {
      page: 'test',
    });
  });

  it('wraps sections with SectionErrorBoundary', () => {
    let shouldThrow = true;
    const Boom = createBoom(() => shouldThrow);
    render(
      <SectionErrorBoundary>
        <Boom />
      </SectionErrorBoundary>
    );
    expect(screen.getByText('This section failed to load')).toBeInTheDocument();
  });
});
