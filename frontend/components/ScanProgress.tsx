'use client';

import { useState, useEffect, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import confetti from 'canvas-confetti';
import {
  CheckCircleIcon,
  ExclamationTriangleIcon,
  XCircleIcon,
  ClockIcon,
  DocumentTextIcon,
  CpuChipIcon,
  MagnifyingGlassIcon,
  ClipboardDocumentCheckIcon,
} from '@heroicons/react/24/outline';

interface ScanProgressProps {
  scanId: string;
  onComplete?: (results: any) => void;
  onError?: (error: string) => void;
}

interface ScanState {
  status: string;
  currentStep: string;
  progress: number;
  message: string;
  logs: string[];
  vulnerabilities: any[];
}

const SCAN_STEPS = [
  { key: 'uploading', label: 'Uploading', icon: DocumentTextIcon },
  { key: 'parsing', label: 'Parsing', icon: CpuChipIcon },
  { key: 'fuzzing', label: 'Fuzzing', icon: MagnifyingGlassIcon },
  { key: 'analysis', label: 'Analysis', icon: MagnifyingGlassIcon },
  { key: 'reporting', label: 'Reporting', icon: ClipboardDocumentCheckIcon },
];

export default function ScanProgress({ scanId, onComplete, onError }: ScanProgressProps) {
  const [scanState, setScanState] = useState<ScanState>({
    status: 'pending',
    currentStep: 'uploading',
    progress: 0,
    message: 'Initializing scan...',
    logs: [],
    vulnerabilities: [],
  });
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCompleted, setIsCompleted] = useState(false);
  const [showConfetti, setShowConfetti] = useState(false);
  
  const socketRef = useRef<Socket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  const triggerConfetti = () => {
    setShowConfetti(true);
    confetti({
      particleCount: 100,
      spread: 70,
      origin: { y: 0.6 }
    });
    setTimeout(() => setShowConfetti(false), 5000);
  };

  const connectWebSocket = () => {
    if (socketRef.current?.connected) {
      return;
    }

    const socket = io('/scan-progress', {
      transports: ['websocket', 'polling'],
      timeout: 10000,
      forceNew: true,
    });

    socketRef.current = socket;

    socket.on('connect', () => {
      setIsConnected(true);
      setError(null);
      console.log('Connected to scan progress WebSocket');
      
      // Subscribe to scan updates
      socket.emit('subscribe-scan', { scanId });
    });

    socket.on('disconnect', () => {
      setIsConnected(false);
      console.log('Disconnected from scan progress WebSocket');
      
      // Attempt to reconnect silently after 3 seconds
      reconnectTimeoutRef.current = setTimeout(() => {
        connectWebSocket();
      }, 3000);
    });

    socket.on('connect_error', (err) => {
      console.error('WebSocket connection error:', err);
      setError('Connection error. Attempting to reconnect...');
      setIsConnected(false);
    });

    socket.on('scan-status', (data) => {
      setScanState(prev => ({
        ...prev,
        status: data.status,
        currentStep: data.currentStep || prev.currentStep,
        progress: data.progress || prev.progress,
      }));
    });

    socket.on('scan-progress', (data) => {
      setScanState(prev => ({
        ...prev,
        currentStep: data.currentStep,
        progress: data.progress,
        message: data.message,
      }));
    });

    socket.on('scan-log', (data) => {
      setScanState(prev => ({
        ...prev,
        logs: [...prev.logs, `[${new Date(data.timestamp).toLocaleTimeString()}] ${data.log}`],
      }));
      setTimeout(scrollToBottom, 100);
    });

    socket.on('scan-complete', (data) => {
      setScanState(prev => ({
        ...prev,
        status: 'completed',
        progress: 100,
        currentStep: 'completed',
        vulnerabilities: data.vulnerabilities || [],
        message: 'Scan completed successfully!',
      }));
      setIsCompleted(true);
      
      // Trigger confetti if zero vulnerabilities
      if (data.zeroVulnerabilities) {
        triggerConfetti();
      }
      
      onComplete?.(data);
    });

    socket.on('scan-error', (data) => {
      const errorMessage = data.error || 'An unknown error occurred';
      setError(errorMessage);
      setScanState(prev => ({
        ...prev,
        status: 'failed',
        currentStep: 'error',
        message: errorMessage,
      }));
      onError?.(errorMessage);
    });
  };

  useEffect(() => {
    connectWebSocket();

    return () => {
      // Cleanup WebSocket connection
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      
      if (socketRef.current) {
        socketRef.current.emit('unsubscribe-scan', { scanId });
        socketRef.current.disconnect();
        socketRef.current = null;
      }
    };
  }, [scanId]);

  const getCurrentStepIndex = () => {
    return SCAN_STEPS.findIndex(step => step.key === scanState.currentStep);
  };

  const getStepIcon = (stepKey: string, isActive: boolean, isCompleted: boolean) => {
    const step = SCAN_STEPS.find(s => s.key === stepKey);
    if (!step) return ClockIcon;
    
    const IconComponent = step.icon;
    
    if (isCompleted) {
      return CheckCircleIcon;
    } else if (isActive) {
      return IconComponent;
    } else {
      return ClockIcon;
    }
  };

  const getStepColor = (stepKey: string, isActive: boolean, isCompleted: boolean) => {
    if (isCompleted) return 'text-green-600 bg-green-100';
    if (isActive) return 'text-blue-600 bg-blue-100';
    return 'text-gray-400 bg-gray-100';
  };

  if (error && !isConnected) {
    return (
      <div className="card border-red-200 bg-red-50">
        <div className="flex items-center space-x-3">
          <XCircleIcon className="h-8 w-8 text-red-600" />
          <div>
            <h3 className="text-lg font-semibold text-red-800">Connection Error</h3>
            <p className="text-red-600">{error}</p>
            <button
              onClick={connectWebSocket}
              className="mt-2 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
            >
              Retry Connection
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold text-gray-900">Scan Progress</h2>
          <div className="flex items-center space-x-2">
            <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'} animate-pulse`} />
            <span className="text-sm text-gray-600">
              {isConnected ? 'Connected' : 'Connecting...'}
            </span>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700">{scanState.message}</span>
            <span className="text-sm text-gray-500">{scanState.progress}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-blue-600 h-2 rounded-full transition-all duration-500 ease-out"
              style={{ width: `${scanState.progress}%` }}
            />
          </div>
        </div>

        {/* Step Progress */}
        <div className="flex items-center justify-between mb-8">
          {SCAN_STEPS.map((step, index) => {
            const isActive = step.key === scanState.currentStep;
            const isCompleted = index < getCurrentStepIndex() || isCompleted;
            const IconComponent = getStepIcon(step.key, isActive, isCompleted);
            const colorClass = getStepColor(step.key, isActive, isCompleted);

            return (
              <div key={step.key} className="flex flex-col items-center flex-1">
                <div className={`w-10 h-10 rounded-full flex items-center justify-center ${colorClass} transition-colors duration-300`}>
                  <IconComponent className="w-5 h-5" />
                </div>
                <span className={`text-xs mt-2 text-center ${isActive ? 'font-semibold text-gray-900' : 'text-gray-500'}`}>
                  {step.label}
                </span>
              </div>
            );
          })}
        </div>

        {/* Error State */}
        {error && scanState.status === 'failed' && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-md">
            <div className="flex items-start space-x-3">
              <ExclamationTriangleIcon className="h-5 w-5 text-red-600 mt-0.5" />
              <div>
                <h4 className="font-semibold text-red-800">Scan Failed</h4>
                <p className="text-red-600 text-sm">{error}</p>
                {error.includes('WASM Compilation Error') && (
                  <div className="mt-2 p-3 bg-red-100 rounded-md">
                    <p className="text-red-800 text-sm font-medium">WASM Compilation Error</p>
                    <p className="text-red-700 text-xs mt-1">
                      Please check your contract code for syntax errors and try again.
                    </p>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Success State with Confetti */}
        {isCompleted && scanState.vulnerabilities.length === 0 && (
          <div className="mb-6 p-4 bg-green-50 border border-green-200 rounded-md">
            <div className="flex items-center space-x-3">
              <CheckCircleIcon className="h-5 w-5 text-green-600" />
              <div>
                <h4 className="font-semibold text-green-800">Perfect Security!</h4>
                <p className="text-green-600 text-sm">
                  No vulnerabilities found. Your contract is secure! 🎉
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Terminal-style Logs */}
        <div className="bg-gray-900 rounded-md p-4 h-64 overflow-y-auto">
          <div className="font-mono text-xs text-green-400">
            <div className="mb-2 text-gray-500">$ soroban-scanner --progress</div>
            {scanState.logs.map((log, index) => (
              <div key={index} className="mb-1 text-green-300">
                {log}
              </div>
            ))}
            {scanState.status === 'running' && (
              <div className="animate-pulse text-green-400">▊</div>
            )}
            <div ref={logsEndRef} />
          </div>
        </div>

        {/* Vulnerabilities Summary */}
        {isCompleted && scanState.vulnerabilities.length > 0 && (
          <div className="mt-6 p-4 bg-yellow-50 border border-yellow-200 rounded-md">
            <h4 className="font-semibold text-yellow-800 mb-2">Vulnerabilities Found</h4>
            <div className="text-sm text-yellow-700">
              {scanState.vulnerabilities.length} vulnerabilities detected. 
              Please review the detailed report for remediation steps.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
