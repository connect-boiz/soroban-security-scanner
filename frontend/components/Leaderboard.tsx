'use client';

import { useState, useEffect } from 'react';
import { useBountyStore } from '@/store/bountyStore';
import { Researcher } from '@/types/bounty';
import { 
  Trophy, 
  Medal, 
  Award, 
  Star, 
  TrendingUp, 
  DollarSign,
  Shield,
  Target,
  Calendar,
  Filter
} from 'lucide-react';

export const Leaderboard: React.FC = () => {
  const { researchers, setResearchers, loading, error } = useBountyStore();
  const [timeFilter, setTimeFilter] = useState<'all' | 'month' | 'week'>('all');
  const [categoryFilter, setCategoryFilter] = useState<'all' | 'critical' | 'high' | 'medium' | 'low'>('all');

  useEffect(() => {
    // Mock data for development
    const mockResearchers: Researcher[] = [
      {
        address: 'GABC123...XYZ789',
        username: 'SecurityNinja',
        reputation: 2850,
        completedBounties: 47,
        totalEarned: 156750,
        rank: 1,
        badges: ['🏆 Top Researcher', '🔒 Critical Finder', '⚡ Speed Demon', '🎯 Perfect Score']
      },
      {
        address: 'GDEF456...UVW012',
        username: 'CryptoHunter',
        reputation: 2420,
        completedBounties: 38,
        totalEarned: 124500,
        rank: 2,
        badges: ['🥈 Runner Up', '🔍 Detail Oriented', '💰 High Earner']
      },
      {
        address: 'GHI789...RST345',
        username: 'BugSlayer',
        reputation: 2180,
        completedBounties: 35,
        totalEarned: 98200,
        rank: 3,
        badges: ['🥉 Third Place', '🛡️ Guardian', '📈 Consistent']
      },
      {
        address: 'GJK012...MNO678',
        username: 'AuditPro',
        reputation: 1950,
        completedBounties: 32,
        totalEarned: 87600,
        rank: 4,
        badges: ['🔧 Expert', '🎖️ Veteran', '⭐ Rising Star']
      },
      {
        address: 'GHI345...PQR901',
        username: 'StellarSec',
        reputation: 1720,
        completedBounties: 28,
        totalEarned: 72300,
        rank: 5,
        badges: ['🌟 Stellar Expert', '🚀 Fast Finder', '💎 Quality Work']
      },
      {
        address: 'LMN678...STU234',
        username: 'CodeDefender',
        reputation: 1580,
        completedBounties: 25,
        totalEarned: 65400,
        rank: 6,
        badges: ['🛡️ Security Expert', '📊 Analyst']
      },
      {
        address: 'OPQ901...VWX567',
        username: 'VulnHunter',
        reputation: 1420,
        completedBounties: 22,
        totalEarned: 58900,
        rank: 7,
        badges: ['🎯 Precision', '🔎 Deep Diver']
      },
      {
        address: 'RST234...YZA890',
        username: 'SmartContractGuru',
        reputation: 1350,
        completedBounties: 20,
        totalEarned: 54200,
        rank: 8,
        badges: ['🧠 Smart Mind', '💡 Innovative']
      }
    ];

    setResearchers(mockResearchers);
  }, [setResearchers]);

  const getRankIcon = (rank: number) => {
    switch (rank) {
      case 1:
        return <Trophy className="h-8 w-8 text-yellow-500" />;
      case 2:
        return <Medal className="h-8 w-8 text-gray-400" />;
      case 3:
        return <Award className="h-8 w-8 text-orange-600" />;
      default:
        return <div className="h-8 w-8 flex items-center justify-center text-lg font-bold text-gray-600">#{rank}</div>;
    }
  };

  const getRankBadgeColor = (rank: number) => {
    if (rank === 1) return 'bg-yellow-100 text-yellow-800 border-yellow-200';
    if (rank === 2) return 'bg-gray-100 text-gray-800 border-gray-200';
    if (rank === 3) return 'bg-orange-100 text-orange-800 border-orange-200';
    return 'bg-blue-100 text-blue-800 border-blue-200';
  };

  const formatXLM = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(amount);
  };

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
        <Shield className="h-12 w-12 mx-auto mb-4" />
        <p>Error loading leaderboard: {error}</p>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto p-6">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900 mb-2 flex items-center">
          <Trophy className="h-8 w-8 text-yellow-500 mr-3" />
          Security Researcher Leaderboard
        </h1>
        <p className="text-gray-600">
          Top security researchers making the Stellar ecosystem safer
        </p>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
        <div className="card bg-gradient-to-r from-blue-500 to-blue-600 text-white">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-blue-100">Total Researchers</p>
              <p className="text-2xl font-bold">{researchers.length}</p>
            </div>
            <Shield className="h-8 w-8 text-blue-200" />
          </div>
        </div>
        
        <div className="card bg-gradient-to-r from-green-500 to-green-600 text-white">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-green-100">Total Earned</p>
              <p className="text-2xl font-bold">
                {formatXLM(researchers.reduce((sum, r) => sum + r.totalEarned, 0))} XLM
              </p>
            </div>
            <DollarSign className="h-8 w-8 text-green-200" />
          </div>
        </div>
        
        <div className="card bg-gradient-to-r from-purple-500 to-purple-600 text-white">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-purple-100">Bounties Completed</p>
              <p className="text-2xl font-bold">
                {researchers.reduce((sum, r) => sum + r.completedBounties, 0)}
              </p>
            </div>
            <Target className="h-8 w-8 text-purple-200" />
          </div>
        </div>
        
        <div className="card bg-gradient-to-r from-orange-500 to-orange-600 text-white">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-orange-100">Avg Reputation</p>
              <p className="text-2xl font-bold">
                {Math.round(researchers.reduce((sum, r) => sum + r.reputation, 0) / researchers.length)}
              </p>
            </div>
            <TrendingUp className="h-8 w-8 text-orange-200" />
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="card mb-6">
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center">
            <Filter className="h-4 w-4 text-gray-500 mr-2" />
            <span className="text-sm font-medium text-gray-700">Filters:</span>
          </div>
          
          <div className="flex gap-2">
            {(['all', 'month', 'week'] as const).map((period) => (
              <button
                key={period}
                onClick={() => setTimeFilter(period)}
                className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
                  timeFilter === period
                    ? 'bg-primary-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {period === 'all' ? 'All Time' : period === 'month' ? 'This Month' : 'This Week'}
              </button>
            ))}
          </div>

          <div className="flex gap-2">
            {(['all', 'critical', 'high', 'medium', 'low'] as const).map((category) => (
              <button
                key={category}
                onClick={() => setCategoryFilter(category)}
                className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
                  categoryFilter === category
                    ? 'bg-primary-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {category === 'all' ? 'All Levels' : category.charAt(0).toUpperCase() + category.slice(1)}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Leaderboard Table */}
      <div className="card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-gray-200">
                <th className="text-left p-4 font-semibold text-gray-900">Rank</th>
                <th className="text-left p-4 font-semibold text-gray-900">Researcher</th>
                <th className="text-center p-4 font-semibold text-gray-900">Reputation</th>
                <th className="text-center p-4 font-semibold text-gray-900">Completed</th>
                <th className="text-center p-4 font-semibold text-gray-900">Total Earned</th>
                <th className="text-left p-4 font-semibold text-gray-900">Badges</th>
              </tr>
            </thead>
            <tbody>
              {researchers.map((researcher) => (
                <tr key={researcher.address} className="border-b border-gray-100 hover:bg-gray-50 transition-colors">
                  <td className="p-4">
                    <div className="flex items-center">
                      {getRankIcon(researcher.rank)}
                    </div>
                  </td>
                  
                  <td className="p-4">
                    <div>
                      <div className="font-semibold text-gray-900">{researcher.username}</div>
                      <div className="text-sm text-gray-500 font-mono">
                        {researcher.address.slice(0, 8)}...{researcher.address.slice(-8)}
                      </div>
                    </div>
                  </td>
                  
                  <td className="p-4 text-center">
                    <div className="flex items-center justify-center">
                      <Star className="h-4 w-4 text-yellow-500 mr-1" />
                      <span className="font-semibold text-gray-900">{researcher.reputation.toLocaleString()}</span>
                    </div>
                  </td>
                  
                  <td className="p-4 text-center">
                    <div className="flex items-center justify-center">
                      <Target className="h-4 w-4 text-blue-500 mr-1" />
                      <span className="font-semibold text-gray-900">{researcher.completedBounties}</span>
                    </div>
                  </td>
                  
                  <td className="p-4 text-center">
                    <div className="flex items-center justify-center">
                      <DollarSign className="h-4 w-4 text-green-500 mr-1" />
                      <span className="font-semibold text-green-600">
                        {formatXLM(researcher.totalEarned)} XLM
                      </span>
                    </div>
                  </td>
                  
                  <td className="p-4">
                    <div className="flex flex-wrap gap-1">
                      {researcher.badges.slice(0, 3).map((badge, index) => (
                        <span
                          key={index}
                          className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded-full"
                        >
                          {badge}
                        </span>
                      ))}
                      {researcher.badges.length > 3 && (
                        <span className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded-full">
                          +{researcher.badges.length - 3}
                        </span>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Top Performers Highlights */}
      <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* #1 Performer */}
        {researchers[0] && (
          <div className="card bg-gradient-to-br from-yellow-50 to-yellow-100 border-yellow-200">
            <div className="text-center">
              <Trophy className="h-12 w-12 text-yellow-500 mx-auto mb-3" />
              <h3 className="text-lg font-bold text-gray-900 mb-1">🏆 Top Researcher</h3>
              <p className="font-semibold text-gray-800">{researchers[0].username}</p>
              <p className="text-sm text-gray-600 mt-1">
                {researchers[0].completedBounties} bounties • {formatXLM(researchers[0].totalEarned)} XLM earned
              </p>
            </div>
          </div>
        )}

        {/* Highest Earner */}
        {researchers.reduce((max, r) => r.totalEarned > max.totalEarned ? r : max, researchers[0]) && (
          <div className="card bg-gradient-to-br from-green-50 to-green-100 border-green-200">
            <div className="text-center">
              <DollarSign className="h-12 w-12 text-green-500 mx-auto mb-3" />
              <h3 className="text-lg font-bold text-gray-900 mb-1">💰 Highest Earner</h3>
              <p className="font-semibold text-gray-800">
                {researchers.reduce((max, r) => r.totalEarned > max.totalEarned ? r : max, researchers[0]).username}
              </p>
              <p className="text-sm text-gray-600 mt-1">
                {formatXLM(researchers.reduce((max, r) => r.totalEarned > max.totalEarned ? r : max, researchers[0]).totalEarned)} XLM total
              </p>
            </div>
          </div>
        )}

        {/* Most Active */}
        {researchers.reduce((max, r) => r.completedBounties > max.completedBounties ? r : max, researchers[0]) && (
          <div className="card bg-gradient-to-br from-blue-50 to-blue-100 border-blue-200">
            <div className="text-center">
              <Target className="h-12 w-12 text-blue-500 mx-auto mb-3" />
              <h3 className="text-lg font-bold text-gray-900 mb-1">🎯 Most Active</h3>
              <p className="font-semibold text-gray-800">
                {researchers.reduce((max, r) => r.completedBounties > max.completedBounties ? r : max, researchers[0]).username}
              </p>
              <p className="text-sm text-gray-600 mt-1">
                {researchers.reduce((max, r) => r.completedBounties > max.completedBounties ? r : max, researchers[0]).completedBounties} bounties completed
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
