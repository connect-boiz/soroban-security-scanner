'use client';

import { useState, useCallback, useEffect } from 'react';

interface LoadingState {
  isLoading: boolean;
  progress: number;
  stage: string;
  error: string | null;
}

interface LoadingStage {
  name: string;
  duration: number;
  progress: number;
}

interface UseLoadingStatesOptions {
  stages?: LoadingStage[];
  onError?: (error: string) => void;
  onComplete?: () => void;
  onProgress?: (progress: number, stage: string) => void;
}

export const useLoadingStates = (options: UseLoadingStatesOptions = {}) => {
  const [loadingState, setLoadingState] = useState<LoadingState>({
    isLoading: false,
    progress: 0,
    stage: '',
    error: null
  });

  const [isCancelled, setIsCancelled] = useState(false);

  const startLoading = useCallback(async () => {
    setIsCancelled(false);
    setLoadingState({
      isLoading: true,
      progress: 0,
      stage: 'Initializing...',
      error: null
    });

    if (!options.stages) {
      return;
    }

    try {
      for (const stage of options.stages) {
        if (isCancelled) {
          throw new Error('Operation was cancelled');
        }

        setLoadingState(prev => ({
          ...prev,
          stage: stage.name,
          progress: stage.progress
        }));

        options.onProgress?.(stage.progress, stage.name);

        await new Promise(resolve => setTimeout(resolve, stage.duration));
      }

      if (!isCancelled) {
        setLoadingState({
          isLoading: false,
          progress: 100,
          stage: 'Complete',
          error: null
        });
        options.onComplete?.();
      }
    } catch (error) {
      if (!isCancelled) {
        const errorMessage = error instanceof Error ? error.message : 'An error occurred';
        setLoadingState(prev => ({
          ...prev,
          isLoading: false,
          error: errorMessage
        }));
        options.onError?.(errorMessage);
      }
    }
  }, [options.stages, isCancelled, options.onProgress, options.onComplete, options.onError]);

  const cancelLoading = useCallback(() => {
    setIsCancelled(true);
    setLoadingState(prev => ({
      ...prev,
      isLoading: false,
      stage: 'Cancelled',
      error: 'Operation was cancelled'
    }));
  }, []);

  const resetLoading = useCallback(() => {
    setIsCancelled(false);
    setLoadingState({
      isLoading: false,
      progress: 0,
      stage: '',
      error: null
    });
  }, []);

  const updateProgress = useCallback((progress: number, stage: string) => {
    setLoadingState(prev => ({
      ...prev,
      progress: Math.min(100, Math.max(0, progress)),
      stage
    }));
    options.onProgress?.(progress, stage);
  }, [options.onProgress]);

  const setLoading = useCallback((loading: boolean) => {
    setLoadingState(prev => ({
      ...prev,
      isLoading: loading
    }));
  }, []);

  const setError = useCallback((error: string | null) => {
    setLoadingState(prev => ({
      ...prev,
      error,
      isLoading: false
    }));
    if (error) {
      options.onError?.(error);
    }
  }, [options.onError]);

  return {
    ...loadingState,
    startLoading,
    cancelLoading,
    resetLoading,
    updateProgress,
    setLoading,
    setError
  };
};

// Hook for async operations with loading states
interface UseAsyncOperationOptions<T> {
  operation: () => Promise<T>;
  onSuccess?: (result: T) => void;
  onError?: (error: string) => void;
  loadingMessage?: string;
}

export const useAsyncOperation = <T,>() => {
  const [state, setState] = useState<{
    isLoading: boolean;
    data: T | null;
    error: string | null;
  }>({
    isLoading: false,
    data: null,
    error: null
  });

  const execute = useCallback(async (options: UseAsyncOperationOptions<T>) => {
    setState({ isLoading: true, data: null, error: null });

    try {
      const result = await options.operation();
      setState({ isLoading: false, data: result, error: null });
      options.onSuccess?.(result);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'An error occurred';
      setState({ isLoading: false, data: null, error: errorMessage });
      options.onError?.(errorMessage);
    }
  }, []);

  const reset = useCallback(() => {
    setState({ isLoading: false, data: null, error: null });
  }, []);

  return {
    ...state,
    execute,
    reset
  };
};

// Hook for staged loading with progress tracking
interface UseStagedLoadingOptions {
  stages: Array<{
    name: string;
    action: () => Promise<void>;
    progressWeight: number;
  }>;
  onStageComplete?: (stageName: string) => void;
  onComplete?: () => void;
  onError?: (error: string, stageName: string) => void;
}

export const useStagedLoading = (options: UseStagedLoadingOptions) => {
  const [state, setState] = useState<{
    isLoading: boolean;
    currentStage: string;
    progress: number;
    completedStages: string[];
    error: string | null;
  }>({
    isLoading: false,
    currentStage: '',
    progress: 0,
    completedStages: [],
    error: null
  });

  const execute = useCallback(async () => {
    setState({
      isLoading: true,
      currentStage: 'Starting...',
      progress: 0,
      completedStages: [],
      error: null
    });

    let accumulatedProgress = 0;

    try {
      for (const stage of options.stages) {
        setState(prev => ({
          ...prev,
          currentStage: stage.name
        }));

        await stage.action();
        
        accumulatedProgress += stage.progressWeight;
        
        setState(prev => ({
          ...prev,
          progress: Math.min(100, accumulatedProgress),
          completedStages: [...prev.completedStages, stage.name]
        }));

        options.onStageComplete?.(stage.name);
      }

      setState({
        isLoading: false,
        currentStage: 'Complete',
        progress: 100,
        completedStages: options.stages.map(s => s.name),
        error: null
      });

      options.onComplete?.();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'An error occurred';
      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMessage
      }));
      options.onError?.(errorMessage, state.currentStage);
    }
  }, [options]);

  const reset = useCallback(() => {
    setState({
      isLoading: false,
      currentStage: '',
      progress: 0,
      completedStages: [],
      error: null
    });
  }, []);

  return {
    ...state,
    execute,
    reset
  };
};

// Hook for retry operations with loading states
interface UseRetryOperationOptions<T> {
  maxRetries?: number;
  retryDelay?: number;
  operation: () => Promise<T>;
  onSuccess?: (result: T) => void;
  onError?: (error: string, attempt: number) => void;
}

export const useRetryOperation = <T,>(options: UseRetryOperationOptions<T>) => {
  const [state, setState] = useState<{
    isLoading: boolean;
    data: T | null;
    error: string | null;
    attempt: number;
  }>({
    isLoading: false,
    data: null,
    error: null,
    attempt: 0
  });

  const execute = useCallback(async () => {
    const maxRetries = options.maxRetries || 3;
    const retryDelay = options.retryDelay || 1000;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      setState(prev => ({
        ...prev,
        isLoading: true,
        attempt,
        error: null
      }));

      try {
        const result = await options.operation();
        setState({
          isLoading: false,
          data: result,
          error: null,
          attempt
        });
        options.onSuccess?.(result);
        return;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'An error occurred';
        
        if (attempt === maxRetries) {
          setState({
            isLoading: false,
            data: null,
            error: errorMessage,
            attempt
          });
          options.onError?.(errorMessage, attempt);
        } else {
          setState(prev => ({
            ...prev,
            error: `${errorMessage} (Retrying... ${attempt}/${maxRetries})`
          }));
          await new Promise(resolve => setTimeout(resolve, retryDelay));
        }
      }
    }
  }, [options]);

  const reset = useCallback(() => {
    setState({
      isLoading: false,
      data: null,
      error: null,
      attempt: 0
    });
  }, []);

  return {
    ...state,
    execute,
    reset
  };
};
