'use client';

import { useEffect, useState, useCallback } from 'react';

interface PerformanceMetrics {
  firstContentfulPaint?: number;
  largestContentfulPaint?: number;
  firstInputDelay?: number;
  cumulativeLayoutShift?: number;
  timeToInteractive?: number;
}

export function usePerformanceMonitoring() {
  const [metrics, setMetrics] = useState<PerformanceMetrics>({});
  const [isMonitoring, setIsMonitoring] = useState(false);

  const measurePageLoad = useCallback(() => {
    if (typeof window === 'undefined' || !window.performance) return;

    const navigation = window.performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
    
    const pageLoadMetrics = {
      domContentLoaded: navigation.domContentLoadedEventEnd - navigation.domContentLoadedEventStart,
      loadComplete: navigation.loadEventEnd - navigation.loadEventStart,
      firstPaint: 0,
      firstContentfulPaint: 0,
    };

    // Get paint metrics
    const paintEntries = window.performance.getEntriesByType('paint');
    paintEntries.forEach((entry) => {
      if (entry.name === 'first-paint') {
        pageLoadMetrics.firstPaint = entry.startTime;
      }
      if (entry.name === 'first-contentful-paint') {
        pageLoadMetrics.firstContentfulPaint = entry.startTime;
      }
    });

    return pageLoadMetrics;
  }, []);

  const observeWebVitals = useCallback(() => {
    if (typeof window === 'undefined') return;

    // Observe Largest Contentful Paint
    if ('PerformanceObserver' in window) {
      try {
        const lcpObserver = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const lastEntry = entries[entries.length - 1];
          setMetrics(prev => ({
            ...prev,
            largestContentfulPaint: lastEntry.startTime
          }));
        });
        lcpObserver.observe({ entryTypes: ['largest-contentful-paint'] });

        // Observe First Input Delay
        const fidObserver = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          entries.forEach((entry: any) => {
            setMetrics(prev => ({
              ...prev,
              firstInputDelay: entry.processingStart - entry.startTime
            }));
          });
        });
        fidObserver.observe({ entryTypes: ['first-input'] });

        // Observe Cumulative Layout Shift
        let clsValue = 0;
        const clsObserver = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          entries.forEach((entry: any) => {
            if (!entry.hadRecentInput) {
              clsValue += entry.value;
            }
          });
          setMetrics(prev => ({
            ...prev,
            cumulativeLayoutShift: clsValue
          }));
        });
        clsObserver.observe({ entryTypes: ['layout-shift'] });
      } catch (e) {
        console.warn('Performance Observer not fully supported:', e);
      }
    }
  }, []);

  const startMonitoring = useCallback(() => {
    setIsMonitoring(true);
    
    // Measure initial page load
    const pageMetrics = measurePageLoad();
    if (pageMetrics) {
      setMetrics(prev => ({
        ...prev,
        firstContentfulPaint: pageMetrics.firstContentfulPaint
      }));
    }

    // Start observing Web Vitals
    observeWebVitals();
  }, [measurePageLoad, observeWebVitals]);

  const stopMonitoring = useCallback(() => {
    setIsMonitoring(false);
  }, []);

  const logMetrics = useCallback(() => {
    console.log('Performance Metrics:', metrics);
    
    // Log warnings for poor performance
    if (metrics.firstContentfulPaint && metrics.firstContentfulPaint > 2000) {
      console.warn('First Contentful Paint is slow (>2s):', metrics.firstContentfulPaint);
    }
    
    if (metrics.largestContentfulPaint && metrics.largestContentfulPaint > 2500) {
      console.warn('Largest Contentful Paint is slow (>2.5s):', metrics.largestContentfulPaint);
    }
    
    if (metrics.firstInputDelay && metrics.firstInputDelay > 100) {
      console.warn('First Input Delay is high (>100ms):', metrics.firstInputDelay);
    }
    
    if (metrics.cumulativeLayoutShift && metrics.cumulativeLayoutShift > 0.1) {
      console.warn('Cumulative Layout Shift is high (>0.1):', metrics.cumulativeLayoutShift);
    }
  }, [metrics]);

  useEffect(() => {
    if (isMonitoring) {
      logMetrics();
    }
  }, [metrics, isMonitoring, logMetrics]);

  return {
    metrics,
    isMonitoring,
    startMonitoring,
    stopMonitoring,
    logMetrics
  };
}
