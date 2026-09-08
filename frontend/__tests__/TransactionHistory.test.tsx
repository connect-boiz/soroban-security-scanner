import React from 'react';
import { render, screen, fireEvent, within } from '@testing-library/react';
import TransactionHistory from '../components/TransactionHistory';

const storageKey = 'transactionHistory.savedSearches.v1';

describe('TransactionHistory', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('renders the first page of transactions', () => {
    render(<TransactionHistory />);
    expect(screen.getByText('scan-001')).toBeInTheDocument();
    expect(screen.getByText('Showing 5 of 10 results')).toBeInTheDocument();
    expect(screen.getByText('Page 1 of 2')).toBeInTheDocument();
  });

  it('filters by search term', () => {
    render(<TransactionHistory />);
    fireEvent.change(screen.getByPlaceholderText('Search by contract or ID...'), {
      target: { value: 'scan-007' },
    });
    expect(screen.getByText('scan-007')).toBeInTheDocument();
    expect(screen.queryByText('scan-001')).not.toBeInTheDocument();
    expect(screen.getByText('Showing 1 of 1 results')).toBeInTheDocument();
  });

  it('shows the empty state when nothing matches', () => {
    render(<TransactionHistory />);
    fireEvent.change(screen.getByPlaceholderText('Search by contract or ID...'), {
      target: { value: 'zzz-nothing' },
    });
    expect(
      screen.getByText('No transactions match the current search and facet filters.')
    ).toBeInTheDocument();
  });

  it('filters by severity facet', () => {
    render(<TransactionHistory />);
    const severitySection = screen.getByText('Severity facets').closest('div')!;
    fireEvent.click(within(severitySection).getByRole('button', { name: 'Critical' }));
    expect(screen.getByText('scan-002')).toBeInTheDocument();
    expect(screen.getByText('scan-008')).toBeInTheDocument();
    expect(screen.queryByText('scan-001')).not.toBeInTheDocument();
    // Toggle off restores everything
    fireEvent.click(within(severitySection).getByRole('button', { name: 'Critical' }));
    expect(screen.getByText('scan-001')).toBeInTheDocument();
  });

  it('filters by status facet', () => {
    render(<TransactionHistory />);
    const statusSection = screen.getByText('Status facets').closest('div')!;
    fireEvent.click(within(statusSection).getByRole('button', { name: 'Failed' }));
    expect(screen.getByText('scan-006')).toBeInTheDocument();
    expect(screen.queryByText('scan-001')).not.toBeInTheDocument();
  });

  it('filters by issue scope', () => {
    render(<TransactionHistory />);
    const select = screen.getAllByRole('combobox')[1];
    fireEvent.change(select, { target: { value: 'clean' } });
    expect(screen.queryByText('scan-001')).not.toBeInTheDocument();
    expect(screen.getByText('scan-003')).toBeInTheDocument();
  });

  it('sorts by column, toggling direction', () => {
    const { container } = render(<TransactionHistory />);
    const firstCell = () =>
      container.querySelectorAll('tbody tr')[0]?.querySelector('td')?.textContent;
    const idHeader = screen.getByRole('columnheader', { name: 'ID' });
    fireEvent.click(idHeader);
    expect(firstCell()).toBe('scan-001');
    // Clicking again reverses the order
    fireEvent.click(idHeader);
    expect(firstCell()).toBe('scan-010');
  });

  it('paginates through results', () => {
    render(<TransactionHistory />);
    expect(screen.getByText('scan-001')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(screen.getByText('scan-006')).toBeInTheDocument();
    expect(screen.getByText('Page 2 of 2')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /previous/i }));
    expect(screen.getByText('scan-001')).toBeInTheDocument();
  });

  it('opens and closes the details modal', () => {
    render(<TransactionHistory />);
    fireEvent.click(screen.getAllByRole('button', { name: /details/i })[0]);
    expect(screen.getByText('Scan Details: scan-001')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(screen.queryByText('Scan Details: scan-001')).not.toBeInTheDocument();
  });

  it('exports the filtered data to CSV', () => {
    const createObjectURL = jest.fn(() => 'blob:mock');
    const revokeObjectURL = jest.fn();
    window.URL.createObjectURL = createObjectURL;
    window.URL.revokeObjectURL = revokeObjectURL;
    const clickSpy = jest.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    render(<TransactionHistory />);
    fireEvent.click(screen.getByRole('button', { name: /export csv/i }));
    expect(createObjectURL).toHaveBeenCalled();
    expect(clickSpy).toHaveBeenCalled();

    clickSpy.mockRestore();
  });

  it('saves, applies, and removes saved searches', () => {
    render(<TransactionHistory />);
    // Save a filtered search
    fireEvent.change(screen.getByPlaceholderText('Search by contract or ID...'), {
      target: { value: 'scan-005' },
    });
    fireEvent.change(screen.getByPlaceholderText('Save current search as...'), {
      target: { value: 'My Search' },
    });
    fireEvent.click(screen.getByRole('button', { name: /save search/i }));
    expect(screen.getByText('My Search')).toBeInTheDocument();
    expect(localStorage.getItem(storageKey)).toContain('My Search');

    // Reset filters then re-apply the saved search
    fireEvent.click(screen.getByRole('button', { name: /reset filters/i }));
    expect(screen.getByText('Showing 5 of 10 results')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'My Search' }));
    expect(screen.getByText('Showing 1 of 1 results')).toBeInTheDocument();

    // Remove the saved search
    fireEvent.click(screen.getByRole('button', { name: /remove saved search my search/i }));
    expect(screen.queryByText('My Search')).not.toBeInTheDocument();
  });

  it('does not save searches with an empty name', () => {
    render(<TransactionHistory />);
    fireEvent.click(screen.getByRole('button', { name: /save search/i }));
    expect(screen.getByText(/No saved searches yet/i)).toBeInTheDocument();
  });

  it('restores saved searches from localStorage on mount', () => {
    localStorage.setItem(
      storageKey,
      JSON.stringify([
        {
          id: 's1',
          name: 'Stored',
          filters: { query: '', severities: [], statuses: [], issueScope: 'all' },
          createdAt: '',
        },
      ])
    );
    render(<TransactionHistory />);
    expect(screen.getByText('Stored')).toBeInTheDocument();
  });

  it('tolerates corrupt localStorage', () => {
    localStorage.setItem(storageKey, '{not json');
    render(<TransactionHistory />);
    expect(screen.getByText('scan-001')).toBeInTheDocument();
  });
});
