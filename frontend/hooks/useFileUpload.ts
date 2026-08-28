'use client';

import { useState, useCallback, useRef, DragEvent, ChangeEvent } from 'react';

export type FileStatus =
  | 'pending'
  | 'validating'
  | 'uploading'
  | 'complete'
  | 'error'
  | 'cancelled';

export interface UploadedFile {
  id: string;
  file: File;
  status: FileStatus;
  progress: number;
  error?: string;
  preview?: string;
  /** Instantaneous upload speed in bytes/second (populated while uploading). */
  speedBps?: number;
  /** Estimated seconds remaining until the upload completes. */
  etaSeconds?: number;
}

export interface FileValidationOptions {
  maxSizeMB?: number;
  allowedTypes?: string[];
  maxFiles?: number;
  /**
   * Per-extension size limits in MB. Takes precedence over `maxSizeMB` for a
   * matching extension (e.g. compiled WASM binaries are allowed to be larger
   * than the plain-text Rust sources they are built from).
   */
  maxSizeByExt?: Record<string, number>;
}

const DEFAULT_OPTIONS: Required<FileValidationOptions> = {
  maxSizeMB: 10,
  allowedTypes: ['.rs', '.wasm', '.toml', '.txt'],
  maxFiles: 5,
  maxSizeByExt: { '.wasm': 10, '.rs': 5 },
};

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/** Lower-cased extension including the leading dot, e.g. `".wasm"`. */
export function getExt(name: string): string {
  return '.' + (name.split('.').pop()?.toLowerCase() ?? '');
}

/** Magic bytes every WebAssembly binary starts with: `\0asm`. */
export const WASM_MAGIC = [0x00, 0x61, 0x73, 0x6d];

// Tokens a Rust source file's first non-empty line is expected to start with.
const RUST_FIRST_LINE_TOKENS = [
  'use',
  'pub',
  'mod',
  'fn',
  'impl',
  'trait',
  'struct',
  'enum',
  'const',
  'static',
  'type',
  'let',
  'unsafe',
  'async',
  'extern',
  '//',
  '//!',
  '///',
  '/*',
  '#!',
  '#[',
];

/**
 * Read the first `n` bytes of a file. Prefers `Blob.arrayBuffer()` and falls
 * back to `FileReader.readAsArrayBuffer()` on engines without it. Reads only a
 * slice, so it stays cheap even for very large files.
 */
export async function readHeadBytes(file: File, n: number): Promise<Uint8Array> {
  const blob = file.slice(0, n);
  if (typeof blob.arrayBuffer === 'function') {
    return new Uint8Array(await blob.arrayBuffer());
  }
  return new Promise<Uint8Array>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.onerror = () => reject(reader.error ?? new Error('Failed to read file'));
    reader.readAsArrayBuffer(blob);
  });
}

// Decode bytes as ASCII, mapping anything outside printable ASCII + common
// whitespace to a replacement char. This is deliberately dependency-free (no
// TextDecoder) and only used to inspect a source file's first line, whose Rust
// keywords are always ASCII. Binary content collapses to replacement chars and
// therefore never matches a Rust token.
function bytesToAscii(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i++) {
    const c = bytes[i];
    const printable = c === 9 || c === 10 || c === 13 || (c >= 32 && c <= 126);
    out += printable ? String.fromCharCode(c) : '�';
  }
  return out;
}

function firstNonEmptyLine(text: string): string {
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line) return line;
  }
  return '';
}

function lineLooksLikeRust(line: string): boolean {
  return RUST_FIRST_LINE_TOKENS.some(tok => {
    if (!line.startsWith(tok)) return false;
    // Require a word boundary for keyword tokens so `fn` does not match `fnord`;
    // symbol tokens (`//`, `#[`, …) match as-is.
    if (!/^[a-z]/.test(tok)) return true;
    const next = line.charAt(tok.length);
    return next === '' || !/[A-Za-z0-9_]/.test(next);
  });
}

/**
 * Content-based validation using magic bytes / first-line heuristics, so a file
 * renamed to bypass the extension check (e.g. `malware.exe` → `contract.wasm`)
 * is rejected client-side before any upload begins. Returns an error message,
 * or `null` when the content is consistent with the claimed type.
 */
