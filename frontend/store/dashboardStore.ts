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
  setTimeFilter: (filter: TimeFilter) => void;
  setScanResults: (results: ScanResult[]) => void;
  setMetrics: (metrics: DashboardMetrics) => void;
  setLoading: (loading: boolean) => void;
  fetchDashboardData: () => Promise<void>;
}

const generateMockData = () => {
  const severities: Array<'critical' | 'high' | 'medium' | 'low'> = ['critical', 'high', 'medium', 'low'];
  const statuses: Array<'pass' | 'fail'> = ['pass', 'fail'];
  
  const scanResults: ScanResult[] = Array.from({ length: 50 }, (_, i) => ({
    id: `scan-${i + 1}`,
    contract: `Contract ${i + 1}`,
    status: statuses[Math.floor(Math.random() * statuses.length)],
    executionTime: Math.floor(Math.random() * 3000) + 500,
    timestamp: new Date(Date.now() - Math.floor(Math.random() * 30) * 24 * 60 * 60 * 1000),
    vulnerabilities: Array.from({ length: Math.floor(Math.random() * 5) }, (_, j) => ({
      id: `vuln-${i}-${j}`,
      severity: severities[Math.floor(Math.random() * severities.length)],
      description: `Vulnerability ${j + 1}`,
      contract: `Contract ${i + 1}`,
      timestamp: new Date(Date.now() - Math.floor(Math.random() * 30) * 24 * 60 * 60 * 1000),
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

  setTimeFilter: (filter) => set({ timeFilter: filter }),

  setScanResults: (results) => set({ scanResults: results }),

  setMetrics: (metrics) => set({ metrics }),

  setLoading: (loading) => set({ isLoading: loading }),

  fetchDashboardData: async () => {
    set({ isLoading: true });
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000));
      const { scanResults, metrics } = generateMockData();
      set({ scanResults, metrics });
    } catch (error) {
      console.error('Failed to fetch dashboard data:', error);
    } finally {
      set({ isLoading: false });
    }
  },
}));
