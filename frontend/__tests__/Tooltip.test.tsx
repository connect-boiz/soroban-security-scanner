import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import Tooltip from '../components/help/Tooltip';

describe('Tooltip', () => {
  it('renders children correctly', () => {
    render(
      <Tooltip content="Helpful text">
        <span>Hover me</span>
      </Tooltip>
    );
    expect(screen.getByText('Hover me')).toBeInTheDocument();
  });

  it('shows content when hovered', async () => {
    render(
      <Tooltip content="Helpful text">
        <span>Hover me</span>
      </Tooltip>
    );

    const trigger = screen.getByText('Hover me');
    fireEvent.mouseEnter(trigger);

    await waitFor(() => {
      expect(screen.getByText('Helpful text')).toBeInTheDocument();
    });
  });

  it('hides content when mouse leaves', async () => {
    render(
      <Tooltip content="Helpful text">
        <span>Hover me</span>
      </Tooltip>
    );

    const trigger = screen.getByText('Hover me');
    fireEvent.mouseEnter(trigger);
    
    await waitFor(() => {
      expect(screen.getByText('Helpful text')).toBeInTheDocument();
    });

    fireEvent.mouseLeave(trigger);

    await waitFor(() => {
      expect(screen.queryByText('Helpful text')).not.toBeInTheDocument();
    });
  });

  it('shows content on focus and hides on blur', async () => {
    render(
      <Tooltip content="Helpful text">
        <button>Focus me</button>
      </Tooltip>
    );

    const trigger = screen.getByText('Focus me');
    fireEvent.focus(trigger);

    await waitFor(() => {
      expect(screen.getByText('Helpful text')).toBeInTheDocument();
    });

    fireEvent.blur(trigger);

    await waitFor(() => {
      expect(screen.queryByText('Helpful text')).not.toBeInTheDocument();
    });
  });

  it('has correct accessibility attributes', async () => {
    render(
      <Tooltip content="Helpful text">
        <span>Hover me</span>
      </Tooltip>
    );
    
    const trigger = screen.getByText('Hover me');
    fireEvent.mouseEnter(trigger);
    
    const tooltip = await screen.findByRole('tooltip');
    expect(tooltip).toHaveAttribute('id');
    expect(trigger.parentElement).toHaveAttribute('aria-describedby', tooltip.id);
  });
});