export async function detectContentError(file: File, ext: string): Promise<string | null> {
  if (file.size === 0) {
    return 'File is empty';
  }

  if (ext === '.wasm') {
    const head = await readHeadBytes(file, WASM_MAGIC.length);
    const valid =
      head.length >= WASM_MAGIC.length && WASM_MAGIC.every((byte, i) => head[i] === byte);
    return valid ? null : 'This file does not appear to be a valid WASM binary';
  }

  if (ext === '.rs') {
    const head = await readHeadBytes(file, 512);
    const firstLine = firstNonEmptyLine(bytesToAscii(head));
    return lineLooksLikeRust(firstLine)
      ? null
      : 'This file does not appear to be valid Rust source';
  }

  // .toml/.txt (and any other allowed type) have no reliable magic signature.
  return null;
}

function resolveMaxSizeMB(file: File, options: Required<FileValidationOptions>): number {
  return options.maxSizeByExt[getExt(file.name)] ?? options.maxSizeMB;
}

function validateFile(file: File, options: Required<FileValidationOptions>): string | null {
  const ext = getExt(file.name);
  const mime = file.type;

  const isAllowedExt = options.allowedTypes.some(t => (t.startsWith('.') ? t === ext : t === mime));
  if (!isAllowedExt) {
    return `File type "${ext}" is not allowed. Accepted: ${options.allowedTypes.join(', ')}`;
  }

  const maxMB = resolveMaxSizeMB(file, options);
  if (file.size > maxMB * 1024 * 1024) {
    return `File exceeds the ${maxMB}MB size limit`;
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
  const opts: Required<FileValidationOptions> = {
    ...DEFAULT_OPTIONS,
    ...options,
    maxSizeByExt: options.maxSizeByExt ?? DEFAULT_OPTIONS.maxSizeByExt,
  };
  const [files, setFiles] = useState<UploadedFile[]>([]);
  const [isDragActive, setIsDragActive] = useState(false);
  const abortRefs = useRef<Record<string, AbortController>>({});

  const updateFile = useCallback((id: string, patch: Partial<UploadedFile>) => {
    setFiles((prev: UploadedFile[]) =>
      prev.map((f: UploadedFile) => (f.id === id ? { ...f, ...patch } : f))
    );
  }, []);

  const simulateUpload = useCallback(
    async (id: string, controller: AbortController, fileSize: number) => {
      const startedAt = Date.now();
      const intervals = [15, 30, 45, 60, 75, 88, 95, 100];
      for (const pct of intervals) {
        if (controller.signal.aborted) return;
        await new Promise(r => setTimeout(r, 250 + Math.random() * 200));
        if (controller.signal.aborted) return;
        const elapsedSec = Math.max((Date.now() - startedAt) / 1000, 0.001);
        const bytesSoFar = (fileSize * pct) / 100;
        const speedBps = bytesSoFar / elapsedSec;
        const etaSeconds = speedBps > 0 ? (fileSize - bytesSoFar) / speedBps : 0;
        updateFile(id, { progress: pct, speedBps, etaSeconds });
      }
      if (!controller.signal.aborted) {
        updateFile(id, { status: 'complete', progress: 100, speedBps: undefined, etaSeconds: 0 });
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

        const contentError = await detectContentError(entry.file, getExt(entry.file.name));
        if (contentError) {
          updateFile(entry.id, { status: 'error', error: contentError });
          continue;
        }

        const controller = new AbortController();
        abortRefs.current[entry.id] = controller;
        updateFile(entry.id, { status: 'uploading', progress: 0 });
        simulateUpload(entry.id, controller, entry.file.size);
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

  const cancelUpload = useCallback(
    (id: string) => {
      abortRefs.current[id]?.abort();
      delete abortRefs.current[id];
      updateFile(id, {
        status: 'cancelled',
        error: 'Upload cancelled',
        speedBps: undefined,
        etaSeconds: undefined,
      });
    },
    [updateFile]
  );

  const retryFile = useCallback(
    async (id: string) => {
      const target = files.find((f: UploadedFile) => f.id === id);
      if (!target) return;

      const error = validateFile(target.file, opts);
      if (error) {
        updateFile(id, { status: 'error', error });
        return;
      }

      const contentError = await detectContentError(target.file, getExt(target.file.name));
      if (contentError) {
        updateFile(id, { status: 'error', error: contentError });
        return;
      }

      updateFile(id, {
        status: 'uploading',
        progress: 0,
        error: undefined,
        speedBps: undefined,
        etaSeconds: undefined,
      });
      const controller = new AbortController();
      abortRefs.current[id] = controller;
      simulateUpload(id, controller, target.file.size);
    },
    [files, opts, updateFile, simulateUpload]
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
    cancelUpload,
    retryFile,
    clearAll,
  };
}
