'use client';

import { useState, useEffect } from 'react';
import { useBountyStore } from '@/store/bountyStore';
import { Bounty, FilterOptions } from '@/types/bounty';
import { CountdownTimer } from './CountdownTimer';
import { Search, Filter, DollarSign, Clock, User, AlertCircle } from 'lucide-react';

const BountyBoard: React.FC = () => {
  const {
    filteredBounties,
    filters,
    setFilters,
    setBounties,
    loading,
    error
  } = useBountyStore();

  const [showFilters, setShowFilters] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  // Mock data for development
  useEffect(() => {
    const mockBounties: Bounty[] = [
      {
        id: '1',
        title: 'Critical Vulnerability in Token Contract',
        description: 'Find potential reentrancy vulnerabilities in our DeFi token contract',
        rewardAmount: 5000,
        difficulty: 'Critical',
        status: 'Active',
        creator: 'GDU5K...3F7H',
        createdAt: new Date('2024-01-15'),
        deadline: new Date('2024-02-15'),
        contractAddress: 'CC7Z5...2M8N',
        firstToFind: true,
        tags: ['reentrancy', 'defi', 'token'],
        severity: 'Critical'
      },
      {
        id: '2',
        title: 'Access Control Audit Required',
        description: 'Review access control mechanisms in our staking contract',
        rewardAmount: 2000,
        difficulty: 'Medium',
        status: 'Active',
        creator: 'GDABC...9XYZ',
        createdAt: new Date('2024-01-10'),
        deadline: new Date('2024-02-10'),
        contractAddress: 'CA8B2...5K9L',
        firstToFind: false,
        tags: ['access-control', 'staking'],
        severity: 'High'
      },
      {
        id: '3',
        title: 'Smart Contract Gas Optimization',
        description: 'Identify gas optimization opportunities in our trading contract',
        rewardAmount: 1500,
        difficulty: 'Easy',
        status: 'InReview',
        creator: 'GDEF1...4MNO',
        assignedResearcher: 'GHIJK...6PQR',
        createdAt: new Date('2024-01-05'),
        deadline: new Date('2024-01-30'),
        contractAddress: 'CB3D9...8WXY',
        firstToFind: false,
        tags: ['gas-optimization', 'trading'],
        severity: 'Medium'
      }
    ];

    setBounties(mockBounties);
  }, [setBounties]);

  const handleFilterChange = (key: keyof FilterOptions, value: any) => {
    setFilters({ [key]: value });
  };

  const getDifficultyColor = (difficulty: string) => {
    switch (difficulty) {
      case 'Critical': return 'text-red-600 bg-red-50 border-red-200';
      case 'Hard': return 'text-orange-600 bg-orange-50 border-orange-200';
      case 'Medium': return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      case 'Easy': return 'text-green-600 bg-green-50 border-green-200';
      default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Active': return 'text-green-600 bg-green-50 border-green-200';
      case 'InReview': return 'text-blue-600 bg-blue-50 border-blue-200';
      case 'Approved': return 'text-purple-600 bg-purple-50 border-purple-200';
      case 'Rejected': return 'text-red-600 bg-red-50 border-red-200';
      case 'Completed': return 'text-gray-600 bg-gray-50 border-gray-200';
      default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const filteredBountiesWithSearch = filteredBounties.filter(bounty =>
    bounty.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
    bounty.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
    bounty.tags.some(tag => tag.toLowerCase().includes(searchTerm.toLowerCase()))
  );

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center text-red-600 p-8">
        <AlertCircle className="h-12 w-12 mx-auto mb-4" />
        <p>Error loading bounties: {error}</p>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto p-6">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900 mb-2">🏆 Bounty Board</h1>
        <p className="text-gray-600">Find and participate in security bounties for Stellar smart contracts</p>
      </div>

      {/* Search and Filters */}
      <div className="mb-6 space-y-4">
        {/* Search Bar */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 h-5 w-5" />
          <input
            type="text"
            placeholder="Search bounties by title, description, or tags..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-10 pr-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          />
        </div>

        {/* Filter Toggle */}
        <button
          onClick={() => setShowFilters(!showFilters)}
          className="flex items-center space-x-2 btn-secondary"
        >
          <Filter className="h-4 w-4" />
          <span>Filters</span>
          {Object.values(filters).some(v => 
            Array.isArray(v) ? v.length > 0 : v !== 0 && v !== 10000
          ) && (
            <span className="bg-primary-600 text-white text-xs px-2 py-1 rounded-full">Active</span>
          )}
        </button>

        {/* Filter Panel */}
        {showFilters && (
          <div className="card">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              {/* Reward Range */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  <DollarSign className="inline h-4 w-4 mr-1" />
                  Reward Range (XLM)
                </label>
                <div className="flex space-x-2">
                  <input
                    type="number"
                    placeholder="Min"
                    value={filters.minReward}
                    onChange={(e) => handleFilterChange('minReward', Number(e.target.value))}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                  />
                  <input
                    type="number"
                    placeholder="Max"
                    value={filters.maxReward}
                    onChange={(e) => handleFilterChange('maxReward', Number(e.target.value))}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                  />
                </div>
              </div>

              {/* Difficulty */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">Difficulty</label>
                <div className="space-y-2">
                  {['Critical', 'Hard', 'Medium', 'Easy'].map(difficulty => (
                    <label key={difficulty} className="flex items-center">
                      <input
                        type="checkbox"
                        checked={filters.difficulty.includes(difficulty)}
                        onChange={(e) => {
                          const newDifficulty = e.target.checked
                            ? [...filters.difficulty, difficulty]
                            : filters.difficulty.filter(d => d !== difficulty);
                          handleFilterChange('difficulty', newDifficulty);
                        }}
                        className="mr-2"
                      />
                      <span className={`px-2 py-1 rounded-full text-xs border ${getDifficultyColor(difficulty)}`}>
                        {difficulty}
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Status */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">Status</label>
                <div className="space-y-2">
                  {['Active', 'InReview', 'Approved', 'Completed'].map(status => (
                    <label key={status} className="flex items-center">
                      <input
                        type="checkbox"
                        checked={filters.status.includes(status)}
                        onChange={(e) => {
                          const newStatus = e.target.checked
                            ? [...filters.status, status]
                            : filters.status.filter(s => s !== status);
                          handleFilterChange('status', newStatus);
                        }}
                        className="mr-2"
                      />
                      <span className={`px-2 py-1 rounded-full text-xs border ${getStatusColor(status)}`}>
                        {status}
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Clear Filters */}
              <div className="flex items-end">
                <button
                  onClick={() => setFilters({
                    minReward: 0,
                    maxReward: 10000,
                    difficulty: [],
                    status: [],
                    tags: []
                  })}
                  className="w-full btn-secondary"
                >
                  Clear Filters
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Results Count */}
      <div className="mb-4 text-sm text-gray-600">
        Showing {filteredBountiesWithSearch.length} of {filteredBounties.length} bounties
      </div>

      {/* Bounty List */}
      <div className="space-y-4">
        {filteredBountiesWithSearch.map(bounty => (
          <div key={bounty.id} className="card hover:shadow-xl transition-shadow">
            <div className="flex justify-between items-start mb-4">
              <div className="flex-1">
                <h3 className="text-xl font-semibold text-gray-900 mb-2">{bounty.title}</h3>
                <p className="text-gray-600 mb-3">{bounty.description}</p>
                
                {/* Tags */}
                <div className="flex flex-wrap gap-2 mb-3">
                  {bounty.tags.map(tag => (
                    <span key={tag} className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded-full">
                      #{tag}
                    </span>
                  ))}
                </div>

                {/* Meta Information */}
                <div className="flex flex-wrap items-center gap-4 text-sm text-gray-500">
                  <div className="flex items-center">
                    <DollarSign className="h-4 w-4 mr-1" />
                    <span className="font-semibold text-green-600">{bounty.rewardAmount} XLM</span>
                  </div>
                  <div className="flex items-center">
                    <User className="h-4 w-4 mr-1" />
                    <span>{bounty.creator.slice(0, 8)}...</span>
                  </div>
                  {bounty.deadline && (
                    <div className="flex items-center">
                      <Clock className="h-4 w-4 mr-1" />
                      <CountdownTimer deadline={bounty.deadline} />
                    </div>
                  )}
                </div>
              </div>

              {/* Status and Difficulty Badges */}
              <div className="flex flex-col items-end space-y-2">
                <span className={`px-3 py-1 rounded-full text-sm font-medium border ${getDifficultyColor(bounty.difficulty)}`}>
                  {bounty.difficulty}
                </span>
                <span className={`px-3 py-1 rounded-full text-sm font-medium border ${getStatusColor(bounty.status)}`}>
                  {bounty.status}
                </span>
                {bounty.firstToFind && (
                  <span className="px-3 py-1 bg-yellow-100 text-yellow-800 text-sm font-medium border border-yellow-200 rounded-full">
                    🏅 First-to-Find
                  </span>
                )}
              </div>
            </div>

            {/* Action Buttons */}
            <div className="flex justify-end space-x-3 pt-4 border-t border-gray-200">
              <button className="btn-secondary">View Details</button>
              {bounty.status === 'Active' && (
                <button className="btn-primary">Start Audit</button>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Empty State */}
      {filteredBountiesWithSearch.length === 0 && (
        <div className="text-center py-12">
          <AlertCircle className="h-16 w-16 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">No bounties found</h3>
          <p className="text-gray-600">Try adjusting your filters or search terms</p>
        </div>
      )}
    </div>
  );
};

export default BountyBoard;
