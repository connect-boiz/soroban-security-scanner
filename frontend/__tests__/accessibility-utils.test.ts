import {
  getContrastRatio,
  checkContrastLevel,
  A11Y_COLORS,
  focusFirstIn,
  restoreFocus,
  getFocusableElements,
  trapFocus,
  generateAriaId,
  toggleAriaLabel,
  liveRegion,
  prefersReducedMotion,
  skipToContent,
} from '../lib/accessibility/utils';

describe('color contrast', () => {
  it('computes contrast ratios (WCAG formula)', () => {
    expect(getContrastRatio('#000000', '#ffffff')).toBeGreaterThan(15);
    expect(getContrastRatio('#ffffff', '#ffffff')).toBe(1);
  });
  it('classifies contrast levels', () => {
    expect(checkContrastLevel('#000000', '#ffffff')).toBe('AAA');
    expect(checkContrastLevel('#767676', '#ffffff')).toBe('AA');
    // #808080 on white is ~3.9:1 — AA only for large text.
    expect(checkContrastLevel('#808080', '#ffffff')).toBe('fail');
    expect(checkContrastLevel('#808080', '#ffffff', true)).toBe('AA-large');
    expect(checkContrastLevel('#ffffff', '#ffffff')).toBe('fail');
  });
  it('exposes AA-verified brand tokens', () => {
    expect(A11Y_COLORS.primaryText).toBe('#22d3ee');
    expect(A11Y_COLORS.bodyText).toBe('#e2e8f0');
  });
});

describe('focus management', () => {
  it('focuses the first focusable descendant, else the container', () => {
    const container = document.createElement('div');
    container.setAttribute('tabindex', '-1');
    document.body.appendChild(container);
    focusFirstIn(container);
    expect(document.activeElement).toBe(container);

    const button = document.createElement('button');
    container.appendChild(button);
    focusFirstIn(container);
    expect(document.activeElement).toBe(button);
    document.body.removeChild(container);
  });

  it('restores focus when the element is still in the document', () => {
    const el = document.createElement('button');
    document.body.appendChild(el);
    restoreFocus(el);
    expect(document.activeElement).toBe(el);
    document.body.removeChild(el);
    // Gracefully no-ops for detached elements
    restoreFocus(el);
  });

  it('collects only visible focusable elements', () => {
    const container = document.createElement('div');
    const visible = document.createElement('button');
    const hidden = document.createElement('button');
    hidden.setAttribute('hidden', '');
    const ariaHidden = document.createElement('a');
    ariaHidden.setAttribute('aria-hidden', 'true');
    const disabled = document.createElement('button');
    disabled.setAttribute('disabled', '');
    container.append(visible, hidden, ariaHidden, disabled);
    expect(getFocusableElements(container)).toEqual([visible]);
  });
});

describe('trapFocus', () => {
  it('traps Tab and Shift+Tab within the container', () => {
    const container = document.createElement('div');
    const first = document.createElement('button');
    const last = document.createElement('button');
    container.append(first, last);
    document.body.appendChild(container);

    const cleanup = trapFocus(container);

    last.focus();
    last.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    expect(document.activeElement).toBe(first);

    first.focus();
    first.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true })
    );
    expect(document.activeElement).toBe(last);

    // Non-Tab keys are ignored.
    first.focus();
    first.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    expect(document.activeElement).toBe(first);

    cleanup();
    document.body.removeChild(container);
  });

  it('no-ops when there are no focusable elements', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const cleanup = trapFocus(container);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    cleanup();
    document.body.removeChild(container);
  });
});

describe('ARIA helpers', () => {
  it('generates unique aria ids', () => {
    expect(generateAriaId('x')).toBe('x-1');
    expect(generateAriaId('x')).toBe('x-2');
    expect(generateAriaId()).toMatch(/^aria-\d+$/);
  });
  it('builds toggle labels', () => {
    expect(toggleAriaLabel('Menu', true)).toContain('active');
    expect(toggleAriaLabel('Menu', false)).toContain('press to activate');
  });
});

describe('live region', () => {
  it('announces and clears polite and assertive regions', () => {
    liveRegion.announce('hello');
    liveRegion.announce('urgent', 'assertive');
    expect(document.body.querySelector('[aria-live="polite"]')).not.toBeNull();
    expect(document.body.querySelector('[role="alert"]')).not.toBeNull();

    liveRegion.clear('polite');
    const polite = document.body.querySelector('[aria-live="polite"]');
    expect(polite?.textContent).toBe('');

    liveRegion.announce('again');
    liveRegion.clear();
    const regions = document.body.querySelectorAll('[aria-live]');
    regions.forEach(r => expect(r.textContent).toBe(''));
  });
});

describe('motion & skip links', () => {
  it('detects reduced-motion preference', () => {
    window.matchMedia = jest
      .fn()
      .mockReturnValue({ matches: true }) as unknown as typeof window.matchMedia;
    expect(prefersReducedMotion()).toBe(true);
    window.matchMedia = jest
      .fn()
      .mockReturnValue({ matches: false }) as unknown as typeof window.matchMedia;
    expect(prefersReducedMotion()).toBe(false);
  });

  it('focuses a skip-link target and cleans up the temporary tabindex', () => {
    const target = document.createElement('main');
    target.id = 'main-content';
    document.body.appendChild(target);
    skipToContent('main-content');
    expect(document.activeElement).toBe(target);
    expect(target.hasAttribute('tabindex')).toBe(true);

    // No-op for missing targets.
    skipToContent('missing-target');
    document.body.removeChild(target);
  });
});
