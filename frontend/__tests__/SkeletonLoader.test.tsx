import React from 'react';
import { render } from '@testing-library/react';
import SkeletonLoader from '../components/ui/SkeletonLoader';

describe('SkeletonLoader', () => {
  it('renders the card skeleton with avatar and button', () => {
    const { container } = render(<SkeletonLoader type="card" lines={3} avatar button />);
    expect(container.querySelector('.rounded-full')).toBeInTheDocument();
    expect(container.querySelectorAll('.animate-pulse').length).toBeGreaterThan(3);
  });

  it('renders the table skeleton', () => {
    const { container } = render(<SkeletonLoader type="table" lines={2} />);
    expect(container.querySelector('.grid-cols-4')).toBeInTheDocument();
    expect(container.querySelectorAll('.divide-y > div').length).toBe(2);
  });

  it('renders the list skeleton with avatars', () => {
    const { container } = render(<SkeletonLoader type="list" lines={2} avatar />);
    expect(container.querySelectorAll('.rounded-full').length).toBe(2);
  });

  it('renders the chart skeleton', () => {
    const { container } = render(<SkeletonLoader type="chart" />);
    expect(container.querySelectorAll('.w-12').length).toBe(5);
  });

  it('renders the form skeleton', () => {
    const { container } = render(<SkeletonLoader type="form" lines={2} />);
    expect(container.querySelectorAll('.h-10').length).toBeGreaterThan(1);
  });

  it('renders the modal skeleton', () => {
    const { container } = render(<SkeletonLoader type="modal" lines={2} />);
    expect(container.querySelector('.max-w-md')).toBeInTheDocument();
  });

  it('omits the pulse animation when animated is false', () => {
    const { container } = render(<SkeletonLoader type="card" animated={false} />);
    expect(container.querySelectorAll('.animate-pulse').length).toBe(0);
  });

  it('renders with a custom className', () => {
    const { container } = render(
      <SkeletonLoader type="card" animated={false} className="my-custom-class" />
    );
    expect(container.querySelector('.my-custom-class')).toBeInTheDocument();
  });
});
