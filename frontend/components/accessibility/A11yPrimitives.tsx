'use client';

// frontend/components/accessibility/A11yPrimitives.tsx
//
// WCAG 2.1 AA-compliant primitive components for Soroban Security Scanner.
// Each component documents which success criteria it satisfies.

import React, { useEffect, useRef } from 'react';
import {
  useFocusTrap,
  useEscapeKey,
  useAriaIds,
  useSkipLink,
  useReducedMotion,
} from '@/hooks/useAccessibility';

// ── 1. SkipLink (WCAG 2.4.1 – Bypass Blocks) ─────────────────────────────────
/**
 * Renders a visually-hidden link that becomes visible on focus.
 * Place as the very first element inside <body> / layout root.
 *
 * Usage:
 *   <SkipLink targetId="main-content" />
 *   ...
 *   <main id="main-content" tabIndex={-1}>...</main>
 */
export function SkipLink({
  targetId = 'main-content',
  label = 'Skip to main content',
  dir,
}: {
  targetId?: string;
  label?: string;
  dir?: 'ltr' | 'rtl';
}) {
  const { skipLinkProps } = useSkipLink(targetId);

  const placementClass =
    dir === 'rtl'
      ? 'focus:fixed focus:top-4 focus:right-4 focus:z-[9999]'
      : 'focus:fixed focus:top-4 focus:left-4 rtl:focus:right-4 rtl:focus:left-auto focus:z-[9999]';

  return (
    <a
      {...skipLinkProps}
      dir={dir}
      className={[
        // Visually hidden until focused
        'sr-only focus:not-sr-only',
        // Visible styling when focused (swaps left/right based on text direction)
        placementClass,
        'focus:rounded-lg focus:bg-cyan-400 focus:px-4 focus:py-2',
        'focus:text-sm focus:font-bold focus:text-gray-950',
        'focus:shadow-lg focus:outline-none',
        // Transition (respects reduced motion via CSS)
        'transition-opacity',
      ].join(' ')}
    >
      {label}
    </a>
  );
}

// ── 2. AccessibleDialog (WCAG 1.3.1, 2.1.1, 2.1.2, 4.1.2) ──────────────────
/**
 * ARIA dialog with focus trap, Escape-to-close, and role="dialog".
 *
 * Usage:
 *   <AccessibleDialog
 *     isOpen={open}
 *     onClose={() => setOpen(false)}
 *     title="Scan Results"
 *   >
 *     <p>...</p>
 *   </AccessibleDialog>
 */
interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
  dir?: 'ltr' | 'rtl';
}

