import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import ScannerInterface from '../components/ScannerInterface';

describe('ScannerInterface', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders the paste mode by default with a disabled scan button', () => {
    render(<ScannerInterface />);
    expect(screen.getByText('Contract Scanner')).toBeInTheDocument();
    expect(screen.getByText('📋 Paste Code')).toBeInTheDocument();
    expect(screen.getByText('📁 Upload Files')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /scan contract/i })).toBeDisabled();
  });

  it('switches to upload mode', () => {
    render(<ScannerInterface />);
    fireEvent.click(screen.getByRole('button', { name: /upload files/i }));
    expect(screen.getByText('Upload Files')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /file upload area/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /scan contract/i })).toBeDisabled();
  });

  it('runs a scan and shows the results', async () => {
    render(<ScannerInterface />);
    fireEvent.change(screen.getByPlaceholderText(/paste your soroban contract code/i), {
      target: { value: 'pub fn scan() {}' },
    });
    fireEvent.click(screen.getByRole('button', { name: /scan contract/i }));

    // Advance through the 5 scan stages (timers chain sequentially)
    for (let i = 0; i < 8; i++) {
      await act(async () => {
        jest.advanceTimersByTime(500);
      });
    }

    expect(screen.getByText('Scan Results')).toBeInTheDocument();
    expect(screen.getByText('HIGH')).toBeInTheDocument();
    expect(screen.getByText('0x1234...5678')).toBeInTheDocument();
    expect(screen.getByText('Potential reentrancy vulnerability detected')).toBeInTheDocument();
    expect(screen.getByText('Missing input validation in function transfer')).toBeInTheDocument();
    expect(screen.getByText('Unprotected external call')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /scan contract/i })).toBeEnabled();
  });

  it('shows scan progress stages while scanning', async () => {
    render(<ScannerInterface />);
    fireEvent.change(screen.getByPlaceholderText(/paste your soroban contract code/i), {
      target: { value: 'pub fn scan() {}' },
    });
    fireEvent.click(screen.getByRole('button', { name: /scan contract/i }));

    await act(async () => {
      jest.advanceTimersByTime(900);
    });
    // After the first two stages progress is past 0
    expect(screen.getByText('Scan Progress')).toBeInTheDocument();
    expect(screen.getByText(/Analyzing bytecode/)).toBeInTheDocument();

    await act(async () => {
      jest.advanceTimersByTime(1000);
    });
    // Skeleton results appear past 70%
    expect(screen.getByText(/Checking for vulnerabilities/)).toBeInTheDocument();

    // Finish the scan
    for (let i = 0; i < 6; i++) {
      await act(async () => {
        jest.advanceTimersByTime(400);
      });
    }
    expect(screen.getByText('Scan Results')).toBeInTheDocument();
  });

  it('cannot start a scan while one is running', async () => {
    render(<ScannerInterface />);
    fireEvent.change(screen.getByPlaceholderText(/paste your soroban contract code/i), {
      target: { value: 'pub fn scan() {}' },
    });
    fireEvent.click(screen.getByRole('button', { name: /scan contract/i }));
    expect(screen.getByRole('button', { name: /scanning…/i })).toBeDisabled();
    expect(screen.getByPlaceholderText(/paste your soroban contract code/i)).toBeDisabled();
    // Let the scan finish so no timers leak
    for (let i = 0; i < 8; i++) {
      await act(async () => {
        jest.advanceTimersByTime(500);
      });
    }
  });
});
