import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { HelpTopic } from '../help-content';

interface HelpState {
  activeTour: string | null;
  completedTours: string[];
  helpPanelTopic: HelpTopic | null;
  setActiveTour: (tourId: string | null) => void;
  markTourComplete: (tourId: string) => void;
  setHelpPanelTopic: (topic: HelpTopic | null) => void;
  resetTours: () => void;
}

/**
 * Zustand store for managing help-related state, including guided tours
 * and the help panel topic. Persistence is used for completed tours.
 */
export const useHelpStore = create<HelpState>()(
  persist(
    set => ({
      activeTour: null,
      completedTours: [],
      helpPanelTopic: null,
      setActiveTour: tourId => set({ activeTour: tourId }),
      markTourComplete: tourId =>
        set(state => ({
          completedTours: state.completedTours.includes(tourId)
            ? state.completedTours
            : [...state.completedTours, tourId],
        })),
      setHelpPanelTopic: topic => set({ helpPanelTopic: topic }),
      resetTours: () => set({ completedTours: [] }),
    }),
    {
      name: 'soroban-help-storage',
      // Only persist the completedTours array
      partialize: state => ({ completedTours: state.completedTours }),
    }
  )
);
