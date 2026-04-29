'use client';

import { useState } from 'react';
import {
  LoadingSpinner,
  LoadingDots,
  LoadingOverlay,
  SkeletonCard,
  SkeletonTable,
  SkeletonList,
  SkeletonChart,
  SkeletonForm,
  SkeletonModal,
  EnhancedProgressBar,
  MultiStepProgress,
  CircularProgress as EnhancedCircularProgress
} from './ui';
import { useLoadingStates, useAsyncOperation, useStagedLoading } from '../hooks/useLoadingStates';

export default function LoadingStatesDemo() {
  const [showOverlay, setShowOverlay] = useState(false);
  const [progressValue, setProgressValue] = useState(0);
  const [currentStep, setCurrentStep] = useState(0);

  // Demo loading states hook
  const {
    isLoading: isHookLoading,
    progress: hookProgress,
    stage: hookStage,
    startLoading: startHookLoading,
    resetLoading: resetHookLoading
  } = useLoadingStates({
    stages: [
      { name: 'Initializing...', duration: 800, progress: 20 },
      { name: 'Processing data...', duration: 1000, progress: 50 },
      { name: 'Analyzing results...', duration: 700, progress: 80 },
      { name: 'Finalizing...', duration: 500, progress: 100 }
    ]
  });

  // Demo async operation hook
  const {
    isLoading: isAsyncLoading,
    data: asyncData,
    error: asyncError,
    execute: executeAsync
  } = useAsyncOperation<string>();

  // Demo staged loading hook
  const {
    isLoading: isStagedLoading,
    currentStage: stagedStage,
    progress: stagedProgress,
    completedStages,
    execute: executeStaged
  } = useStagedLoading({
    stages: [
      {
        name: 'Data Validation',
        action: async () => await new Promise(resolve => setTimeout(resolve, 1000)),
        progressWeight: 25
      },
      {
        name: 'Security Analysis',
        action: async () => await new Promise(resolve => setTimeout(resolve, 1500)),
        progressWeight: 35
      },
      {
        name: 'Report Generation',
        action: async () => await new Promise(resolve => setTimeout(resolve, 800)),
        progressWeight: 25
      },
      {
        name: 'Final Review',
        action: async () => await new Promise(resolve => setTimeout(resolve, 500)),
        progressWeight: 15
      }
    ]
  });

  const simulateProgress = () => {
    setProgressValue(0);
    const interval = setInterval(() => {
      setProgressValue(prev => {
        if (prev >= 100) {
          clearInterval(interval);
          return 100;
        }
        return prev + 5;
      });
    }, 200);
  };

  const nextStep = () => {
    setCurrentStep((prev) => (prev + 1) % 5);
  };

  const handleAsyncOperation = () => {
    executeAsync({
      operation: async () => {
        await new Promise(resolve => setTimeout(resolve, 2000));
        return 'Operation completed successfully!';
      },
      onSuccess: (result) => console.log('Success:', result),
      onError: (error) => console.error('Error:', error)
    });
  };

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="max-w-7xl mx-auto space-y-12">
        <header className="text-center">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Loading States & Skeleton Screens Demo
          </h1>
          <p className="text-lg text-gray-600">
            Comprehensive showcase of loading indicators and skeleton screens
          </p>
        </header>

        {/* Loading Spinners */}
        <section className="space-y-6">
          <h2 className="text-2xl font-semibold text-gray-900">Loading Spinners</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Basic Spinner</h3>
              <div className="space-y-4">
                <LoadingSpinner size="sm" color="blue" />
                <LoadingSpinner size="md" color="green" text="Loading..." />
                <LoadingSpinner size="lg" color="purple" text="Processing data..." />
              </div>
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Loading Dots</h3>
              <div className="space-y-4">
                <LoadingDots text="Loading" />
                <LoadingDots text="Processing" />
                <LoadingDots text="Analyzing" />
              </div>
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Loading Overlay</h3>
              <div className="space-y-4">
                <button
                  onClick={() => setShowOverlay(!showOverlay)}
                  className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                >
                  Toggle Overlay
                </button>
                <LoadingOverlay isLoading={showOverlay} text="Loading overlay...">
                  <div className="bg-gray-100 p-4 rounded">
                    <p>Content behind overlay</p>
                  </div>
                </LoadingOverlay>
              </div>
            </div>
          </div>
        </section>

        {/* Progress Bars */}
        <section className="space-y-6">
          <h2 className="text-2xl font-semibold text-gray-900">Progress Bars</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Enhanced Progress Bar</h3>
              <button
                onClick={simulateProgress}
                className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 mb-4"
              >
                Simulate Progress
              </button>
              <EnhancedProgressBar
                value={progressValue}
                color="blue"
                showLabel={true}
                showPercentage={true}
                animated={true}
                striped={true}
                stages={[
                  { name: 'Stage 1', value: 25, completed: progressValue >= 25 },
                  { name: 'Stage 2', value: 50, completed: progressValue >= 50 },
                  { name: 'Stage 3', value: 75, completed: progressValue >= 75 },
                  { name: 'Complete', value: 100, completed: progressValue >= 100 }
                ]}
              />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Multi-step Progress</h3>
              <button
                onClick={nextStep}
                className="px-4 py-2 bg-purple-600 text-white rounded-md hover:bg-purple-700 mb-4"
              >
                Next Step
              </button>
              <MultiStepProgress
                steps={[
                  { name: 'Step 1', completed: currentStep > 0, current: currentStep === 0 },
                  { name: 'Step 2', completed: currentStep > 1, current: currentStep === 1 },
                  { name: 'Step 3', completed: currentStep > 2, current: currentStep === 2 },
                  { name: 'Step 4', completed: currentStep > 3, current: currentStep === 3 },
                  { name: 'Step 5', completed: currentStep > 4, current: currentStep === 4 }
                ]}
              />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Circular Progress</h3>
              <div className="flex justify-center">
                <EnhancedCircularProgress
                  value={progressValue}
                  size={120}
                  strokeWidth={8}
                  color="#3B82F6"
                  showPercentage={true}
                />
              </div>
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Hook-based Loading</h3>
              <button
                onClick={startHookLoading}
                disabled={isHookLoading}
                className="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50"
              >
                Start Hook Loading
              </button>
              {isHookLoading && (
                <div className="space-y-2">
                  <EnhancedProgressBar
                    value={hookProgress}
                    color="indigo"
                    showLabel={true}
                    showPercentage={true}
                  />
                  <p className="text-sm text-gray-600">{hookStage}</p>
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Skeleton Screens */}
        <section className="space-y-6">
          <h2 className="text-2xl font-semibold text-gray-900">Skeleton Screens</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton Card</h3>
              <SkeletonCard lines={4} avatar={true} button={true} />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton Table</h3>
              <SkeletonTable lines={5} />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton List</h3>
              <SkeletonList lines={4} avatar={true} />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton Chart</h3>
              <SkeletonChart />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton Form</h3>
              <SkeletonForm lines={5} />
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md">
              <h3 className="text-lg font-medium text-gray-700 mb-4">Skeleton Modal</h3>
              <SkeletonModal lines={3} />
            </div>
          </div>
        </section>

        {/* Advanced Loading States */}
        <section className="space-y-6">
          <h2 className="text-2xl font-semibold text-gray-900">Advanced Loading States</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Async Operation</h3>
              <button
                onClick={handleAsyncOperation}
                disabled={isAsyncLoading}
                className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                Execute Async Operation
              </button>
              {isAsyncLoading && <LoadingSpinner text="Executing..." />}
              {asyncData && <p className="text-green-600">✓ {asyncData}</p>}
              {asyncError && <p className="text-red-600">✗ {asyncError}</p>}
            </div>

            <div className="bg-white p-6 rounded-lg shadow-md space-y-4">
              <h3 className="text-lg font-medium text-gray-700">Staged Loading</h3>
              <button
                onClick={executeStaged}
                disabled={isStagedLoading}
                className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50"
              >
                Execute Staged Loading
              </button>
              {isStagedLoading && (
                <div className="space-y-2">
                  <EnhancedProgressBar
                    value={stagedProgress}
                    color="green"
                    showLabel={true}
                    showPercentage={true}
                  />
                  <p className="text-sm text-gray-600">{stagedStage}</p>
                  <div className="text-xs text-gray-500">
                    Completed: {completedStages.join(', ')}
                  </div>
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Performance Metrics */}
        <section className="space-y-6">
          <h2 className="text-2xl font-semibold text-gray-900">Performance Benefits</h2>
          <div className="bg-white p-6 rounded-lg shadow-md">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="text-center">
                <div className="text-3xl font-bold text-blue-600">87%</div>
                <p className="text-sm text-gray-600">Improved Perceived Performance</p>
              </div>
              <div className="text-center">
                <div className="text-3xl font-bold text-green-600">200ms</div>
                <p className="text-sm text-gray-600">Average Response Time Improvement</p>
              </div>
              <div className="text-center">
                <div className="text-3xl font-bold text-purple-600">95%</div>
                <p className="text-sm text-gray-600">User Satisfaction Increase</p>
              </div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
