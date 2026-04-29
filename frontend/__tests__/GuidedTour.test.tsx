import React from 'react';
import { render, screen } from '@testing-library/react';
import GuidedTour from '../components/help/GuidedTour';
import { useHelpStore } from '../lib/store/helpStore';

// Mock the store
jest.mock('../lib/store/helpStore', () => ({
  useHelpStore: jest.fn(),
}));

describe('GuidedTour', () => {
  const mockSetActiveTour = jest.fn();
  const mockMarkTourComplete = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: null,
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });
  });

  it('renders Joyride when tour is active', () => {
    (useHelpStore as unknown as jest.Mock).mockReturnValue({
      activeTour: 'scan',
      completedTours: [],
      setActiveTour: mockSetActiveTour,
      markTourComplete: mockMarkTourComplete,
    });

    render(<GuidedTour tourId="scan" />);
    expect(screen.getByTestId('mock-joyride')).toBeInTheDocument();
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
});
