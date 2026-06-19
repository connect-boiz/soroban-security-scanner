import React, { useEffect } from 'react';
import dynamic from 'next/dynamic';
// @ts-ignore - react-joyride types may vary by version
import { STATUS, Step } from 'react-joyride';
import { useHelpStore } from '../../lib/store/helpStore';

// @ts-ignore - react-joyride dynamic import type
const Joyride = dynamic(() => import('react-joyride'), { ssr: false });
import { SCAN_TOUR_STEPS } from '../../lib/tours/scan-tour';
import { VULNERABILITY_TOUR_STEPS } from '../../lib/tours/vulnerability-tour';
import { TIME_TRAVEL_TOUR_STEPS } from '../../lib/tours/time-travel-tour';

interface GuidedTourProps {
  tourId: 'scan' | 'vulnerability' | 'time-travel';
}

/**
 * GuidedTour component - Manages the interactive walkthroughs using react-joyride.
 * Automatically starts the tour if it hasn't been completed before.
 */
const GuidedTour: React.FC<GuidedTourProps> = ({ tourId }) => {
  const { activeTour, completedTours, setActiveTour, markTourComplete } = useHelpStore();

  const tourSteps: Record<string, Step[]> = {
    scan: SCAN_TOUR_STEPS,
    vulnerability: VULNERABILITY_TOUR_STEPS,
    'time-travel': TIME_TRAVEL_TOUR_STEPS,
  };

  const steps = tourSteps[tourId] || [];

  useEffect(() => {
    // Check localStorage for completion state to persist across sessions
    const storageKey = `tourCompleted_${tourId}`;
    const isCompletedInStorage = localStorage.getItem(storageKey) === 'true';

    if (!completedTours.includes(tourId) && isCompletedInStorage) {
      markTourComplete(tourId);
    }

    // Auto-start tour if not completed and no other tour is active
    if (!completedTours.includes(tourId) && !isCompletedInStorage && activeTour === null) {
      setActiveTour(tourId);
    }
  }, [tourId, completedTours, activeTour, setActiveTour, markTourComplete]);

  const handleJoyrideCallback = (data: any) => {
    const { status } = data;
    const finishedStatuses: string[] = [STATUS.FINISHED, STATUS.SKIPPED];

    if (finishedStatuses.includes(status)) {
      markTourComplete(tourId);
      localStorage.setItem(`tourCompleted_${tourId}`, 'true');
      setActiveTour(null);
    }
  };

  const JoyrideComponent = Joyride as React.ComponentType<any>;

  return (
    <JoyrideComponent
      steps={steps}
      run={activeTour === tourId}
      continuous
      showProgress
      showSkipButton
      callback={handleJoyrideCallback}
      scrollToFirstStep
      disableScrolling={false}
      styles={{
        // @ts-ignore - Joyride style props compatibility
        options: {
          primaryColor: '#2563eb',
          zIndex: 10000,
          backgroundColor: '#ffffff',
          textColor: '#0f172a',
          arrowColor: '#ffffff',
        },
        tooltipContainer: {
          textAlign: 'left',
          borderRadius: '16px',
          padding: '10px',
        },
        buttonNext: {
          borderRadius: '8px',
          padding: '8px 16px',
          fontWeight: 'bold',
        },
        buttonBack: {
          marginRight: '10px',
          fontWeight: 'bold',
        },
        buttonSkip: {
          color: '#64748b',
        },
      }}
    />
  );
};

export default GuidedTour;
