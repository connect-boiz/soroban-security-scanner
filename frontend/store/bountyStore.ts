import { create } from 'zustand';
import { shallow } from 'zustand/shallow';

interface BountyStore {
  bounties: any[];
  selectedBounty: any;
  loading: boolean;
  error: string | null;
  filters: any;
  searchTerm: string;
  getFilteredBounties: () => any[];
  getBountyStats: () => any;
  getBountyById: (id: string) => any;
  getBountiesByCreator: (creator: string) => any[];
  getBountiesByResearcher: (researcher: string) => any[];
  setBounties: (bounties: any[]) => void;
  setSelectedBounty: (bounty: any) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setFilters: (filters: any) => void;
  setSearchTerm: (term: string) => void;
}

export const useBountyStore = create<BountyStore>()((set, get) => ({
  bounties: [],
  selectedBounty: null,
  loading: false,
  error: null,
  filters: {},
  searchTerm: '',
  getFilteredBounties: () => [],
  getBountyStats: () => ({}),
  getBountyById: (id: string) => null,
  getBountiesByCreator: (creator: string) => [],
  getBountiesByResearcher: (researcher: string) => [],
  setBounties: bounties => set({ bounties }),
  setSelectedBounty: bounty => set({ selectedBounty: bounty }),
  setLoading: loading => set({ loading }),
  setError: error => set({ error }),
  setFilters: filters => set({ filters }),
  setSearchTerm: searchTerm => set({ searchTerm }),
}));
