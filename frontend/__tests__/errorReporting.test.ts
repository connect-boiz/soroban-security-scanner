import { registerErrorHandler, reportError } from '../lib/errorReporting';

describe('errorReporting', () => {
  it('reports errors to all registered handlers', () => {
    const handler = jest.fn();
    registerErrorHandler(handler);

    const error = new Error('boom');
    reportError(error, 'ComponentStack', { userId: 42 });

    expect(handler).toHaveBeenCalledTimes(1);
    const report = handler.mock.calls[0][0];
    expect(report.message).toBe('boom');
    expect(report.stack).toBeDefined();
    expect(report.componentStack).toBe('ComponentStack');
    expect(report.context).toEqual({ userId: 42 });
    expect(report.timestamp).toBeDefined();
    expect(report.url).toBeDefined();
  });

  it('never lets a throwing handler crash the app', () => {
    const throwing = jest.fn(() => {
      throw new Error('handler failed');
    });
    const healthy = jest.fn();
    registerErrorHandler(throwing);
    registerErrorHandler(healthy);

    expect(() => reportError(new Error('boom'))).not.toThrow();
    expect(healthy).toHaveBeenCalledTimes(1);
  });
});
