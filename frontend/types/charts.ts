// Chart and Graph Component Types

export interface ChartDataPoint {
  x: string | number | Date;
  y: number;
  label?: string;
  color?: string;
}

export interface TimeSeriesData {
  timestamp: Date;
  value: number;
  category?: string;
}

export interface PortfolioData {
  asset: string;
  value: number;
  percentage: number;
  change: number;
  changePercent: number;
}

export interface TransactionData {
  id: string;
  timestamp: Date;
  amount: number;
  type: 'deposit' | 'withdrawal' | 'reward' | 'penalty';
  status: 'completed' | 'pending' | 'failed';
  from?: string;
  to?: string;
}

export interface PerformanceMetrics {
  date: Date;
  reputation: number;
  completedBounties: number;
  totalEarned: number;
  successRate: number;
  avgCompletionTime: number;
}

export interface ChartConfig {
  type: 'line' | 'bar' | 'pie' | 'area' | 'scatter' | 'donut' | 'radar';
  title: string;
  xAxisLabel?: string;
  yAxisLabel?: string;
  showLegend?: boolean;
  showGrid?: boolean;
  colors?: string[];
  height?: number;
  width?: number;
  responsive?: boolean;
}

export interface ChartFilter {
  timeRange: '24h' | '7d' | '30d' | '90d' | '1y' | 'all';
  category?: string;
  dataType?: string;
}

export interface ChartTooltip {
  show: boolean;
  formatter?: (value: number, name: string) => string;
  labelFormatter?: (label: string) => string;
}

// Chart component props interfaces
export interface BaseChartProps {
  data: ChartDataPoint[];
  config: ChartConfig;
  loading?: boolean;
  error?: string;
  className?: string;
}

export interface LineChartProps extends BaseChartProps {
  smooth?: boolean;
  showDots?: boolean;
  strokeWidth?: number;
}

export interface BarChartProps extends BaseChartProps {
  horizontal?: boolean;
  stackId?: string;
  barSize?: number;
}

export interface PieChartProps extends BaseChartProps {
  innerRadius?: number;
  outerRadius?: number;
  startAngle?: number;
  endAngle?: number;
}

export interface AreaChartProps extends BaseChartProps {
  type?: 'linear' | 'monotone' | 'step';
  strokeDasharray?: string;
}

// Dashboard specific types
export interface DashboardMetrics {
  totalValue: number;
  change24h: number;
  change7d: number;
  change30d: number;
  allTimeHigh: number;
  allTimeLow: number;
}

export interface AnalyticsData {
  portfolio: PortfolioData[];
  transactions: TransactionData[];
  performance: PerformanceMetrics[];
  metrics: DashboardMetrics;
}
