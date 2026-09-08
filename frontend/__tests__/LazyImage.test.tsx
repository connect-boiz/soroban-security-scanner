import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import LazyImage from '../components/LazyImage';

class MockIntersectionObserver {
  static instances: MockIntersectionObserver[] = [];
  callback: IntersectionObserverCallback;
  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
    MockIntersectionObserver.instances.push(this);
  }
  observe() {}
  unobserve() {}
  disconnect() {}
}

describe('LazyImage', () => {
  const originalObserver = global.IntersectionObserver;

  beforeEach(() => {
    (global as any).IntersectionObserver = MockIntersectionObserver;
    MockIntersectionObserver.instances = [];
  });

  afterEach(() => {
    (global as any).IntersectionObserver = originalObserver;
  });

  it('renders a placeholder until the image loads', () => {
    render(<LazyImage src="/img.png" alt="A screenshot" />);
    const img = screen.getByAltText('A screenshot') as HTMLImageElement;
    expect(img).toBeInTheDocument();
    expect(img.src).toBe('');
    expect(img.getAttribute('sizes')).toContain('(max-width: 640px) 320px');
  });

  it('loads the image when it enters the viewport and fires onLoad', () => {
    const onLoad = jest.fn();
    render(<LazyImage src="/img.png" alt="A screenshot" onLoad={onLoad} />);
    act(() => {
      MockIntersectionObserver.instances[0].callback(
        [{ isIntersecting: true }] as any,
        {} as IntersectionObserver
      );
    });
    const img = screen.getByAltText('A screenshot') as HTMLImageElement;
    expect(img.src).toContain('/img.png');
    expect(img.srcset).toContain('320w');
    fireEvent.load(img);
    expect(onLoad).toHaveBeenCalled();
  });

  it('shows the error state and fires onError', () => {
    const onError = jest.fn();
    render(<LazyImage src="/broken.png" alt="Broken" onError={onError} />);
    fireEvent.error(screen.getByAltText('Broken'));
    expect(screen.getByText('Failed to load image')).toBeInTheDocument();
    expect(onError).toHaveBeenCalled();
  });
});
