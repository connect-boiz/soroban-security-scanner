import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import Loading from '../app/loading';
import robots from '../app/robots';
import sitemap from '../app/sitemap';
import TimeTravelDebugger from '../components/TimeTravelDebugger';
import BatchOperations from '../components/BatchOperations';

describe('app loading page', () => {
  it('renders the loading skeleton', () => {
    render(<Loading />);
    expect(document.querySelector('.skeleton')).toBeInTheDocument();
  });
});

describe('metadata routes', () => {
  it('generates the robots config with the sitemap URL', () => {
    const result = robots();
    expect(result.rules[0].userAgent).toBe('*');
    expect(result.sitemap).toContain('/sitemap.xml');
  });

  it('generates the sitemap entry', () => {
    const result = sitemap();
    expect(result[0].url).toContain('http');
    expect(result[0].priority).toBe(1);
  });
});

describe('TimeTravelDebugger', () => {
  it('renders the ledger input and updates its value', () => {
    render(<TimeTravelDebugger />);
    const input = screen.getByRole('spinbutton') as HTMLInputElement;
    expect(input.value).toBe('123456');
    fireEvent.change(input, { target: { value: '999' } });
    expect(input.value).toBe('999');
    expect(screen.getByRole('button', { name: /fork network state/i })).toBeInTheDocument();
  });

  it('renders the contract upgrade option', () => {
    render(<TimeTravelDebugger />);
    expect(screen.getByText('Simulate Contract Upgrade')).toBeInTheDocument();
  });
});

describe('BatchOperations', () => {
  it('renders the batch inputs', () => {
    render(<BatchOperations />);
    const batchSize = screen.getByRole('spinbutton') as HTMLInputElement;
    expect(batchSize.value).toBe('10');
    fireEvent.change(batchSize, { target: { value: '25' } });
    expect(batchSize.value).toBe('25');
    expect(screen.getByPlaceholderText('G...')).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/comma-separated escrow IDs/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /execute batch release/i })).toBeInTheDocument();
  });
});
