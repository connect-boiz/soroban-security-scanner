import { useHelpStore } from '../lib/store/helpStore';

describe('useHelpStore', () => {
  beforeEach(() => {
    useHelpStore.setState({ activeTour: null, completedTours: [], helpPanelTopic: null });
    localStorage.clear();
  });

  it('sets the active tour', () => {
    useHelpStore.getState().setActiveTour('scan-tour');
    expect(useHelpStore.getState().activeTour).toBe('scan-tour');
  });

  it('marks tours complete without duplicates', () => {
    useHelpStore.getState().markTourComplete('t1');
    useHelpStore.getState().markTourComplete('t1');
    expect(useHelpStore.getState().completedTours).toEqual(['t1']);
  });

  it('sets the help panel topic', () => {
    const topic = { id: 'rbac', title: 'RBAC' } as never;
    useHelpStore.getState().setHelpPanelTopic(topic);
    expect(useHelpStore.getState().helpPanelTopic).toBe(topic);
  });

  it('resets completed tours', () => {
    useHelpStore.getState().markTourComplete('t1');
    useHelpStore.getState().resetTours();
    expect(useHelpStore.getState().completedTours).toEqual([]);
  });
});