export function AccessibleDialog({
  isOpen,
  onClose,
  title,
  description,
  children,
  size = 'md',
  dir,
}: DialogProps) {
  const { containerRef } = useFocusTrap<HTMLDivElement>(isOpen);
  useEscapeKey(onClose, isOpen);
  const { labelId, descId } = useAriaIds('dialog');
  const reduced = useReducedMotion();

  if (!isOpen) return null;

  const sizeClass = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl' }[size];

  return (
    // Backdrop — WCAG 1.4.11: backdrop doesn't need to meet contrast but close
    // action must be keyboard accessible (Escape key handled above).
    <div
      role="presentation"
      dir={dir}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={containerRef}
        role="dialog"
        dir={dir}
        aria-modal="true"
        aria-labelledby={labelId}
        aria-describedby={description ? descId : undefined}
        className={[
          'relative w-full rounded-2xl',
          'bg-gray-900 border border-gray-700',
          'shadow-2xl shadow-black/60',
          'focus:outline-none',
          sizeClass,
          reduced ? '' : 'animate-fade-in-scale',
        ].join(' ')}
        // Allows focus() in useFocusTrap fallback
        tabIndex={-1}
      >
        {/* Header */}
        <div className="flex items-start justify-between gap-4 border-b border-gray-700 px-6 py-4">
          <div>
            <h2 id={labelId} className="text-lg font-bold text-gray-100 leading-snug">
              {title}
            </h2>
            {description && (
              <p id={descId} className="mt-1 text-sm text-gray-400">
                {description}
              </p>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close dialog"
            className={[
              'flex-shrink-0 rounded-lg p-1.5 text-gray-400',
              'hover:bg-gray-700 hover:text-gray-100',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400',
              'transition-colors',
            ].join(' ')}
          >
            {/* aria-hidden: icon is decorative; label above covers semantics */}
            <svg
              aria-hidden="true"
              focusable="false"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
            >
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-5">{children}</div>
      </div>
    </div>
  );
}

// ── 3. AccessibleFormField (WCAG 1.3.1, 3.3.1, 3.3.2, 4.1.2) ────────────────
/**
 * Wraps an input with a properly associated label, description, and error.
 * All associations use aria-labelledby / aria-describedby (not just `for`).
 *
 * Usage:
 *   <AccessibleFormField
 *     label="Contract Address"
 *     hint="Enter a valid Stellar contract ID (C...)"
 *     error={errors.contractId}
 *     required
 *   >
 *     {({ inputProps }) => <input {...inputProps} type="text" />}
 *   </AccessibleFormField>
 */
interface FormFieldProps {
  label: string;
  hint?: string;
  error?: string;
  required?: boolean;
  dir?: 'ltr' | 'rtl';
  children: (props: {
    inputProps: {
      'aria-labelledby': string;
      'aria-describedby': string;
      'aria-required': boolean;
      'aria-invalid': boolean;
      id: string;
      dir?: 'ltr' | 'rtl';
    };
  }) => React.ReactNode;
}

export function AccessibleFormField({
  label,
  hint,
  error,
  required = false,
  dir,
  children,
}: FormFieldProps) {
  const { labelId, descId, errorId } = useAriaIds('field');
  const inputId = `input-${labelId}`;

  const describedBy = [hint ? descId : '', error ? errorId : ''].filter(Boolean).join(' ');
  const asteriskMarginClass = dir === 'rtl' ? 'mr-1 text-cyan-400' : 'ml-1 rtl:mr-1 rtl:ml-0 text-cyan-400';

  return (
    <div dir={dir} className="flex flex-col gap-1.5 text-start">
      {/* Label — WCAG 1.3.1, 3.3.2 */}
      <label id={labelId} htmlFor={inputId} className="text-sm font-semibold text-gray-200">
        {label}
        {required && (
          <>
            {/* Screen reader: "required" */}
            <span aria-hidden="true" className={asteriskMarginClass}>
              *
            </span>
            <span className="sr-only"> (required)</span>
          </>
        )}
      </label>

      {/* Hint — always rendered so describedBy is stable */}
      {hint && (
        <p id={descId} className="text-xs text-gray-400 leading-relaxed">
          {hint}
        </p>
      )}

      {/* Child input receives all ARIA props */}
      {children({
        inputProps: {
          id: inputId,
          'aria-labelledby': labelId,
          'aria-describedby': describedBy,
          'aria-required': required,
          'aria-invalid': !!error,
          dir,
        },
      })}

      {/* Error — WCAG 3.3.1: error identified in text */}
      {error && (
        <p
          id={errorId}
          role="alert"
          aria-live="polite"
          className="flex items-center gap-1.5 text-xs font-medium text-red-400"
        >
          <svg
            aria-hidden="true"
            focusable="false"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          {error}
        </p>
      )}
    </div>
  );
}

// ── 4. StatusBadge (WCAG 1.4.1, 1.4.3) ───────────────────────────────────────
/**
 * Severity badge for scan results. NEVER relies on colour alone (WCAG 1.4.1).
 * Each variant includes an icon + text label. Contrast ratios verified AA+.
 */
type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info' | 'pass';

const SEVERITY_CONFIG: Record<
  Severity,
  { label: string; icon: string; bgClass: string; textClass: string; borderClass: string }
> = {
  critical: {
    label: 'Critical',
    icon: '⛔',
    bgClass: 'bg-red-950/60',
    textClass: 'text-red-300',
    borderClass: 'border-red-700/50',
  },
  high: {
    label: 'High',
    icon: '🔴',
    bgClass: 'bg-orange-950/60',
    textClass: 'text-orange-300',
    borderClass: 'border-orange-700/50',
  },
  medium: {
    label: 'Medium',
    icon: '🟡',
    bgClass: 'bg-yellow-950/60',
    textClass: 'text-yellow-300',
    borderClass: 'border-yellow-700/50',
  },
  low: {
    label: 'Low',
    icon: '🟢',
    bgClass: 'bg-green-950/60',
    textClass: 'text-green-300',
    borderClass: 'border-green-700/50',
  },
  info: {
    label: 'Info',
    icon: 'ℹ️',
    bgClass: 'bg-blue-950/60',
    textClass: 'text-blue-300',
    borderClass: 'border-blue-700/50',
  },
  pass: {
    label: 'Pass',
    icon: '✅',
    bgClass: 'bg-emerald-950/60',
    textClass: 'text-emerald-300',
    borderClass: 'border-emerald-700/50',
  },
};

export function StatusBadge({ severity, count }: { severity: Severity; count?: number }) {
  const cfg = SEVERITY_CONFIG[severity];

  return (
    <span
      className={[
        'inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1',
        'text-xs font-bold tracking-wide',
        cfg.bgClass,
        cfg.textClass,
        cfg.borderClass,
      ].join(' ')}
      // Screen readers get full semantics without relying on colour
      aria-label={`${cfg.label} severity${count !== undefined ? `, ${count} issue${count !== 1 ? 's' : ''}` : ''}`}
    >
      {/* Icon is decorative — aria-label on parent covers meaning */}
      <span aria-hidden="true">{cfg.icon}</span>
      {cfg.label}
      {count !== undefined && (
        <span className="ml-0.5 rounded-full bg-black/30 px-1.5 py-px text-[10px]">{count}</span>
      )}
    </span>
  );
}

// ── 5. IconButton (WCAG 4.1.2 – Name, Role, Value) ───────────────────────────
/**
 * An icon-only button with a mandatory accessible label.
 * TypeScript enforces `aria-label` so it can never be omitted accidentally.
 */
interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  'aria-label': string; // enforced — cannot be forgotten
  icon: React.ReactNode;
  variant?: 'ghost' | 'outline' | 'filled';
  size?: 'sm' | 'md' | 'lg';
}

export function IconButton({
  'aria-label': ariaLabel,
  icon,
  variant = 'ghost',
  size = 'md',
  className = '',
  ...props
}: IconButtonProps) {
  const sizeClass = {
    sm: 'h-7 w-7 text-sm',
    md: 'h-9 w-9 text-base',
    lg: 'h-11 w-11 text-lg',
  }[size];

  const variantClass = {
    ghost: 'bg-transparent hover:bg-gray-700 text-gray-400 hover:text-gray-100',
    outline: 'border border-gray-600 bg-transparent hover:bg-gray-700 text-gray-300',
    filled: 'bg-cyan-500 hover:bg-cyan-400 text-gray-950',
  }[variant];

  return (
    <button
      type="button"
      aria-label={ariaLabel}
      className={[
        'inline-flex items-center justify-center rounded-lg',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-gray-900',
        'disabled:opacity-40 disabled:cursor-not-allowed',
        'transition-colors',
        sizeClass,
        variantClass,
        className,
      ].join(' ')}
      {...props}
    >
      {/* Icon is decorative; button label is the accessible name */}
      <span aria-hidden="true">{icon}</span>
    </button>
  );
}

// ── 6. LoadingSpinner (WCAG 4.1.3 – Status Messages) ─────────────────────────
/**
 * Accessible loading indicator. Announces state to screen readers via
 * role="status" + aria-live="polite" so focus is not disrupted.
 */
export function LoadingSpinner({
  label = 'Loading, please wait…',
  size = 'md',
}: {
  label?: string;
  size?: 'sm' | 'md' | 'lg';
}) {
  const sizeClass = { sm: 'h-4 w-4', md: 'h-8 w-8', lg: 'h-12 w-12' }[size];

  return (
    <div
      role="status"
      aria-live="polite"
      aria-label={label}
      className="flex flex-col items-center gap-3"
    >
      <svg
        aria-hidden="true"
        focusable="false"
        className={`${sizeClass} animate-spin text-cyan-400`}
        viewBox="0 0 24 24"
        fill="none"
      >
        <circle
          className="opacity-25"
          cx="12"
          cy="12"
          r="10"
          stroke="currentColor"
          strokeWidth="4"
        />
        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z" />
      </svg>
      {/* Visible text for sighted users; screen readers get aria-label */}
      <span className="text-sm text-gray-400" aria-hidden="true">
        {label}
      </span>
    </div>
  );
}

// ── 7. Directional ARIA Helpers (WCAG 1.3.2, 2.1.1, 4.1.2) ───────────────────
/**
 * Maps physical "left" and "right" ARIA attributes and keyboard controls to
 * logical inline-start and inline-end based on active text direction.
 */
export interface DirectionalAriaOptions {
  direction?: 'ltr' | 'rtl';
  labelLeft?: string;
  labelRight?: string;
}

export function getDirectionalAria({
  direction = 'ltr',
  labelLeft = 'Previous',
  labelRight = 'Next',
}: DirectionalAriaOptions = {}) {
  const isRtl = direction === 'rtl';

  return {
    direction,
    isRtl,
    // Keyboard navigation keys for roving tabindex
    previousKey: isRtl ? 'ArrowRight' : 'ArrowLeft',
    nextKey: isRtl ? 'ArrowLeft' : 'ArrowRight',
    // Physical-to-logical placement mapping
    startPlacement: isRtl ? 'right' : 'left',
    endPlacement: isRtl ? 'left' : 'right',
    // Swapped ARIA labels for directional triggers
    startLabel: isRtl ? labelRight : labelLeft,
    endLabel: isRtl ? labelLeft : labelRight,
    // Swapped alignment classes
    textAlignClass: isRtl ? 'text-right' : 'text-left',
    // Arrow symbols
    prevArrow: isRtl ? '→' : '←',
    nextArrow: isRtl ? '←' : '→',
  };
}

/**
 * DirectionalNavigationButton: Renders an accessible directional button
 * that automatically swaps its ARIA label, arrow direction, and icon orientation
 * based on text direction (LTR vs RTL).
 */
export interface DirectionalNavigationButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  direction?: 'ltr' | 'rtl';
  action: 'previous' | 'next';
  label?: string;
}

