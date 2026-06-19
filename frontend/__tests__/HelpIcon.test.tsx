import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import HelpIcon from '../components/help/HelpIcon';

describe('HelpIcon', () => {
  it('renders the question mark icon', () => {
    render(<HelpIcon content="Help content" label="Contract ID" />);
    const button = screen.getByRole('button', { name: /Help: Contract ID/i });
    expect(button).toBeInTheDocument();
    expect(button.querySelector('svg')).toBeInTheDocument();
  });

  it('shows tooltip on hover', async () => {
    render(<HelpIcon content="Help content" />);
    const button = screen.getByRole('button');

    fireEvent.mouseEnter(button);

    await waitFor(() => {
      expect(screen.getByText('Help content')).toBeInTheDocument();
    });
  });

  it('has correct aria-label', () => {
    render(<HelpIcon content="test" label="Test Field" />);
    expect(screen.getByLabelText('Help: Test Field')).toBeInTheDocument();
  });
});
