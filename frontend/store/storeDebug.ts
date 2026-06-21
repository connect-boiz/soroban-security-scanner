import { devtools } from 'zustand/middleware';
import { subscribeWithSelector } from 'zustand/middleware';
import { create } from 'zustand';

// Enhanced debugging middleware for Zustand stores
export const createDebugStore = <T extends Record<string, any>>(
  initialState: T,
  storeName: string
) => {
  return create<T>()(
    devtools(
      subscribeWithSelector(() => initialState),
      {
        name: storeName,
        // Enable state diff tracking
        trace: true,
        // Enable action logging
        enabled: process.env.NODE_ENV === 'development',
        // Custom serialization for complex objects
        serialize: true,
      }
    )
  );
};

// Performance monitoring utility
export class StorePerformanceMonitor {
  private static instance: StorePerformanceMonitor;
  private metrics: Map<string, { count: number; totalTime: number; lastUpdate: Date }> = new Map();

  static getInstance(): StorePerformanceMonitor {
    if (!StorePerformanceMonitor.instance) {
      StorePerformanceMonitor.instance = new StorePerformanceMonitor();
    }
    return StorePerformanceMonitor.instance;
  }

  recordAction(actionName: string, duration: number): void {
    const current = this.metrics.get(actionName) || {
      count: 0,
      totalTime: 0,
      lastUpdate: new Date(),
    };
    this.metrics.set(actionName, {
      count: current.count + 1,
      totalTime: current.totalTime + duration,
      lastUpdate: new Date(),
    });
  }

  getMetrics(): Record<string, { count: number; avgTime: number; totalTime: number }> {
    const result: Record<string, { count: number; avgTime: number; totalTime: number }> = {};

    this.metrics.forEach((value, key) => {
      result[key] = {
        count: value.count,
        avgTime: value.totalTime / value.count,
        totalTime: value.totalTime,
      };
    });

    return result;
  }

  reset(): void {
    this.metrics.clear();
  }

  logMetrics(): void {
    const metrics = this.getMetrics();
    console.group('Store Performance Metrics');
    Object.entries(metrics).forEach(([action, stats]) => {
      console.log(`${action}:`, {
        calls: stats.count,
        avgTime: `${stats.avgTime.toFixed(2)}ms`,
        totalTime: `${stats.totalTime.toFixed(2)}ms`,
      });
    });
    console.groupEnd();
  }
}

// Action timing wrapper
export const withPerformanceTracking = <T extends (...args: any[]) => any>(
  action: T,
  actionName: string
): T => {
  return ((...args: any[]) => {
    const start = performance.now();
    const result = action(...args);
    const end = performance.now();

    StorePerformanceMonitor.getInstance().recordAction(actionName, end - start);

    return result;
  }) as T;
};

// Store subscription utility for debugging
export const createStoreDebugger = (storeName: string) => {
  return (state: any, prevState: any) => {
    if (process.env.NODE_ENV === 'development') {
      console.group(`Store Change: ${storeName}`);
      console.log('Previous State:', prevState);
      console.log('Current State:', state);

      // Highlight changed properties
      const changes: Record<string, { from: any; to: any }> = {};
      Object.keys(state).forEach(key => {
        if (state[key] !== prevState[key]) {
          changes[key] = {
            from: prevState[key],
            to: state[key],
          };
        }
      });

      if (Object.keys(changes).length > 0) {
        console.log('Changes:', changes);
      }

      console.groupEnd();
    }
  };
};

// State validation utility
export const createStateValidator = <T extends Record<string, any>>(
  schema: Partial<Record<keyof T, (value: any) => boolean>>,
  storeName: string
) => {
  return (state: T): T => {
    if (process.env.NODE_ENV === 'development') {
      const errors: string[] = [];

      Object.entries(schema).forEach(([key, validator]) => {
        if (validator && !validator(state[key as keyof T])) {
          errors.push(`Invalid value for ${key}: ${state[key as keyof T]}`);
        }
      });

      if (errors.length > 0) {
        console.error(`State validation failed for ${storeName}:`, errors);
      }
    }

    return state;
  };
};

// Store hydration utility for SSR compatibility
export const createHydratedStore = <T extends Record<string, any>>(
  initialState: T,
  storeName: string
) => {
  if (typeof window !== 'undefined') {
    // Client-side: create store with persistence
    return create<T>()(
      devtools(
        subscribeWithSelector(() => initialState),
        { name: storeName }
      )
    );
  } else {
    // Server-side: create simple store without persistence
    return create<T>(() => initialState);
  }
};
