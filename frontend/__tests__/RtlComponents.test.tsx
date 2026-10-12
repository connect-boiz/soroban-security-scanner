import React from 'react';
import { render, screen } from '@testing-library/react';
import RtlContainer from '../components/RtlContainer';
import RtlAwareIcon from '../components/RtlAwareIcon';

describe('RtlContainer & RtlAwareIcon Components', () => {
  describe('RtlContainer', () => {
    it('renders with dir="rtl" and reverses flex-row in RTL mode', () => {
      const { container } = render(
        <RtlContainer dir="rtl" className="flex flex-row gap-4">
          <div>Item 1</div>
          <div>Item 2</div>
        </RtlContainer>
      );

      const el = container.firstChild as HTMLElement;
      expect(el).toHaveAttribute('dir', 'rtl');
      expect(el).toHaveAttribute('data-is-rtl', 'true');
      expect(el.className).toContain('flex-row-reverse');
      expect(el.className).not.toContain('flex-row ');
    });

    it('renders with dir="ltr" and preserves flex-row in LTR mode', () => {
      const { container } = render(
        <RtlContainer dir="ltr" className="flex flex-row gap-4">
          <div>Item 1</div>
          <div>Item 2</div>
        </RtlContainer>
      );

      const el = container.firstChild as HTMLElement;
      expect(el).toHaveAttribute('dir', 'ltr');
      expect(el).toHaveAttribute('data-is-rtl', 'false');
      expect(el.className).toContain('flex-row');
      expect(el.className).not.toContain('flex-row-reverse');
    });

    it('supports custom as tag', () => {
      const { container } = render(
        <RtlContainer as="section" dir="rtl">
          <p>Section content</p>
        </RtlContainer>
      );

      expect(container.querySelector('section')).not.toBeNull();
    });
  });

  describe('RtlAwareIcon', () => {
    it('applies transform: scaleX(-1) and mirrored class in RTL mode', () => {
      const { container } = render(
        <RtlAwareIcon dir="rtl" data-testid="arrow-icon">
          <span>➔</span>
        </RtlAwareIcon>
      );

      const el = container.querySelector('[data-testid="arrow-icon"]') as HTMLElement;
      expect(el).toHaveAttribute('data-mirrored', 'true');
      expect(el.className).toContain('mirrored');
      expect(el.style.transform).toBe('scaleX(-1)');
    });

    it('does not mirror in LTR mode', () => {
      const { container } = render(
        <RtlAwareIcon dir="ltr" data-testid="arrow-icon">
          <span>➔</span>
        </RtlAwareIcon>
      );

      const el = container.querySelector('[data-testid="arrow-icon"]') as HTMLElement;
      expect(el).toHaveAttribute('data-mirrored', 'false');
      expect(el.className).not.toContain('mirrored');
      expect(el.style.transform).toBe('');
    });

    it('does not mirror when mirror={false} even in RTL mode', () => {
      const { container } = render(
        <RtlAwareIcon dir="rtl" mirror={false} data-testid="arrow-icon">
          <span>➔</span>
        </RtlAwareIcon>
      );

      const el = container.querySelector('[data-testid="arrow-icon"]') as HTMLElement;
      expect(el).toHaveAttribute('data-mirrored', 'false');
      expect(el.style.transform).toBe('');
    });
  });
});
