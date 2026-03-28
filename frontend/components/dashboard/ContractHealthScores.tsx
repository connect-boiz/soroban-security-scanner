import React from 'react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, Cell } from 'recharts';
import { useDashboardStore } from '../store/dashboardStore';

interface ContractHealth {
  name: string;
  healthScore: number;
  vulnerabilities: number;
  lastScan: Date;
  status: 'excellent' | 'good' | 'fair' | 'poor';
}

export const ContractHealthScores: React.FC = () => {
  const { scanResults } = useDashboardStore();

  // Calculate health scores for each contract
  const getContractHealthData = (): ContractHealth[] => {
    const contractMap = new Map<string, ContractHealth>();

    scanResults.forEach(scan => {
      const existing = contractMap.get(scan.contract);
      
      // Calculate health score based on vulnerabilities and scan status
      const vulnerabilityWeight = scan.vulnerabilities.length * 10;
      const statusPenalty = scan.status === 'fail' ? 20 : 0;
      const healthScore = Math.max(0, Math.min(100, 100 - vulnerabilityWeight - statusPenalty));
      
      // Determine status category
      let status: 'excellent' | 'good' | 'fair' | 'poor';
      if (healthScore >= 90) status = 'excellent';
      else if (healthScore >= 75) status = 'good';
      else if (healthScore >= 60) status = 'fair';
      else status = 'poor';

      const healthData: ContractHealth = {
        name: scan.contract,
        healthScore,
        vulnerabilities: scan.vulnerabilities.length,
        lastScan: scan.timestamp,
        status
      };

      // Update if this is a more recent scan or if no previous scan exists
      if (!existing || scan.timestamp > existing.lastScan) {
        contractMap.set(scan.contract, healthData);
      }
    });

    return Array.from(contractMap.values())
      .sort((a, b) => b.healthScore - a.healthScore)
      .slice(0, 10); // Top 10 contracts
  };

  const getHealthColor = (status: string) => {
    switch (status) {
      case 'excellent':
        return '#10b981'; // green-500
      case 'good':
        return '#3b82f6'; // blue-500
      case 'fair':
        return '#f59e0b'; // amber-500
      case 'poor':
        return '#ef4444'; // red-500
      default:
        return '#6b7280'; // gray-500
    }
  };

  const getHealthBgColor = (status: string) => {
    switch (status) {
      case 'excellent':
        return 'bg-green-100 text-green-800';
      case 'good':
        return 'bg-blue-100 text-blue-800';
      case 'fair':
        return 'bg-amber-100 text-amber-800';
      case 'poor':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const data = getContractHealthData();

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
          <p className="text-sm font-medium text-gray-900 mb-2">{data.name}</p>
          <p className="text-sm text-gray-600">Health Score: <span className="font-semibold">{data.healthScore}/100</span></p>
          <p className="text-sm text-gray-600">Vulnerabilities: <span className="font-semibold">{data.vulnerabilities}</span></p>
          <p className="text-sm text-gray-600">Status: <span className="font-semibold capitalize">{data.status}</span></p>
        </div>
      );
    }
    return null;
  };

  if (data.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
        <h2 className="text-xl font-bold text-gray-800 mb-4">Contract Health Scores</h2>
        <div className="text-center py-8 text-gray-500">
          <p>No contract data available</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow-md p-6 border border-gray-200">
      <h2 className="text-xl font-bold text-gray-800 mb-6">Contract Health Scores</h2>
      
      {/* Health Score Chart */}
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-gray-700 mb-3">Top 10 Contracts by Health Score</h3>
        <ResponsiveContainer width="100%" height={300}>
          <BarChart data={data} margin={{ top: 20, right: 30, left: 20, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis 
              dataKey="name" 
              tick={{ fontSize: 12 }}
              stroke="#6b7280"
              angle={-45}
              textAnchor="end"
              height={80}
            />
            <YAxis 
              tick={{ fontSize: 12 }}
              stroke="#6b7280"
              domain={[0, 100]}
            />
            <Tooltip content={<CustomTooltip />} />
            <Bar dataKey="healthScore" radius={[4, 4, 0, 0]}>
              {data.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={getHealthColor(entry.status)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Health Status Distribution */}
      <div>
        <h3 className="text-lg font-semibold text-gray-700 mb-3">Health Status Distribution</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {['excellent', 'good', 'fair', 'poor'].map((status) => {
            const count = data.filter(contract => contract.status === status).length;
            const percentage = data.length > 0 ? (count / data.length) * 100 : 0;
            
            return (
              <div key={status} className="text-center p-3 bg-gray-50 rounded-lg">
                <div 
                  className={`inline-block w-4 h-4 rounded-full mb-2`}
                  style={{ backgroundColor: getHealthColor(status) }}
                ></div>
                <div className="text-lg font-bold text-gray-800">{count}</div>
                <div className="text-sm text-gray-600 capitalize">{status}</div>
                <div className="text-xs text-gray-500">{percentage.toFixed(1)}%</div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Contract Details Table */}
      <div className="mt-6">
        <h3 className="text-lg font-semibold text-gray-700 mb-3">Contract Details</h3>
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Contract
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Health Score
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Vulnerabilities
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {data.map((contract) => (
                <tr key={contract.name} className="hover:bg-gray-50">
                  <td className="px-4 py-2 whitespace-nowrap text-sm font-medium text-gray-900">
                    {contract.name}
                  </td>
                  <td className="px-4 py-2 whitespace-nowrap text-sm text-gray-500">
                    <div className="flex items-center">
                      <div className="w-16 bg-gray-200 rounded-full h-2 mr-2">
                        <div 
                          className="h-2 rounded-full"
                          style={{
                            width: `${contract.healthScore}%`,
                            backgroundColor: getHealthColor(contract.status)
                          }}
                        ></div>
                      </div>
                      <span className="font-medium">{contract.healthScore}</span>
                    </div>
                  </td>
                  <td className="px-4 py-2 whitespace-nowrap">
                    <span className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full ${getHealthBgColor(contract.status)}`}>
                      {contract.status}
                    </span>
                  </td>
                  <td className="px-4 py-2 whitespace-nowrap text-sm text-gray-500">
                    {contract.vulnerabilities}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Legend */}
      <div className="mt-6 flex flex-wrap gap-4 text-sm">
        <div className="flex items-center">
          <div className="w-4 h-4 bg-green-500 rounded mr-2" aria-label="Excellent health"></div>
          <span className="font-medium">Excellent (90-100)</span>
        </div>
        <div className="flex items-center">
          <div className="w-4 h-4 bg-blue-500 rounded mr-2" aria-label="Good health"></div>
          <span className="font-medium">Good (75-89)</span>
        </div>
        <div className="flex items-center">
          <div className="w-4 h-4 bg-amber-500 rounded mr-2" aria-label="Fair health"></div>
          <span className="font-medium">Fair (60-74)</span>
        </div>
        <div className="flex items-center">
          <div className="w-4 h-4 bg-red-500 rounded mr-2" aria-label="Poor health"></div>
          <span className="font-medium">Poor (&lt;60)</span>
        </div>
      </div>
    </div>
  );
};
