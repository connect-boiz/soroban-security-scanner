import { create } from 'zustand';
import { shallow } from 'zustand/shallow';
import { Bounty, FilterOptions, Researcher, BountySubmission } from '@/types/bounty';
import { useMemo } from 'react';

interface BountyStore {
  // State
  bounties: Bounty[];
  filters: FilterOptions;
  selectedBounty: Bounty | null;
  researchers: Researcher[];
  submissions: BountySubmission[];
  loading: boolean;
  error: string | null;
  searchTerm: string;
  
  // Actions
  setBounties: (bounties: Bounty[]) => void;
  setFilters: (filters: Partial<FilterOptions>) => void;
  setSearchTerm: (term: string) => void;
  setSelectedBounty: (bounty: Bounty | null) => void;
  setResearchers: (researchers: Researcher[]) => void;
  setSubmissions: (submissions: BountySubmission[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  
  // Computed selectors (memoized)
  getFilteredBounties: () => Bounty[];
  getBountyById: (id: string) => Bounty | undefined;
  getBountiesByCreator: (creator: string) => Bounty[];
  getBountiesByResearcher: (researcher: string) => Bounty[];
  getBountyStats: () => { total: number; active: number; completed: number; totalReward: number };
}

export const useBountyStore = create<BountyStore>((set, get) => ({
  // Initial state
  bounties: [],
  filters: {
    minReward: 0,
    maxReward: 10000,
    difficulty: [],
    status: [],
    tags: []
  },
  selectedBounty: null,
  researchers: [],
  submissions: [],
  loading: false,
  error: null,
  searchTerm: '',

  // Actions
  setBounties: (bounties) => set({ bounties }),
  
  setFilters: (newFilters) => {
    set((state) => ({
      filters: { ...state.filters, ...newFilters }
    }));
  },
  
  setSearchTerm: (term) => set({ searchTerm: term }),
  
  setSelectedBounty: (selectedBounty) => set({ selectedBounty }),
  
  setResearchers: (researchers) => set({ researchers }),
  
  setSubmissions: (submissions) => set({ submissions }),
  
  setLoading: (loading) => set({ loading }),
  
  setError: (error) => set({ error }),

  // Memoized selectors
  getFilteredBounties: () => {
    const { bounties, filters, searchTerm } = get();
    
    return bounties.filter(bounty => {
      // Search term filter
      if (searchTerm) {
        const searchLower = searchTerm.toLowerCase();
        const matchesSearch = 
          bounty.title.toLowerCase().includes(searchLower) ||
          bounty.description.toLowerCase().includes(searchLower) ||
          bounty.tags.some(tag => tag.toLowerCase().includes(searchLower));
        
        if (!matchesSearch) return false;
      }
      
      // Reward filter
      if (bounty.rewardAmount < filters.minReward || bounty.rewardAmount > filters.maxReward) {
        return false;
      }
      
      // Difficulty filter
      if (filters.difficulty.length > 0 && !filters.difficulty.includes(bounty.difficulty)) {
        return false;
      }
      
      // Status filter
      if (filters.status.length > 0 && !filters.status.includes(bounty.status)) {
        return false;
      }
      
      // Tags filter
      if (filters.tags.length > 0) {
        const hasMatchingTag = filters.tags.some(tag => 
          bounty.tags.some(bountyTag => 
            bountyTag.toLowerCase().includes(tag.toLowerCase())
          )
        );
        if (!hasMatchingTag) return false;
      }
      
      return true;
    });
  },

  getBountyById: (id) => {
    const { bounties } = get();
    return bounties.find(bounty => bounty.id === id);
  },

  getBountiesByCreator: (creator) => {
    const { bounties } = get();
    return bounties.filter(bounty => bounty.creator === creator);
  },

  getBountiesByResearcher: (researcher) => {
    const { bounties } = get();
    return bounties.filter(bounty => bounty.assignedResearcher === researcher);
  },
  
  getBountyStats: () => {
    const { bounties } = get();
    return {
      total: bounties.length,
      active: bounties.filter(b => b.status === 'Active').length,
      completed: bounties.filter(b => b.status === 'Completed').length,
      totalReward: bounties.reduce((sum, b) => sum + b.rewardAmount, 0)
    };
  }
}));
