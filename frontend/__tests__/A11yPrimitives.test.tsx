import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  SkipLink,
  AccessibleDialog,
  AccessibleFormField,
  StatusBadge,
  IconButton,
  LoadingSpinner,
} from '../components/accessibility/A11yPrimitives';

beforeAll(() => {
  // jsdom has no matchMedia; the a11y hooks query it for reduced-motion
  if (!window.matchMedia) {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: () => {},
        removeEventListener: () => {},
        addListener: () => {},
        removeListener: () => {},
        dispatchEvent: () => false,
      }),
    });
  }
});

describe('SkipLink', () => {
  it('renders a skip link with the target', () => {
    render(<SkipLink targetId="main" label="Skip navigation" />);
    const link = screen.getByRole('link', { name: /skip navigation/i });
    expect(link.getAttribute('href')).toBe('#main');
  });
});

describe('AccessibleDialog', () => {
  it('renders nothing when closed', () => {
    const { container } = render(
      <AccessibleDialog isOpen={false} onClose={jest.fn()} title="Title">
        <p>body</p>
      </AccessibleDialog>
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders the dialog with title and description', () => {
    render(
      <AccessibleDialog isOpen onClose={jest.fn()} title="Scan Results" description="Findings">
        <p>Body content</p>
      </AccessibleDialog>
    );
    expect(screen.getByRole('dialog', { name: 'Scan Results' })).toBeInTheDocument();
    expect(screen.getByText('Body content')).toBeInTheDocument();
    const dialog = screen.getByRole('dialog');
    expect(dialog.getAttribute('aria-describedby')).toBeTruthy();
  });

  it('closes via the close button', () => {
    const onClose = jest.fn();
    render(
      <AccessibleDialog isOpen onClose={onClose} title="Title">
        <p>body</p>
      </AccessibleDialog>
    );
    fireEvent.click(screen.getByRole('button', { name: /close dialog/i }));
    expect(onClose).toHaveBeenCalled();
  });

  it('closes when the backdrop is clicked', () => {
    const onClose = jest.fn();
    const { container } = render(
      <AccessibleDialog isOpen onClose={onClose} title="Title">
        <p>body</p>
      </AccessibleDialog>
    );
    fireEvent.click(container.firstChild as HTMLElement);
    expect(onClose).toHaveBeenCalled();
  });

  it('closes on Escape key', () => {
    const onClose = jest.fn();
    render(
      <AccessibleDialog isOpen onClose={onClose} title="Title">
        <p>body</p>
      </AccessibleDialog>
    );
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });
});

describe('AccessibleFormField', () => {
  it('wires label, hint and input ids together', () => {
    render(
      <AccessibleFormField label="Contract address" hint="Enter a Stellar contract ID" required>
        {({ inputProps }) => <input {...inputProps} type="text" />}
      </AccessibleFormField>
    );
    const input = screen.getByRole('textbox');
    expect(input.getAttribute('aria-required')).toBe('true');
    expect(input.getAttribute('aria-labelledby')).toBeTruthy();
    expect(input.getAttribute('aria-describedby')).toBeTruthy();
    expect(screen.getByText('Contract address')).toBeInTheDocument();
    expect(screen.getByText('Enter a Stellar contract ID')).toBeInTheDocument();
  });

  it('renders the error with role=alert and marks the input invalid', () => {
    render(
      <AccessibleFormField label="Contract address" error="Invalid address">
        {({ inputProps }) => <input {...inputProps} type="text" />}
      </AccessibleFormField>
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Invalid address');
    expect(screen.getByRole('textbox').getAttribute('aria-invalid')).toBe('true');
  });
});

describe('StatusBadge', () => {
  it('renders every severity with its label', () => {
    render(
      <div>
        <StatusBadge severity="critical" />
        <StatusBadge severity="high" />
        <StatusBadge severity="medium" />
        <StatusBadge severity="low" />
        <StatusBadge severity="info" />
        <StatusBadge severity="pass" />
      </div>
    );
    expect(screen.getByText('Critical')).toBeInTheDocument();
    expect(screen.getByText('High')).toBeInTheDocument();
    expect(screen.getByText('Medium')).toBeInTheDocument();
    expect(screen.getByText('Low')).toBeInTheDocument();
    expect(screen.getByText('Info')).toBeInTheDocument();
    expect(screen.getByText('Pass')).toBeInTheDocument();
  });

  it('includes the count in the accessible label and renders it', () => {
    render(<StatusBadge severity="high" count={3} />);
    expect(screen.getByLabelText('High severity, 3 issues')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('uses singular wording for a single issue', () => {
    render(<StatusBadge severity="low" count={1} />);
    expect(screen.getByLabelText('Low severity, 1 issue')).toBeInTheDocument();
  });
});

describe('IconButton', () => {
  it('renders with the enforced aria-label', () => {
    render(<IconButton aria-label="Close panel" icon={<span>x</span>} />);
    expect(screen.getByRole('button', { name: /close panel/i })).toBeInTheDocument();
  });

  it('supports outline and filled variants and disabled state', () => {
    const { container, rerender } = render(
      <IconButton aria-label="Edit" icon={<span>✎</span>} variant="outline" disabled />
    );
    expect(container.querySelector('.border-gray-600')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /edit/i })).toBeDisabled();
    rerender(<IconButton aria-label="Save" icon={<span>✓</span>} variant="filled" size="lg" />);
    expect(container.querySelector('.bg-cyan-500')).toBeInTheDocument();
    expect(container.querySelector('.h-11')).toBeInTheDocument();
  });
});

describe('LoadingSpinner', () => {
  it('announces loading via role=status', () => {
    render(<LoadingSpinner label="Scanning contract…" />);
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.getByLabelText('Scanning contract…')).toBeInTheDocument();
  });
});
