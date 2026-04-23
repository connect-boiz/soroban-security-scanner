import { create } from 'zustand';

export interface Vulnerability {
  id: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  description: string;
  contract: string;
  timestamp: Date;
}

export interface ScanResult {
  id: string;
  contract: string;
  status: 'pass' | 'fail';
  executionTime: number;
  timestamp: Date;
  vulnerabilities: Vulnerability[];
}

export interface DashboardMetrics {
  totalScans: number;
  criticalIssues: number;
  highIssues: number;
  mediumIssues: number;
  lowIssues: number;
  averageExecutionTime: number;
  passRate: number;
}

export interface TimeFilter {
  label: string;
  value: '7days' | '30days' | 'year';
}

interface DashboardStore {
  scanResults: ScanResult[];
  metrics: DashboardMetrics;
  timeFilter: TimeFilter;
  isLoading: boolean;
  allScanResults: ScanResult[];
  allMetrics: DashboardMetrics;
  setTimeFilter: (filter: TimeFilter) => void;
  setScanResults: (results: ScanResult[]) => void;
  setMetrics: (metrics: DashboardMetrics) => void;
  setLoading: (loading: boolean) => void;
  fetchDashboardData: () => Promise<void>;
  getFilteredData: () => { filteredResults: ScanResult[], filteredMetrics: DashboardMetrics };
}

const generateMockData = () => {
  const severities: Array<'critical' | 'high' | 'medium' | 'low'> = ['critical', 'high', 'medium', 'low'];
  const statuses: Array<'pass' | 'fail'> = ['pass', 'fail'];
  
  const scanResults: ScanResult[] = Array.from({ length: 50 }, (_, i) => ({
    id: `scan-${i + 1}`,
    contract: `Contract ${i + 1}`,
    status: statuses[Math.floor(Math.random() * statuses.length)],
    executionTime: Math.floor(Math.random() * 3000) + 500,
    timestamp: new Date(Date.now() - Math.floor(Math.random() * 365) * 24 * 60 * 60 * 1000),
    vulnerabilities: Array.from({ length: Math.floor(Math.random() * 5) }, (_, j) => ({
      id: `vuln-${i}-${j}`,
      severity: severities[Math.floor(Math.random() * severities.length)],
      description: `Vulnerability ${j + 1}`,
      contract: `Contract ${i + 1}`,
      timestamp: new Date(Date.now() - Math.floor(Math.random() * 365) * 24 * 60 * 60 * 1000),
    })),
  }));

  const allVulnerabilities = scanResults.flatMap(scan => scan.vulnerabilities);
  const metrics: DashboardMetrics = {
    totalScans: scanResults.length,
    criticalIssues: allVulnerabilities.filter(v => v.severity === 'critical').length,
    highIssues: allVulnerabilities.filter(v => v.severity === 'high').length,
    mediumIssues: allVulnerabilities.filter(v => v.severity === 'medium').length,
    lowIssues: allVulnerabilities.filter(v => v.severity === 'low').length,
    averageExecutionTime: scanResults.reduce((acc, scan) => acc + scan.executionTime, 0) / scanResults.length,
    passRate: (scanResults.filter(scan => scan.status === 'pass').length / scanResults.length) * 100,
  };

  return { scanResults, metrics };
};

const filterDataByTimeRange = (scanResults: ScanResult[], timeFilter: TimeFilter): { filteredResults: ScanResult[], filteredMetrics: DashboardMetrics } => {
  const now = new Date();
  let cutoffDate: Date;

  switch (timeFilter.value) {
    case '7days':
      cutoffDate = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      break;
    case '30days':
      cutoffDate = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
      break;
    case 'year':
      cutoffDate = new Date(now.getTime() - 365 * 24 * 60 * 60 * 1000);
      break;
    default:
      cutoffDate = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
  }

  const filteredResults = scanResults.filter(scan => scan.timestamp >= cutoffDate);
  const allVulnerabilities = filteredResults.flatMap(scan => scan.vulnerabilities);
  
  const filteredMetrics: DashboardMetrics = {
    totalScans: filteredResults.length,
    criticalIssues: allVulnerabilities.filter(v => v.severity === 'critical').length,
    highIssues: allVulnerabilities.filter(v => v.severity === 'high').length,
    mediumIssues: allVulnerabilities.filter(v => v.severity === 'medium').length,
    lowIssues: allVulnerabilities.filter(v => v.severity === 'low').length,
    averageExecutionTime: filteredResults.length > 0 
      ? filteredResults.reduce((acc, scan) => acc + scan.executionTime, 0) / filteredResults.length 
      : 0,
    passRate: filteredResults.length > 0 
      ? (filteredResults.filter(scan => scan.status === 'pass').length / filteredResults.length) * 100 
      : 0,
  };

  return { filteredResults, filteredMetrics };
};

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  scanResults: [],
  metrics: {
    totalScans: 0,
    criticalIssues: 0,
    highIssues: 0,
    mediumIssues: 0,
    lowIssues: 0,
    averageExecutionTime: 0,
    passRate: 0,
  },
  timeFilter: { label: 'Last 7 Days', value: '7days' },
  isLoading: false,
  allScanResults: [],
  allMetrics: {
    totalScans: 0,
    criticalIssues: 0,
    highIssues: 0,
    mediumIssues: 0,
    lowIssues: 0,
    averageExecutionTime: 0,
    passRate: 0,
  },

  setTimeFilter: (filter) => {
    set({ timeFilter: filter });
    const { getFilteredData } = get();
    const { filteredResults, filteredMetrics } = getFilteredData();
    set({ scanResults: filteredResults, metrics: filteredMetrics });
  },

  setScanResults: (results) => set({ scanResults: results }),

  setMetrics: (metrics) => set({ metrics }),

  setLoading: (loading) => set({ isLoading: loading }),

  getFilteredData: () => {
    const { allScanResults, timeFilter } = get();
    return filterDataByTimeRange(allScanResults, timeFilter);
  },

  fetchDashboardData: async () => {
    set({ isLoading: true });
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000));
      const { scanResults: allResults, metrics: allMetrics } = generateMockData();
      set({ allScanResults: allResults, allMetrics });
      
      // Apply current time filter
      const { getFilteredData } = get();
      const { filteredResults, filteredMetrics } = getFilteredData();
      set({ scanResults: filteredResults, metrics: filteredMetrics });
    } catch (error) {
      console.error('Failed to fetch dashboard data:', error);
    } finally {
      set({ isLoading: false });
    }
  },
}));
