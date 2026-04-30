'use client';

import { useState, useMemo, useEffect } from 'react';

interface Transaction {
  id: string;
  contract: string;
  date: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  status: 'completed' | 'failed' | 'in-progress';
  issuesFound: number;
}

type Severity = Transaction['severity'];
type Status = Transaction['status'];

interface SearchFilters {
  query: string;
  severities: Severity[];
  statuses: Status[];
  issueScope: 'all' | 'with-issues' | 'clean';
}

interface SavedSearch {
  id: string;
  name: string;
  filters: SearchFilters;
  createdAt: string;
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
  const [severityFilters, setSeverityFilters] = useState<Severity[]>([]);
  const [statusFilters, setStatusFilters] = useState<Status[]>([]);
  const [issueScope, setIssueScope] = useState<SearchFilters['issueScope']>('all');
  const [sortConfig, setSortConfig] = useState<{ key: keyof Transaction; direction: 'asc' | 'desc' } | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null);
  const [savedSearches, setSavedSearches] = useState<SavedSearch[]>([]);
  const [savedSearchName, setSavedSearchName] = useState('');
  const itemsPerPage = 5;
  const savedSearchStorageKey = 'transactionHistory.savedSearches.v1';

  const severityOptions: Severity[] = ['critical', 'high', 'medium', 'low'];
  const statusOptions: Status[] = ['completed', 'failed', 'in-progress'];

  useEffect(() => {
    const stored = localStorage.getItem(savedSearchStorageKey);
    if (!stored) return;
    try {
      const parsed = JSON.parse(stored) as SavedSearch[];
      if (Array.isArray(parsed)) {
        setSavedSearches(parsed);
      }
    } catch (error) {
      console.warn('Could not parse saved searches', error);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem(savedSearchStorageKey, JSON.stringify(savedSearches));
  }, [savedSearches]);

  const filteredData = useMemo(() => {
    const normalizedQuery = searchTerm.trim().toLowerCase();
    return mockTransactions
      .filter((tx) => {
        const matchesSearch =
          normalizedQuery.length === 0 ||
          tx.contract.toLowerCase().includes(normalizedQuery) ||
          tx.id.toLowerCase().includes(normalizedQuery);

        const matchesSeverity = severityFilters.length === 0 || severityFilters.includes(tx.severity);
        const matchesStatus = statusFilters.length === 0 || statusFilters.includes(tx.status);
        const matchesIssues =
          issueScope === 'all' ||
          (issueScope === 'with-issues' && tx.issuesFound > 0) ||
          (issueScope === 'clean' && tx.issuesFound === 0);

        return matchesSearch && matchesSeverity && matchesStatus && matchesIssues;
      })
      .sort((a, b) => {
        if (!sortConfig) return 0;
        const { key, direction } = sortConfig;
        if (a[key] < b[key]) return direction === 'asc' ? -1 : 1;
        if (a[key] > b[key]) return direction === 'asc' ? 1 : -1;
        return 0;
      });
  }, [searchTerm, severityFilters, statusFilters, issueScope, sortConfig]);

  useEffect(() => {
    setCurrentPage(1);
  }, [searchTerm, severityFilters, statusFilters, issueScope]);

  const paginatedData = useMemo(() => {
    const start = (currentPage - 1) * itemsPerPage;
    return filteredData.slice(start, start + itemsPerPage);
  }, [filteredData, currentPage]);

  const totalPages = Math.max(1, Math.ceil(filteredData.length / itemsPerPage));

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

  const toggleFacetValue = <T extends string>(
    value: T,
    selectedValues: T[],
    setSelectedValues: React.Dispatch<React.SetStateAction<T[]>>
  ) => {
    setSelectedValues((current) =>
      current.includes(value) ? current.filter((item) => item !== value) : [...current, value]
    );
  };

  const captureCurrentFilters = (): SearchFilters => ({
    query: searchTerm,
    severities: severityFilters,
    statuses: statusFilters,
    issueScope
  });

  const applyFilters = (filters: SearchFilters) => {
    setSearchTerm(filters.query);
    setSeverityFilters(filters.severities);
    setStatusFilters(filters.statuses);
    setIssueScope(filters.issueScope);
  };

  const saveCurrentSearch = () => {
    const cleanedName = savedSearchName.trim();
    if (!cleanedName) return;
    const nextSearch: SavedSearch = {
      id: `search-${Date.now()}`,
      name: cleanedName,
      filters: captureCurrentFilters(),
      createdAt: new Date().toISOString()
    };
    setSavedSearches((current) => [nextSearch, ...current].slice(0, 8));
    setSavedSearchName('');
  };

  const removeSavedSearch = (id: string) => {
    setSavedSearches((current) => current.filter((entry) => entry.id !== id));
  };

  const clearAllFilters = () => {
    setSearchTerm('');
    setSeverityFilters([]);
    setStatusFilters([]);
    setIssueScope('all');
  };

  const autocompleteSuggestions = useMemo(() => {
    const choices = new Set<string>();
    mockTransactions.forEach((tx) => {
      choices.add(tx.id);
      choices.add(tx.contract);
    });
    return Array.from(choices).sort();
  }, []);

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
      <div className="space-y-4">
        <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
          <div className="flex-1 w-full max-w-sm">
            <input
              type="text"
              list="transaction-search-suggestions"
              placeholder="Search by contract or ID..."
              className="input"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
            <datalist id="transaction-search-suggestions">
              {autocompleteSuggestions.map((choice) => (
                <option key={choice} value={choice} />
              ))}
            </datalist>
          </div>
          <div className="flex gap-2 w-full md:w-auto">
            <button className="btn btn-secondary" onClick={exportToCSV}>
              Export CSV
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div className="rounded-xl border border-border p-3 space-y-2">
            <p className="text-xs font-semibold uppercase tracking-wider text-gray-500">Severity facets</p>
            <div className="flex flex-wrap gap-2">
              {severityOptions.map((severity) => (
                <button
                  key={severity}
                  onClick={() => toggleFacetValue(severity, severityFilters, setSeverityFilters)}
                  className={`px-3 py-1 rounded-full text-xs font-semibold border transition-colors ${
                    severityFilters.includes(severity)
                      ? 'bg-blue-50 text-blue-700 border-blue-300'
                      : 'bg-white text-gray-600 border-gray-200 hover:bg-gray-50'
                  }`}
                >
                  {severity.charAt(0).toUpperCase() + severity.slice(1)}
                </button>
              ))}
            </div>
          </div>

          <div className="rounded-xl border border-border p-3 space-y-2">
            <p className="text-xs font-semibold uppercase tracking-wider text-gray-500">Status facets</p>
            <div className="flex flex-wrap gap-2">
              {statusOptions.map((status) => (
                <button
                  key={status}
                  onClick={() => toggleFacetValue(status, statusFilters, setStatusFilters)}
                  className={`px-3 py-1 rounded-full text-xs font-semibold border transition-colors ${
                    statusFilters.includes(status)
                      ? 'bg-blue-50 text-blue-700 border-blue-300'
                      : 'bg-white text-gray-600 border-gray-200 hover:bg-gray-50'
                  }`}
                >
                  {status.charAt(0).toUpperCase() + status.slice(1)}
                </button>
              ))}
            </div>
          </div>

          <div className="rounded-xl border border-border p-3 space-y-2">
            <p className="text-xs font-semibold uppercase tracking-wider text-gray-500">Issue facet</p>
            <select
              className="input"
              value={issueScope}
              onChange={(e) => setIssueScope(e.target.value as SearchFilters['issueScope'])}
            >
              <option value="all">All transactions</option>
              <option value="with-issues">With issues only</option>
              <option value="clean">Clean scans only</option>
            </select>
          </div>
        </div>

        <div className="rounded-xl border border-border p-3 space-y-3">
          <div className="flex flex-col md:flex-row gap-2 md:items-center">
            <input
              type="text"
              className="input md:max-w-xs"
              placeholder="Save current search as..."
              value={savedSearchName}
              onChange={(e) => setSavedSearchName(e.target.value)}
            />
            <button className="btn btn-primary" onClick={saveCurrentSearch} disabled={!savedSearchName.trim()}>
              Save Search
            </button>
            <button className="btn btn-secondary" onClick={clearAllFilters}>
              Reset Filters
            </button>
          </div>

          {savedSearches.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {savedSearches.map((search) => (
                <div key={search.id} className="inline-flex items-center gap-1 rounded-full border border-gray-200 bg-gray-50 px-3 py-1">
                  <button
                    onClick={() => applyFilters(search.filters)}
                    className="text-xs font-medium text-gray-700 hover:text-blue-700"
                  >
                    {search.name}
                  </button>
                  <button
                    onClick={() => removeSavedSearch(search.id)}
                    className="text-xs text-gray-500 hover:text-red-600"
                    aria-label={`Remove saved search ${search.name}`}
                  >
                    x
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs text-gray-500">No saved searches yet. Save commonly used filter combinations.</p>
          )}
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
            {paginatedData.length > 0 ? (
              paginatedData.map((tx) => (
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
              ))
            ) : (
              <tr>
                <td colSpan={7} className="p-8 text-center text-sm text-gray-500">
                  No transactions match the current search and facet filters.
                </td>
              </tr>
            )}
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
