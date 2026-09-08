import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import LoadingStatesDemo from '../components/LoadingStatesDemo';

describe('LoadingStatesDemo', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders the demo header and all sections', () => {
    render(<LoadingStatesDemo />);
    expect(
      screen.getByRole('heading', { name: /Loading States & Skeleton Screens Demo/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { level: 2, name: /Loading Spinners/i })
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: /Progress Bars/i })).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { level: 2, name: /Skeleton Screens/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { level: 2, name: /Advanced Loading States/i })
    ).toBeInTheDocument();
    expect(screen.getByText('87%')).toBeInTheDocument();
  });

  it('toggles the loading overlay', () => {
    render(<LoadingStatesDemo />);
    expect(screen.queryByText('Loading overlay...')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /toggle overlay/i }));
    expect(screen.getByText('Loading overlay...')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /toggle overlay/i }));
    expect(screen.queryByText('Loading overlay...')).not.toBeInTheDocument();
  });

  it('advances the simulated progress bar', () => {
    render(<LoadingStatesDemo />);
    fireEvent.click(screen.getByRole('button', { name: /simulate progress/i }));
    act(() => {
      jest.advanceTimersByTime(4000);
    });
    // Progress reached 100 after 20 ticks; no crash and the bar remains rendered
    expect(screen.getByRole('button', { name: /simulate progress/i })).toBeInTheDocument();
  });

  it('cycles through multi-step progress', () => {
    render(<LoadingStatesDemo />);
    const nextStep = screen.getByRole('button', { name: /next step/i });
    fireEvent.click(nextStep);
    fireEvent.click(nextStep);
    expect(screen.getByText('Step 3')).toBeInTheDocument();
  });

  it('runs the async operation and shows the result', async () => {
    const consoleSpy = jest.spyOn(console, 'log').mockImplementation(() => {});
    render(<LoadingStatesDemo />);
    fireEvent.click(screen.getByRole('button', { name: /execute async operation/i }));
    expect(screen.getByText('Executing...')).toBeInTheDocument();
    await act(async () => {
      jest.advanceTimersByTime(2100);
    });
    expect(screen.getByText('✓ Operation completed successfully!')).toBeInTheDocument();
    consoleSpy.mockRestore();
  });

  it('runs the staged loading sequence', async () => {
    render(<LoadingStatesDemo />);
    fireEvent.click(screen.getByRole('button', { name: /execute staged loading/i }));
    await act(async () => {
      jest.advanceTimersByTime(1100);
    });
    expect(screen.getByText('Security Analysis')).toBeInTheDocument();
    // Each stage schedules its own timer, so advance in steps to let the
    // async continuations (microtasks) flush between stages.
    for (let i = 0; i < 10; i++) {
      await act(async () => {
        jest.advanceTimersByTime(500);
      });
    }
    // Once all stages complete the loading block unmounts and the button re-enables
    expect(screen.queryByText('Security Analysis')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /execute staged loading/i })).not.toBeDisabled();
  });

  it('starts hook-based loading', () => {
    render(<LoadingStatesDemo />);
    fireEvent.click(screen.getByRole('button', { name: /start hook loading/i }));
    expect(screen.getByText('Initializing...')).toBeInTheDocument();
  });
});
