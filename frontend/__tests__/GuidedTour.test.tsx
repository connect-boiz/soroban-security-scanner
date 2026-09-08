import React from 'react';
import { render, screen, act } from '@testing-library/react';
import GuidedTour from '../components/help/GuidedTour';
import { useHelpStore } from '../lib/store/helpStore';

// Mock the store
jest.mock('../lib/store/helpStore', () => ({
  useHelpStore: jest.fn(),
}));

// Capture Joyride props so tests can drive the callback, and export STATUS so
// the component's finished/skipped comparison works.
let joyrideProps: any = null;
jest.mock('react-joyride', () => {
  const React = require('react');
  const MockJoyride = (props: any) => {
    joyrideProps = props;
    return React.createElement('div', {
      'data-testid': 'mock-joyride',
      'data-run': props?.run ? 'true' : 'false',
    });
  };
  return {
    __esModule: true,
    default: MockJoyride,
    STATUS: { FINISHED: 'finished', SKIPPED: 'skipped' },
  };
});

describe('GuidedTour', () => {
  const mockSetActiveTour = jest.fn();
  const mockMarkTourComplete = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    joyrideProps = null;
    localStorage.clear();
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: null,
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });
  });

  it('renders Joyride when tour is active', async () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: 'scan',
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    expect(await screen.findByTestId('mock-joyride')).toBeInTheDocument();
  });

  it('auto-starts tour if not completed', () => {
    render(<GuidedTour tourId="scan" />);
    expect(mockSetActiveTour).toHaveBeenCalledWith('scan');
  });

  it('does not auto-start if already completed', () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: null,
      completedTours: ['scan'],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    expect(mockSetActiveTour).not.toHaveBeenCalled();
  });

  it('syncs the completion state from localStorage', () => {
    localStorage.setItem('tourCompleted_scan', 'true');
    render(<GuidedTour tourId="scan" />);
    expect(mockMarkTourComplete).toHaveBeenCalledWith('scan');
    expect(mockSetActiveTour).not.toHaveBeenCalled();
  });

  it('does not auto-start when another tour is active', () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: 'vulnerability',
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    expect(mockSetActiveTour).not.toHaveBeenCalled();
  });

  it('marks the tour complete when the user finishes it', async () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: 'scan',
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    await screen.findByTestId('mock-joyride');
    act(() => {
      joyrideProps.callback({ status: 'finished' });
    });
    expect(mockMarkTourComplete).toHaveBeenCalledWith('scan');
    expect(localStorage.getItem('tourCompleted_scan')).toBe('true');
    expect(mockSetActiveTour).toHaveBeenCalledWith(null);
  });

  it('marks the tour complete when skipped', async () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: 'scan',
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    await screen.findByTestId('mock-joyride');
    act(() => {
      joyrideProps.callback({ status: 'skipped' });
    });
    expect(mockMarkTourComplete).toHaveBeenCalledWith('scan');
  });
});
