import React from 'react';
import { render, screen } from '@testing-library/react';
import ProgressBar, { CircularProgress } from '../components/ui/ProgressBar';

describe('ProgressBar', () => {
  it('renders a progress bar with a label', () => {
    render(<ProgressBar value={50} showLabel />);
    expect(screen.getByText('Progress')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
  });

  it('clamps values above the max to 100%', () => {
    render(<ProgressBar value={150} showLabel />);
    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('respects a custom max', () => {
    render(<ProgressBar value={5} max={10} showLabel />);
    expect(screen.getByText('50%')).toBeInTheDocument();
  });

  it('renders with different colors and sizes', () => {
    const { container } = render(
      <ProgressBar value={30} color="green" size="sm" animated={false} />
    );
    expect(container.querySelector('.bg-green-500')).toBeInTheDocument();
    expect(container.querySelector('.h-2')).toBeInTheDocument();
  });

  it('renders without a label by default', () => {
    render(<ProgressBar value={30} />);
    expect(screen.queryByText('Progress')).not.toBeInTheDocument();
  });
});

describe('CircularProgress', () => {
  it('renders the percentage label', () => {
    render(<CircularProgress value={75} />);
    expect(screen.getByText('75%')).toBeInTheDocument();
  });

  it('renders an svg with the correct dimensions', () => {
    const { container } = render(<CircularProgress value={50} size={100} strokeWidth={10} />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('100');
    expect(svg.getAttribute('height')).toBe('100');
  });

  it('hides the label when showLabel is false', () => {
    render(<CircularProgress value={50} showLabel={false} />);
    expect(screen.queryByText('50%')).not.toBeInTheDocument();
  });
});
