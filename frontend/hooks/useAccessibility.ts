// frontend/hooks/useAccessibility.ts
//
// Collection of WCAG 2.1 AA React hooks for the Soroban Security Scanner.

import {
  useEffect,
  useRef,
  useCallback,
  useState,
  useId,
  RefObject,
} from "react";
import {
  trapFocus,
  focusFirstIn,
  restoreFocus,
  liveRegion,
  prefersReducedMotion,
  getFocusableElements,
} from "@/lib/accessibility/utils";

// ── useFocusTrap ──────────────────────────────────────────────────────────────
/**
 * Trap keyboard focus inside a container while `active` is true.
 * Restores focus to the previously focused element on deactivation.
 *
 * @example
 * const { containerRef } = useFocusTrap(isModalOpen);
 * <div ref={containerRef} role="dialog" aria-modal="true">...</div>
 */
export function useFocusTrap<T extends HTMLElement = HTMLDivElement>(
  active: boolean
): { containerRef: RefObject<T> } {
  const containerRef = useRef<T>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!active) return;

    previousFocusRef.current = document.activeElement as HTMLElement;

    const container = containerRef.current;
    if (!container) return;

    // Move focus into the trap
    focusFirstIn(container);

    // Set up the keyboard trap
    const release = trapFocus(container);

    return () => {
      release();
      restoreFocus(previousFocusRef.current);
    };
  }, [active]);

  return { containerRef };
}

// ── useAnnounce ───────────────────────────────────────────────────────────────
/**
 * Returns an `announce` function that posts messages to an ARIA live region.
 * Useful for notifying screen readers of async state changes (scan complete,
 * error appeared, etc.) without moving focus.
 *
 * @example
 * const announce = useAnnounce();
 * announce("Scan complete. 3 vulnerabilities found.", "assertive");
 */
export function useAnnounce() {
  return useCallback(
    (message: string, politeness: "polite" | "assertive" = "polite") => {
      liveRegion.announce(message, politeness);
    },
    []
  );
}

// ── useReducedMotion ──────────────────────────────────────────────────────────
/**
 * Reactively tracks `prefers-reduced-motion`.
 * Use to disable or simplify animations for users who request it.
 *
 * @example
 * const reduced = useReducedMotion();
 * <motion.div animate={reduced ? {} : { opacity: 1 }} />
 */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(prefersReducedMotion);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return reduced;
}

// ── useKeyboardNavigation ─────────────────────────────────────────────────────
/**
 * Adds arrow-key roving tabindex navigation for list / toolbar / tab widgets.
 * Conforms to ARIA Authoring Practices Guide composite widget pattern.
 *
 * @param orientation  "horizontal" | "vertical" | "both"
 * @param wrap         Whether navigation wraps around at the ends
 *
 * @example
 * const { containerRef } = useKeyboardNavigation("horizontal");
 * <ul ref={containerRef} role="tablist">
 *   <li role="tab" tabIndex={0}>Tab 1</li>
 *   <li role="tab" tabIndex={-1}>Tab 2</li>
 * </ul>
 */
export function useKeyboardNavigation<T extends HTMLElement = HTMLUListElement>(
  orientation: "horizontal" | "vertical" | "both" = "horizontal",
  wrap = true
): { containerRef: RefObject<T> } {
  const containerRef = useRef<T>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handler = (e: KeyboardEvent) => {
      const items = getFocusableElements(container);
      if (items.length === 0) return;

      const currentIndex = items.indexOf(document.activeElement as HTMLElement);
      if (currentIndex === -1) return;

      const goTo = (index: number) => {
        const clamped = wrap
          ? (index + items.length) % items.length
          : Math.max(0, Math.min(index, items.length - 1));
        items[clamped].focus();
        e.preventDefault();
      };

      const isHoriz = orientation === "horizontal" || orientation === "both";
      const isVert = orientation === "vertical" || orientation === "both";

      switch (e.key) {
        case "ArrowRight": if (isHoriz) goTo(currentIndex + 1); break;
        case "ArrowLeft":  if (isHoriz) goTo(currentIndex - 1); break;
        case "ArrowDown":  if (isVert)  goTo(currentIndex + 1); break;
        case "ArrowUp":    if (isVert)  goTo(currentIndex - 1); break;
        case "Home":       goTo(0);               break;
        case "End":        goTo(items.length - 1); break;
      }
    };

    container.addEventListener("keydown", handler);
    return () => container.removeEventListener("keydown", handler);
  }, [orientation, wrap]);

  return { containerRef };
}

// ── useAriaId ─────────────────────────────────────────────────────────────────
/**
 * Returns a stable, unique ID suitable for aria-labelledby / aria-describedby.
 * Uses React 18's useId for SSR-safe generation.
 *
 * @example
 * const { labelId, descId } = useAriaIds("scan-form");
 * <label id={labelId}>Contract ID</label>
 * <input aria-labelledby={labelId} aria-describedby={descId} />
 * <p id={descId}>Enter a valid Stellar contract address.</p>
 */
export function useAriaIds(prefix = "aria") {
  const base = useId();
  return {
    labelId: `${prefix}-label-${base}`,
    descId: `${prefix}-desc-${base}`,
    errorId: `${prefix}-error-${base}`,
    statusId: `${prefix}-status-${base}`,
  };
}

// ── useSkipLink ───────────────────────────────────────────────────────────────
/**
 * Wires up a skip-link button to jump to a landmark by ID.
 * Attach `skipLinkProps` to your skip-link <a> element.
 *
 * @example
 * const { skipLinkProps } = useSkipLink("main-content");
 * <a {...skipLinkProps} className="skip-link">Skip to main content</a>
 * <main id="main-content">...</main>
 */
export function useSkipLink(targetId: string) {
  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLAnchorElement>) => {
      e.preventDefault();
      const target = document.getElementById(targetId);
      if (!target) return;
      const hadTabIndex = target.hasAttribute("tabindex");
      if (!hadTabIndex) target.setAttribute("tabindex", "-1");
      target.focus({ preventScroll: false });
      if (!hadTabIndex) {
        target.addEventListener("blur", () => target.removeAttribute("tabindex"), {
          once: true,
        });
      }
    },
    [targetId]
  );

  return {
    skipLinkProps: {
      href: `#${targetId}`,
      onClick: handleClick,
    },
  };
}

// ── useHighContrast ───────────────────────────────────────────────────────────
/**
 * Detects Windows High Contrast / forced-colors mode.
 * Use to conditionally swap SVG icons for text alternatives, etc.
 */
export function useHighContrast(): boolean {
  const [highContrast, setHighContrast] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(forced-colors: active)");
    setHighContrast(mq.matches);
    const handler = (e: MediaQueryListEvent) => setHighContrast(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return highContrast;
}

// ── useEscapeKey ──────────────────────────────────────────────────────────────
/**
 * Calls `onEscape` when the Escape key is pressed while `active`.
 * Essential for dismissible dialogs, menus, tooltips.
 */
export function useEscapeKey(onEscape: () => void, active = true): void {
  useEffect(() => {
    if (!active) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onEscape();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onEscape, active]);
}