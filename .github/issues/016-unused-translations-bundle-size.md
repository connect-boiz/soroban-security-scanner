# Issue 16: [Performance] Frontend Bundle Includes Unused Translations from All Locales, Doubling Initial Load Size

## Description

The frontend uses `i18next` with `i18next-fs-backend` (configured in `src/i18n/config.js` and `component-library/src/i18n/config.ts`) to load translation resources. The current configuration loads all locale JSON files at application startup, regardless of the user's selected language. With 15 supported locales and growing, the initial JavaScript bundle includes approximately 1.2MB of unused translation strings for the average user. This contributes to a poor Largest Contentful Paint (LCP) score, particularly on mobile devices with slower network connections. The `PerformanceOptimizations.md` document targets an LCP of 2.5 seconds, but loading all translations on the first request pushes this to over 3.5 seconds on 3G connections.

## Acceptance Criteria

- [ ] Implement lazy locale loading using `i18next`'s `backend` option to load only the user's current language on initial page load
- [ ] Preload the default locale (English) in the critical rendering path to avoid Flash of Untranslated Content (FUTC)
- [ ] Queue loading of additional locales in the background after the page becomes interactive
- [ ] Add a `LanguageSelector` component in `component-library/src/components/LanguageSelector.tsx` that pre-fetches the locale data when hovered
- [ ] Reduce initial bundle size by at least 500KB on the first request for non-English users
- [ ] Update Lighthouse CI thresholds in `lighthouserc.js` to reflect the improved LCP metric
- [ ] Write a performance regression test that asserts initial bundle size stays below 2MB

## Additional Context

Key files: `src/i18n/config.js`, `component-library/src/i18n/config.ts`, `lighthouserc.js`, `frontend/next.config.js`.
