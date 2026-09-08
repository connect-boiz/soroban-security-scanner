import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import FileUploadZone from '../components/FileUploadZone';
import { useFileUpload, UploadedFile, FileStatus } from '../hooks/useFileUpload';

jest.mock('../hooks/useFileUpload', () => ({
  useFileUpload: jest.fn(),
}));

const useFileUploadMock = useFileUpload as jest.Mock;

const makeFile = (name: string, size = 1024): File =>
  new File(['rust content '.repeat(64)], name, { type: 'text/plain' });

const makeEntry = (overrides: Partial<UploadedFile> & { id: string }): UploadedFile => ({
  file: makeFile('contract.rs'),
  status: 'pending',
  progress: 0,
  ...overrides,
});

const hookDefault = (overrides: Partial<ReturnType<typeof useFileUpload>> = {}) => ({
  files: [],
  isDragActive: false,
  canAddMore: true,
  allComplete: false,
  maxFiles: 5,
  allowedTypes: ['.rs', '.wasm', '.toml', '.txt'],
  maxSizeMB: 10,
  onDragEnter: jest.fn(),
  onDragOver: jest.fn(),
  onDragLeave: jest.fn(),
  onDrop: jest.fn(),
  onInputChange: jest.fn(),
  removeFile: jest.fn(),
  cancelUpload: jest.fn(),
  retryFile: jest.fn(),
  clearAll: jest.fn(),
  ...overrides,
});

describe('FileUploadZone', () => {
  beforeEach(() => {
    useFileUploadMock.mockReset();
    useFileUploadMock.mockReturnValue(hookDefault());
  });

  it('renders the drop zone with allowed types and limits', () => {
    render(<FileUploadZone />);
    expect(screen.getByRole('button', { name: /file upload area/i })).toBeInTheDocument();
    expect(screen.getByText(/\.rs, \.wasm, \.toml, \.txt/i)).toBeInTheDocument();
    expect(screen.getByText(/max 10MB · up to 5 files/i)).toBeInTheDocument();
  });

  it('renders a disabled drop zone when the file limit is reached', () => {
    useFileUploadMock.mockReturnValue(hookDefault({ canAddMore: false }));
    render(<FileUploadZone />);
    expect(screen.getByText(/Maximum of 5 files reached/i)).toBeInTheDocument();
  });

  it('lists selected files with size and status', () => {
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [
          makeEntry({ id: 'a', status: 'complete', progress: 100 }),
          makeEntry({ id: 'b', status: 'error', error: 'File is empty', file: makeFile('x.rs') }),
        ],
      })
    );
    render(<FileUploadZone />);
    expect(screen.getByText('2 files selected')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
    expect(screen.getByText('File is empty')).toBeInTheDocument();
  });

  it('shows upload speed and ETA while uploading', () => {
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [
          makeEntry({ id: 'a', status: 'uploading', progress: 50, speedBps: 2048, etaSeconds: 8 }),
        ],
      })
    );
    render(<FileUploadZone />);
    expect(screen.getByText(/KB\/s/)).toBeInTheDocument();
    expect(screen.getByText(/s left/)).toBeInTheDocument();
  });

  it('renders the retry button for errored files and triggers retry', () => {
    const retryFile = jest.fn();
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [makeEntry({ id: 'a', status: 'error', error: 'bad' })],
        retryFile,
      })
    );
    render(<FileUploadZone />);
    fireEvent.click(screen.getByRole('button', { name: /retry uploading contract\.rs/i }));
    expect(retryFile).toHaveBeenCalledWith('a');
  });

  it('renders the cancel button while uploading and triggers cancel', () => {
    const cancelUpload = jest.fn();
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [makeEntry({ id: 'a', status: 'uploading', progress: 30 })],
        cancelUpload,
      })
    );
    render(<FileUploadZone />);
    fireEvent.click(screen.getByRole('button', { name: /cancel uploading contract\.rs/i }));
    expect(cancelUpload).toHaveBeenCalledWith('a');
  });

  it('removes a file via the remove button', () => {
    const removeFile = jest.fn();
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [makeEntry({ id: 'a', status: 'complete', progress: 100 })],
        removeFile,
      })
    );
    render(<FileUploadZone />);
    fireEvent.click(screen.getByRole('button', { name: /remove contract\.rs/i }));
    expect(removeFile).toHaveBeenCalledWith('a');
  });

  it('clears all files', () => {
    const clearAll = jest.fn();
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [makeEntry({ id: 'a', status: 'complete', progress: 100 })],
        clearAll,
      })
    );
    render(<FileUploadZone />);
    fireEvent.click(screen.getByRole('button', { name: /clear all/i }));
    expect(clearAll).toHaveBeenCalled();
  });

  it('calls onFilesReady with ready files when all files complete', () => {
    const onFilesReady = jest.fn();
    const readyFile = makeFile('contract.rs');
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [
          {
            id: 'a',
            file: readyFile,
            status: 'complete',
            progress: 100,
          } as UploadedFile,
        ],
        allComplete: true,
      })
    );
    render(<FileUploadZone onFilesReady={onFilesReady} />);
    fireEvent.click(screen.getByRole('button', { name: /use files/i }));
    expect(onFilesReady).toHaveBeenCalledWith([readyFile]);
  });

  it('disables the submit button until all files are complete', () => {
    useFileUploadMock.mockReturnValue(
      hookDefault({
        files: [makeEntry({ id: 'a', status: 'validating', progress: 10 })],
        allComplete: false,
      })
    );
    render(<FileUploadZone onFilesReady={jest.fn()} />);
    expect(screen.getByRole('button', { name: /use files/i })).toBeDisabled();
  });

  it('forwards drag events to the hook handlers', () => {
    const onDragEnter = jest.fn();
    const onDragLeave = jest.fn();
    const onDrop = jest.fn();
    useFileUploadMock.mockReturnValue(hookDefault({ onDragEnter, onDragLeave, onDrop }));
    render(<FileUploadZone />);
    const zone = screen.getByRole('button', { name: /file upload area/i });
    fireEvent.dragEnter(zone);
    expect(onDragEnter).toHaveBeenCalled();
    fireEvent.drop(zone, { dataTransfer: { files: [makeFile('a.rs')] } });
    expect(onDrop).toHaveBeenCalled();
  });

  it('shows the drag-active state', () => {
    useFileUploadMock.mockReturnValue(hookDefault({ isDragActive: true }));
    render(<FileUploadZone />);
    expect(screen.getByText('Drop your files here')).toBeInTheDocument();
  });

  it('opens the file picker on Enter key', () => {
    const inputClick = jest.fn();
    const onInputChange = jest.fn();
    useFileUploadMock.mockReturnValue(hookDefault({ onInputChange }));
    render(<FileUploadZone />);
    const zone = screen.getByRole('button', { name: /file upload area/i });
    // Attach a listener to the hidden input to observe click forwarding
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    input?.addEventListener('click', inputClick);
    fireEvent.keyDown(zone, { key: 'Enter' });
    expect(inputClick).toHaveBeenCalled();
  });
});
