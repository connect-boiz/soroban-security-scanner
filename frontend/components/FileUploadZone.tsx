'use client';

import { useRef } from 'react';
import { ProgressBar } from './ui';
import { useFileUpload, FileValidationOptions, UploadedFile, FileStatus } from '../hooks/useFileUpload';

interface FileUploadZoneProps extends FileValidationOptions {
  onFilesReady?: (files: File[]) => void;
  className?: string;
}

const FILE_ICONS: Record<string, string> = {
  '.rs': '🦀',
  '.wasm': '⚙️',
  '.toml': '📄',
  '.txt': '📝',
};

const STATUS_COLORS: Record<FileStatus, string> = {
  pending: 'text-gray-400',
  validating: 'text-blue-500',
  uploading: 'text-blue-600',
  complete: 'text-green-600',
  error: 'text-red-500',
};

const STATUS_LABELS: Record<FileStatus, string> = {
  pending: 'Pending',
  validating: 'Validating…',
  uploading: 'Uploading…',
  complete: 'Ready',
  error: 'Failed',
};

const PROGRESS_COLORS: Record<FileStatus, 'blue' | 'green' | 'red' | 'gray'> = {
  pending: 'gray',
  validating: 'blue',
  uploading: 'blue',
  complete: 'green',
  error: 'red',
};

function fileExt(name: string): string {
  return '.' + name.split('.').pop()?.toLowerCase();
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function FileIcon({ name }: { name: string }) {
  const icon = FILE_ICONS[fileExt(name)] ?? '📁';
  return <span className="text-2xl leading-none select-none">{icon}</span>;
}

function FileRow({
  entry,
  onRemove,
  onRetry,
}: {
  entry: UploadedFile;
  onRemove: (id: string) => void;
  onRetry: (id: string) => void;
}) {
  const { id, file, status, progress, error, preview } = entry;

  return (
    <li className="flex items-start gap-3 p-3 rounded-lg bg-gray-50 border border-gray-100 group">
      {/* Preview or icon */}
      <div className="shrink-0 w-10 h-10 rounded-md overflow-hidden bg-gray-100 flex items-center justify-center border border-gray-200">
        {preview ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={preview} alt={file.name} className="w-full h-full object-cover" />
        ) : (
          <FileIcon name={file.name} />
        )}
      </div>

      {/* Info + progress */}
      <div className="flex-1 min-w-0 space-y-1.5">
        <div className="flex items-center justify-between gap-2">
          <p className="text-sm font-medium text-gray-800 truncate">{file.name}</p>
          <div className="flex items-center gap-2 shrink-0">
            <span className={`text-xs font-medium ${STATUS_COLORS[status]}`}>
              {STATUS_LABELS[status]}
            </span>
            {status === 'error' && (
              <button
                onClick={() => onRetry(id)}
                className="text-xs text-blue-600 hover:underline"
                aria-label={`Retry uploading ${file.name}`}
              >
                Retry
              </button>
            )}
            <button
              onClick={() => onRemove(id)}
              className="opacity-0 group-hover:opacity-100 transition-opacity text-gray-400 hover:text-red-500 text-xs leading-none"
              aria-label={`Remove ${file.name}`}
            >
              ✕
            </button>
          </div>
        </div>

        <p className="text-xs text-gray-400">{formatBytes(file.size)}</p>

        {error && (
          <p className="text-xs text-red-500">{error}</p>
        )}

        {status !== 'pending' && status !== 'error' && (
          <ProgressBar
            value={progress}
            color={PROGRESS_COLORS[status]}
            size="sm"
            animated={status === 'uploading'}
            showLabel={false}
            className="mt-1"
          />
        )}
      </div>
    </li>
  );
}

export default function FileUploadZone({
  onFilesReady,
  className = '',
  maxSizeMB = 10,
  allowedTypes = ['.rs', '.wasm', '.toml', '.txt'],
  maxFiles = 5,
}: FileUploadZoneProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  const {
    files,
    isDragActive,
    canAddMore,
    allComplete,
    onDragEnter,
    onDragOver,
    onDragLeave,
    onDrop,
    onInputChange,
    removeFile,
    retryFile,
    clearAll,
  } = useFileUpload({ maxSizeMB, allowedTypes, maxFiles });

  const readyFiles = files.filter((f) => f.status === 'complete').map((f) => f.file);

  const handleSubmit = () => {
    if (allComplete && onFilesReady) onFilesReady(readyFiles);
  };

  const acceptAttr = allowedTypes
    .map((t) => (t.startsWith('.') ? t : `.${t}`))
    .join(',');

  return (
    <div className={`space-y-4 ${className}`}>
      {/* Drop zone */}
      <div
        role="button"
        tabIndex={0}
        aria-label="File upload area. Drag and drop files or press Enter to browse."
        onDragEnter={onDragEnter}
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onDrop={onDrop}
        onClick={() => canAddMore && inputRef.current?.click()}
        onKeyDown={(e) => {
          if ((e.key === 'Enter' || e.key === ' ') && canAddMore) {
            inputRef.current?.click();
          }
        }}
        className={[
          'relative flex flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed p-10 text-center cursor-pointer select-none transition-all duration-200',
          isDragActive
            ? 'border-blue-500 bg-blue-50 scale-[1.01]'
            : canAddMore
            ? 'border-gray-300 bg-gray-50 hover:border-blue-400 hover:bg-blue-50/40'
            : 'border-gray-200 bg-gray-50 opacity-60 cursor-not-allowed',
        ].join(' ')}
      >
        <input
          ref={inputRef}
          type="file"
          multiple
          accept={acceptAttr}
          className="sr-only"
          onChange={onInputChange}
          disabled={!canAddMore}
          aria-hidden="true"
        />

        <div
          className={`text-4xl transition-transform duration-200 ${isDragActive ? 'scale-125' : ''}`}
        >
          {isDragActive ? '📂' : '📁'}
        </div>

        <div>
          {isDragActive ? (
            <p className="text-base font-semibold text-blue-600">Drop your files here</p>
          ) : canAddMore ? (
            <>
              <p className="text-base font-medium text-gray-700">
                Drag & drop files here, or{' '}
                <span className="text-blue-600 underline underline-offset-2">browse</span>
              </p>
              <p className="text-xs text-gray-400 mt-1">
                {allowedTypes.join(', ')} · max {maxSizeMB}MB · up to {maxFiles} files
              </p>
            </>
          ) : (
            <p className="text-sm font-medium text-gray-500">
              Maximum of {maxFiles} files reached
            </p>
          )}
        </div>
      </div>

      {/* File list */}
      {files.length > 0 && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <p className="text-sm font-medium text-gray-700">
              {files.length} file{files.length !== 1 ? 's' : ''} selected
            </p>
            <button
              onClick={clearAll}
              className="text-xs text-gray-400 hover:text-red-500 transition-colors"
            >
              Clear all
            </button>
          </div>

          <ul className="space-y-2" role="list" aria-label="Uploaded files">
            {files.map((entry) => (
              <FileRow
                key={entry.id}
                entry={entry}
                onRemove={removeFile}
                onRetry={retryFile}
              />
            ))}
          </ul>

          {/* Summary + action */}
          <div className="flex items-center justify-between pt-1">
            <p className="text-xs text-gray-400">
              {files.filter((f) => f.status === 'complete').length} of {files.length} ready
            </p>
            {onFilesReady && (
              <button
                onClick={handleSubmit}
                disabled={!allComplete}
                className="px-4 py-1.5 text-sm font-medium rounded-lg bg-blue-600 text-white hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                Use Files
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
