// Coverage for barrel/index modules: importing them executes the re-export
// statements, and the named exports must resolve to real components.
import { PortfolioChart, TransactionChart, PerformanceChart } from '../components/charts';
import {
  ProgressBar,
  CircularProgress,
  LoadingSpinner,
  LoadingDots,
  LoadingOverlay,
  SkeletonLoader,
  SkeletonCard,
  SkeletonTable,
  SkeletonList,
  SkeletonChart,
  SkeletonForm,
  SkeletonModal,
  ErrorBoundary,
  EnhancedProgressBar,
  MultiStepProgress,
} from '../components/ui';

describe('barrel re-exports', () => {
  it('re-exports the chart components', () => {
    expect(PortfolioChart).toBeDefined();
    expect(TransactionChart).toBeDefined();
    expect(PerformanceChart).toBeDefined();
  });

  it('re-exports the UI components', () => {
    expect(ProgressBar).toBeDefined();
    expect(CircularProgress).toBeDefined();
    expect(LoadingSpinner).toBeDefined();
    expect(LoadingDots).toBeDefined();
    expect(LoadingOverlay).toBeDefined();
    expect(SkeletonLoader).toBeDefined();
    expect(SkeletonCard).toBeDefined();
    expect(SkeletonTable).toBeDefined();
    expect(SkeletonList).toBeDefined();
    expect(SkeletonChart).toBeDefined();
    expect(SkeletonForm).toBeDefined();
    expect(SkeletonModal).toBeDefined();
    expect(ErrorBoundary).toBeDefined();
    expect(EnhancedProgressBar).toBeDefined();
    expect(MultiStepProgress).toBeDefined();
  });
});
