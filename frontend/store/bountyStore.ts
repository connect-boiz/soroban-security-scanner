import { create } from 'zustand';
import { Bounty, FilterOptions, Researcher, BountySubmission } from '@/types/bounty';

interface BountyStore {
  bounties: Bounty[];
  filteredBounties: Bounty[];
  filters: FilterOptions;
  selectedBounty: Bounty | null;
  researchers: Researcher[];
  submissions: BountySubmission[];
  loading: boolean;
  error: string | null;
  
  // Actions
  setBounties: (bounties: Bounty[]) => void;
  setFilteredBounties: (bounties: Bounty[]) => void;
  setFilters: (filters: Partial<FilterOptions>) => void;
  setSelectedBounty: (bounty: Bounty | null) => void;
  setResearchers: (researchers: Researcher[]) => void;
  setSubmissions: (submissions: BountySubmission[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  
  // Computed
  applyFilters: () => void;
  getBountyById: (id: string) => Bounty | undefined;
  getBountiesByCreator: (creator: string) => Bounty[];
  getBountiesByResearcher: (researcher: string) => Bounty[];
}

export const useBountyStore = create<BountyStore>((set, get) => ({
  bounties: [],
  filteredBounties: [],
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

  setBounties: (bounties) => set({ bounties }),
  
  setFilteredBounties: (filteredBounties) => set({ filteredBounties }),
  
  setFilters: (newFilters) => {
    set((state) => ({
      filters: { ...state.filters, ...newFilters }
    }));
    get().applyFilters();
  },
  
  setSelectedBounty: (selectedBounty) => set({ selectedBounty }),
  
  setResearchers: (researchers) => set({ researchers }),
  
  setSubmissions: (submissions) => set({ submissions }),
  
  setLoading: (loading) => set({ loading }),
  
  setError: (error) => set({ error }),

  applyFilters: () => {
    const { bounties, filters } = get();
    
    const filtered = bounties.filter(bounty => {
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
    
    set({ filteredBounties: filtered });
  },

  getBountyById: (id) => {
    return get().bounties.find(bounty => bounty.id === id);
  },

  getBountiesByCreator: (creator) => {
    return get().bounties.filter(bounty => bounty.creator === creator);
  },

  getBountiesByResearcher: (researcher) => {
    return get().bounties.filter(bounty => bounty.assignedResearcher === researcher);
  }
}));
