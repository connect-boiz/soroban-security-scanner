import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import HelpPanel from '../components/help/HelpPanel';

describe('HelpPanel', () => {
  const mockOnClose = jest.fn();

  it('renders nothing when topic is null', () => {
    const { container } = render(<HelpPanel topic={null} onClose={mockOnClose} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders correctly when a topic is provided', () => {
    render(<HelpPanel topic="scan" onClose={mockOnClose} />);
    expect(screen.getByText(/Scan Submission/i)).toBeInTheDocument();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', () => {
    render(<HelpPanel topic="scan" onClose={mockOnClose} />);
    const closeButton = screen.getByLabelText(/Close help panel/i);
    fireEvent.click(closeButton);
    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when Escape key is pressed', async () => {
    render(<HelpPanel topic="scan" onClose={mockOnClose} />);
    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => {
      expect(mockOnClose).toHaveBeenCalled();
    });
  });
});
