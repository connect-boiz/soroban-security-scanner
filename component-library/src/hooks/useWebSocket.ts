import { useEffect, useRef, useState, useCallback } from 'react';

type Status = 'connecting' | 'open' | 'closed' | 'error';

interface UseWebSocketOptions {
  onMessage?: (data: unknown) => void;
  onOpen?: () => void;
  onClose?: () => void;
  reconnectDelay?: number;
  maxRetries?: number;
}

export function useWebSocket(url: string, options: UseWebSocketOptions = {}) {
  const { onMessage, onOpen, onClose, reconnectDelay = 3000, maxRetries = 5 } = options;
  const [status, setStatus] = useState<Status>('connecting');
  const wsRef = useRef<WebSocket | null>(null);
  const retriesRef = useRef(0);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    const ws = new WebSocket(url);
    wsRef.current = ws;
    setStatus('connecting');

    ws.onopen = () => {
      setStatus('open');
      retriesRef.current = 0;
      onOpen?.();
    };

    ws.onmessage = (e) => {
      try {
        onMessage?.(JSON.parse(e.data));
      } catch {
        onMessage?.(e.data);
      }
    };

    ws.onclose = () => {
      setStatus('closed');
      onClose?.();
      if (retriesRef.current < maxRetries) {
        retriesRef.current++;
        timeoutRef.current = setTimeout(connect, reconnectDelay);
      }
    };

    ws.onerror = () => setStatus('error');
  }, [url, onMessage, onOpen, onClose, reconnectDelay, maxRetries]);

  useEffect(() => {
    connect();
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const send = useCallback((data: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(data));
    }
  }, []);

  return { status, send };
}
