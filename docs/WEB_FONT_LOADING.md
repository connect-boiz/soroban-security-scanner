# Web Font Loading

## Applicability

The core Soroban Security Scanner is a **CLI tool** and does not load web fonts. This document applies to any future web frontend built on top of the scanner API.

## Recommended Strategy (for future Next.js frontend)

The README describes a planned Next.js 14 frontend. When that frontend is built, apply the following font loading strategy to prevent layout shifts (CLS) and improve loading times:

### 1. Use `next/font` (Next.js 13+)

```tsx
// app/layout.tsx
import { Inter } from 'next/font/google';

const inter = Inter({
  subsets: ['latin'],
  display: 'swap',   // prevents invisible text during font load
  preload: true,
});

export default function RootLayout({ children }) {
  return (
    <html lang="en" className={inter.className}>
      <body>{children}</body>
    </html>
  );
}
```

`next/font` automatically:
- Self-hosts fonts (no external network request at runtime)
- Inlines the `@font-face` CSS
- Sets `font-display: swap` to avoid invisible text

### 2. Avoid `@import` in CSS

Do **not** use `@import url('https://fonts.googleapis.com/...')` in CSS files — this blocks rendering. Use `next/font` or a `<link rel="preload">` in `<head>` instead.

### 3. Subset fonts

Only load the character subsets you need:

```tsx
const inter = Inter({ subsets: ['latin'] }); // not 'latin-ext', 'cyrillic', etc.
```

### 4. Limit font weights

```tsx
const inter = Inter({
  subsets: ['latin'],
  weight: ['400', '600'],  // only load what you use
});
```

## References

- [Next.js Font Optimization](https://nextjs.org/docs/app/building-your-application/optimizing/fonts)
- [web.dev: Best practices for fonts](https://web.dev/font-best-practices/)
