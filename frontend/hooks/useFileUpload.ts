'use client';

import { useState, useCallback, useRef, DragEvent, ChangeEvent } from 'react';

export type FileStatus = 'pending' | 'validating' | 'uploading' | 'complete' | 'error';

export interface UploadedFile {
  id: string;
  file: File;
  status: FileStatus;
  progress: number;
  error?: string;
  preview?: string;
}

export interface FileValidationOptions {
  maxSizeMB?: number;
  allowedTypes?: string[];
  maxFiles?: number;
}

const DEFAULT_OPTIONS: Required<FileValidationOptions> = {
  maxSizeMB: 10,
  allowedTypes: ['.rs', '.wasm', '.toml', '.txt'],
  maxFiles: 5,
};

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

function validateFile(file: File, options: Required<FileValidationOptions>): string | null {
  const maxBytes = options.maxSizeMB * 1024 * 1024;
  if (file.size > maxBytes) {
    return `File exceeds ${options.maxSizeMB}MB limit`;
  }

  const ext = '.' + file.name.split('.').pop()?.toLowerCase();
  const mime = file.type;

  const isAllowedExt = options.allowedTypes.some(t => (t.startsWith('.') ? t === ext : t === mime));
  if (!isAllowedExt) {
    return `File type "${ext}" is not allowed. Accepted: ${options.allowedTypes.join(', ')}`;
  }

  return null;
}

async function generatePreview(file: File): Promise<string | undefined> {
  if (!file.type.startsWith('image/')) return undefined;
  return new Promise(resolve => {
    const reader = new FileReader();
    reader.onload = e => resolve(e.target?.result as string);
    reader.onerror = () => resolve(undefined);
    reader.readAsDataURL(file);
  });
}

export function useFileUpload(options: FileValidationOptions = {}) {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const [files, setFiles] = useState<UploadedFile[]>([]);
  const [isDragActive, setIsDragActive] = useState(false);
  const abortRefs = useRef<Record<string, AbortController>>({});

  const updateFile = useCallback((id: string, patch: Partial<UploadedFile>) => {
    setFiles((prev: UploadedFile[]) =>
      prev.map((f: UploadedFile) => (f.id === id ? { ...f, ...patch } : f))
    );
  }, []);

  const simulateUpload = useCallback(
    async (id: string, controller: AbortController) => {
      const intervals = [15, 30, 45, 60, 75, 88, 95, 100];
      for (const pct of intervals) {
        if (controller.signal.aborted) return;
        await new Promise(r => setTimeout(r, 250 + Math.random() * 200));
        if (controller.signal.aborted) return;
        updateFile(id, { progress: pct });
      }
      if (!controller.signal.aborted) {
        updateFile(id, { status: 'complete', progress: 100 });
      }
    },
    [updateFile]
  );

  const processFiles = useCallback(
    async (incoming: File[]) => {
      const available = opts.maxFiles - files.length;
      if (available <= 0) return;

      const toProcess = incoming.slice(0, available);

      const newEntries: UploadedFile[] = await Promise.all(
        toProcess.map(async (file: File) => {
          const preview = await generatePreview(file);
          return {
            id: generateId(),
            file,
            status: 'pending' as FileStatus,
            progress: 0,
            preview,
          };
        })
      );

      setFiles((prev: UploadedFile[]) => [...prev, ...newEntries]);

      for (const entry of newEntries) {
        updateFile(entry.id, { status: 'validating' });
        await new Promise(r => setTimeout(r, 150));

        const error = validateFile(entry.file, opts);
        if (error) {
          updateFile(entry.id, { status: 'error', error });
          continue;
        }

        const controller = new AbortController();
        abortRefs.current[entry.id] = controller;
        updateFile(entry.id, { status: 'uploading', progress: 0 });
        simulateUpload(entry.id, controller);
      }
    },
    [files.length, opts, updateFile, simulateUpload]
  );

  const onDragEnter = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragActive(true);
  }, []);

  const onDragOver = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const onDragLeave = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault();
    e.stopPropagation();
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
      setIsDragActive(false);
    }
  }, []);

  const onDrop = useCallback(
    (e: DragEvent<HTMLElement>) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragActive(false);
      const dropped = Array.from(e.dataTransfer.files);
      if (dropped.length) processFiles(dropped);
    },
    [processFiles]
  );

  const onInputChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      const selected = Array.from(e.target.files ?? []);
      if (selected.length) processFiles(selected);
      e.target.value = '';
    },
    [processFiles]
  );

  const removeFile = useCallback((id: string) => {
    abortRefs.current[id]?.abort();
    delete abortRefs.current[id];
    setFiles((prev: UploadedFile[]) => {
      const target = prev.find((f: UploadedFile) => f.id === id);
      if (target?.preview) URL.revokeObjectURL(target.preview);
      return prev.filter((f: UploadedFile) => f.id !== id);
    });
  }, []);

  const retryFile = useCallback(
    (id: string) => {
      setFiles((prev: UploadedFile[]) => {
        const target = prev.find((f: UploadedFile) => f.id === id);
        if (!target) return prev;
        const error = validateFile(target.file, opts);
        if (error) return prev;
        return prev.map((f: UploadedFile) =>
          f.id === id
            ? { ...f, status: 'uploading' as FileStatus, progress: 0, error: undefined }
            : f
        );
      });
      const controller = new AbortController();
      abortRefs.current[id] = controller;
      simulateUpload(id, controller);
    },
    [opts, simulateUpload]
  );

  const clearAll = useCallback(() => {
    const controllers = Object.keys(abortRefs.current).map(k => abortRefs.current[k]);
    controllers.forEach((c: AbortController) => c.abort());
    abortRefs.current = {};
    setFiles((prev: UploadedFile[]) => {
      prev.forEach((f: UploadedFile) => f.preview && URL.revokeObjectURL(f.preview));
      return [];
    });
  }, []);

  const canAddMore = files.length < opts.maxFiles;
  const allComplete = files.length > 0 && files.every((f: UploadedFile) => f.status === 'complete');

  return {
    files,
    isDragActive,
    canAddMore,
    allComplete,
    maxFiles: opts.maxFiles,
    allowedTypes: opts.allowedTypes,
    maxSizeMB: opts.maxSizeMB,
    onDragEnter,
    onDragOver,
    onDragLeave,
    onDrop,
    onInputChange,
    removeFile,
    retryFile,
    clearAll,
  };
}
