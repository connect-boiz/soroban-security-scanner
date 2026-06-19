import { useMemo } from 'react';
import { useBountyStore } from './bountyStore';

// Optimized selectors using shallow comparison to prevent unnecessary re-renders
export const useFilteredBounties = () => {
  return useBountyStore(state => state.getFilteredBounties());
};

export const useBountyStats = () => {
  return useBountyStore(state => state.getBountyStats());
};

export const useBountyFilters = () => {
  return useBountyStore(state => ({
    filters: state.filters,
    searchTerm: state.searchTerm,
    setFilters: state.setFilters,
    setSearchTerm: state.setSearchTerm,
  }));
};

export const useBountyActions = () => {
  return useBountyStore(state => ({
    setBounties: state.setBounties,
    setSelectedBounty: state.setSelectedBounty,
    setLoading: state.setLoading,
    setError: state.setError,
    loading: state.loading,
    error: state.error,
  }));
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
