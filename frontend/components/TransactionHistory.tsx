'use client';

import { useState, useMemo } from 'react';

interface Transaction {
  id: string;
  contract: string;
  date: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  status: 'completed' | 'failed' | 'in-progress';
  issuesFound: number;
}

const mockTransactions: Transaction[] = [
  { id: 'scan-001', contract: '0x123...456', date: '2024-04-20 10:30', severity: 'high', status: 'completed', issuesFound: 3 },
  { id: 'scan-002', contract: '0xabc...def', date: '2024-04-20 11:45', severity: 'critical', status: 'completed', issuesFound: 1 },
  { id: 'scan-003', contract: '0x789...012', date: '2024-04-21 09:15', severity: 'low', status: 'completed', issuesFound: 0 },
  { id: 'scan-004', contract: '0x456...789', date: '2024-04-21 14:20', severity: 'medium', status: 'completed', issuesFound: 2 },
  { id: 'scan-005', contract: '0xdef...012', date: '2024-04-22 16:00', severity: 'high', status: 'in-progress', issuesFound: 0 },
  { id: 'scan-006', contract: '0x321...654', date: '2024-04-23 08:30', severity: 'high', status: 'failed', issuesFound: 0 },
  { id: 'scan-007', contract: '0x987...654', date: '2024-04-24 12:00', severity: 'low', status: 'completed', issuesFound: 1 },
  { id: 'scan-008', contract: '0x111...222', date: '2024-04-25 10:00', severity: 'critical', status: 'completed', issuesFound: 5 },
  { id: 'scan-009', contract: '0x333...444', date: '2024-04-26 15:30', severity: 'medium', status: 'completed', issuesFound: 2 },
  { id: 'scan-010', contract: '0x555...666', date: '2024-04-27 11:20', severity: 'low', status: 'completed', issuesFound: 0 },
];