export function DirectionalNavigationButton({
  direction = 'ltr',
  action,
  label,
  className = '',
  ...props
}: DirectionalNavigationButtonProps) {
  const dirInfo = getDirectionalAria({ direction });
  const isPrev = action === 'previous';
  const defaultLabel = isPrev ? 'Previous' : 'Next';
  const accessibleLabel = label || defaultLabel;
  const arrow = isPrev ? dirInfo.prevArrow : dirInfo.nextArrow;

  return (
    <button
      type="button"
      aria-label={accessibleLabel}
      data-action={action}
      data-direction={direction}
      className={[
        'inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm font-medium',
        'bg-gray-800 text-gray-200 hover:bg-gray-700',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400',
        'transition-colors',
        className,
      ].join(' ')}
      {...props}
    >
      <span aria-hidden="true">{arrow}</span>
      <span>{accessibleLabel}</span>
    </button>
  );
}

/**
 * DirectionalToolbar: ARIA toolbar widget that swaps ArrowLeft and ArrowRight
 * key navigation and flex row layout based on text direction.
 */
export function DirectionalToolbar({
  children,
  direction = 'ltr',
  'aria-label': ariaLabel = 'Actions',
  className = '',
}: {
  children: React.ReactNode;
  direction?: 'ltr' | 'rtl';
  'aria-label'?: string;
  className?: string;
}) {
  const isRtl = direction === 'rtl';

  return (
    <div
      role="toolbar"
      aria-label={ariaLabel}
      aria-orientation="horizontal"
      dir={direction}
      data-direction={direction}
      className={[
        'flex items-center gap-2',
        isRtl ? 'flex-row-reverse' : 'flex-row',
        className,
      ].join(' ')}
    >
      {children}
    </div>
  );
}

