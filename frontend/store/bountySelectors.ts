import { useMemo } from 'react';
import { useBountyStore } from './bountyStore';
import { shallow } from 'zustand/shallow';

// Optimized selectors using shallow comparison to prevent unnecessary re-renders
export const useFilteredBounties = () => {
  return (useBountyStore as any)((state: any) => state.getFilteredBounties(), shallow);
};

export const useBountyStats = () => {
  return (useBountyStore as any)((state: any) => state.getBountyStats(), shallow);
};

export const useBountyFilters = () => {
  return (useBountyStore as any)(
    (state: any) => ({
      filters: state.filters,
      searchTerm: state.searchTerm,
      setFilters: state.setFilters,
      setSearchTerm: state.setSearchTerm,
    }),
    shallow
  );
};

export const useBountyActions = () => {
  return (useBountyStore as any)(
    (state: any) => ({
      setBounties: state.setBounties,
      setSelectedBounty: state.setSelectedBounty,
      setLoading: state.setLoading,
      setError: state.setError,
      loading: state.loading,
      error: state.error,
    }),
    shallow
  );
};

export const useBountyById = (id: string) => {
  return useMemo(() => {
    return useBountyStore.getState().getBountyById(id);
  }, [id]);
};

export const useBountiesByCreator = (creator: string) => {
  return useMemo(() => {
    return useBountyStore.getState().getBountiesByCreator(creator);
  }, [creator]);
};

export const useBountiesByResearcher = (researcher: string) => {
  return useMemo(() => {
    return useBountyStore.getState().getBountiesByResearcher(researcher);
  }, [researcher]);
};