export default function TransactionHistory() {
  const [searchTerm, setSearchTerm] = useState('');
  const [severityFilter, setSeverityFilter] = useState<string>('all');
  const [sortConfig, setSortConfig] = useState<{ key: keyof Transaction; direction: 'asc' | 'desc' } | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null);
  const itemsPerPage = 5;

  const filteredData = useMemo(() => {
    return mockTransactions
      .filter((tx) => {
        const matchesSearch = tx.contract.toLowerCase().includes(searchTerm.toLowerCase()) || tx.id.toLowerCase().includes(searchTerm.toLowerCase());
        const matchesSeverity = severityFilter === 'all' || tx.severity === severityFilter;
        return matchesSearch && matchesSeverity;
      })
      .sort((a, b) => {
        if (!sortConfig) return 0;
        const { key, direction } = sortConfig;
        if (a[key] < b[key]) return direction === 'asc' ? -1 : 1;
        if (a[key] > b[key]) return direction === 'asc' ? 1 : -1;
        return 0;
      });
  }, [searchTerm, severityFilter, sortConfig]);

  const paginatedData = useMemo(() => {
    const start = (currentPage - 1) * itemsPerPage;
    return filteredData.slice(start, start + itemsPerPage);
  }, [filteredData, currentPage]);

  const totalPages = Math.ceil(filteredData.length / itemsPerPage);

  const requestSort = (key: keyof Transaction) => {
    let direction: 'asc' | 'desc' = 'asc';
    if (sortConfig && sortConfig.key === key && sortConfig.direction === 'asc') {
      direction = 'desc';
    }
    setSortConfig({ key, direction });
  };

  const exportToCSV = () => {
    const headers = ['ID', 'Contract', 'Date', 'Severity', 'Status', 'Issues Found'];
    const rows = filteredData.map(tx => [tx.id, tx.contract, tx.date, tx.severity, tx.status, tx.issuesFound]);
    const csvContent = [headers, ...rows].map(e => e.join(",")).join("\n");
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement("a");
    const url = URL.createObjectURL(blob);
    link.setAttribute("href", url);
    link.setAttribute("download", "scan_history.csv");
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const getSeverityClass = (severity: string) => {
    switch (severity) {
      case 'critical': return 'bg-red-100 text-red-800 border-red-200';
      case 'high': return 'bg-orange-100 text-orange-800 border-orange-200';
      case 'medium': return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'low': return 'bg-green-100 text-green-800 border-green-200';
      default: return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="card space-y-6">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
        <div className="flex-1 w-full max-w-sm">
          <input
            type="text"
            placeholder="Search by contract or ID..."
            className="input"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
        <div className="flex gap-2 w-full md:w-auto">
          <select 
            className="input w-auto"
            value={severityFilter}
            onChange={(e) => setSeverityFilter(e.target.value)}
          >
            <option value="all">All Severities</option>
            <option value="critical">Critical</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
          <button className="btn btn-secondary" onClick={exportToCSV}>
            Export CSV
          </button>
        </div>
      </div>

      <div className="overflow-x-auto border border-border rounded-xl">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="bg-gray-50 border-b border-border">
              <th className="p-4 font-semibold text-sm cursor-pointer hover:bg-gray-100" onClick={() => requestSort('id')}>ID</th>
              <th className="p-4 font-semibold text-sm cursor-pointer hover:bg-gray-100" onClick={() => requestSort('contract')}>Contract</th>
              <th className="p-4 font-semibold text-sm cursor-pointer hover:bg-gray-100" onClick={() => requestSort('date')}>Date</th>
              <th className="p-4 font-semibold text-sm cursor-pointer hover:bg-gray-100" onClick={() => requestSort('severity')}>Max Severity</th>
              <th className="p-4 font-semibold text-sm cursor-pointer hover:bg-gray-100" onClick={() => requestSort('status')}>Status</th>
              <th className="p-4 font-semibold text-sm">Issues</th>
              <th className="p-4 font-semibold text-sm text-right">Action</th>
            </tr>
          </thead>
          <tbody>
            {paginatedData.map((tx) => (
              <tr key={tx.id} className="border-b border-border hover:bg-gray-50 transition-colors">
                <td className="p-4 text-sm font-medium">{tx.id}</td>
                <td className="p-4 text-sm font-mono text-blue-600">{tx.contract}</td>
                <td className="p-4 text-sm text-gray-500">{tx.date}</td>
                <td className="p-4">
                  <span className={`px-2.5 py-0.5 rounded-full text-xs font-bold border ${getSeverityClass(tx.severity)}`}>
                    {tx.severity.toUpperCase()}
                  </span>
                </td>
                <td className="p-4">
                  <span className={`text-xs font-semibold ${tx.status === 'completed' ? 'text-green-600' : tx.status === 'failed' ? 'text-red-600' : 'text-blue-600'}`}>
                    {tx.status.charAt(0).toUpperCase() + tx.status.slice(1)}
                  </span>
                </td>
                <td className="p-4 text-sm font-semibold">{tx.issuesFound}</td>
                <td className="p-4 text-right">
                  <button 
                    className="text-blue-600 hover:underline text-sm font-semibold"
                    onClick={() => setSelectedTx(tx)}
                  >
                    Details
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="flex justify-between items-center pt-4">
        <p className="text-sm text-gray-500">
          Showing {paginatedData.length} of {filteredData.length} results
        </p>
        <div className="flex gap-2">
          <button 
            className="btn btn-secondary py-1.5" 
            disabled={currentPage === 1}
            onClick={() => setCurrentPage(prev => prev - 1)}
          >
            Previous
          </button>
          <div className="flex items-center px-4 text-sm font-medium">
            Page {currentPage} of {totalPages}
          </div>
          <button 
            className="btn btn-secondary py-1.5" 
            disabled={currentPage === totalPages}
            onClick={() => setCurrentPage(prev => prev + 1)}
          >
            Next
          </button>
        </div>
      </div>

      {selectedTx && (
        <div className="fixed inset-0 bg-black/50 flex-center z-[100] backdrop-blur-sm animate-fade-in">
          <div className="card w-full max-w-lg shadow-2xl relative">
            <button 
              className="absolute top-4 right-4 text-gray-400 hover:text-gray-600"
              onClick={() => setSelectedTx(null)}
            >
              ✕
            </button>
            <h3 className="text-xl font-bold mb-4">Scan Details: {selectedTx.id}</h3>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-xs text-gray-500 uppercase font-bold tracking-wider">Contract Address</label>
                  <p className="font-mono text-sm mt-1">{selectedTx.contract}</p>
                </div>
                <div>
                  <label className="text-xs text-gray-500 uppercase font-bold tracking-wider">Scan Date</label>
                  <p className="text-sm mt-1">{selectedTx.date}</p>
                </div>
              </div>
              <div>
                <label className="text-xs text-gray-500 uppercase font-bold tracking-wider">Security Report</label>
                <div className="mt-2 p-4 bg-gray-50 rounded-lg border border-border">
                  <p className="text-sm">Found <span className="font-bold">{selectedTx.issuesFound}</span> security vulnerabilities with a maximum severity of <span className={`font-bold ${selectedTx.severity === 'critical' ? 'text-red-600' : 'text-orange-600'}`}>{selectedTx.severity.toUpperCase()}</span>.</p>
                </div>
              </div>
              <button className="btn btn-primary w-full mt-4" onClick={() => setSelectedTx(null)}>Close</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
