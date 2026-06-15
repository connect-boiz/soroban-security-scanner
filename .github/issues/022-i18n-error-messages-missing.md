# Issue 22: [i18n] Translation Strings Missing for Error Messages — Non-English Users See Raw Error Codes

## Description

The internationalization system configured in `src/i18n/config.js` and `component-library/src/i18n/config.ts` provides translations for UI labels and content strings. However, error messages returned from the backend API (`src/config.rs`, `src/auth/`, `src/wallet/`, etc.) are not internationalized. When an error occurs, the frontend displays the raw English error string or numeric error code directly to the user, regardless of their selected locale. For example, a user with `es` locale sees "Insufficient balance" instead of "Saldo insuficiente", and a user with `ja` locale sees "Invalid signature" instead of "無効な署名". The `I18N_README.md` and `test-i18n.js` files document translation coverage for UI components but do not address backend error responses. The backend uses `thiserror` and `anyhow` for error handling, but the error messages are constructed in English only.

## Acceptance Criteria

- [ ] Define error code constants for all backend errors (e.g., `ERR_INSUFFICIENT_BALANCE`, `ERR_INVALID_SIGNATURE`)
- [ ] Add error message translations in all locale files under `frontend/public/locales/{lang}/errors.json`
- [ ] Create an `ErrorCode` enum in `src/lib.rs` that maps to i18n keys
- [ ] Update the frontend API error handler to map error codes to translated messages using `i18next`
- [ ] Add a fallback mechanism: if a translation for the error is not found in the user's locale, fall back to English
- [ ] Write an i18n coverage test that verifies every error code has a translation in at least the top 5 locales (en, es, ja, fr, zh)

## Additional Context

Key files: `src/i18n/config.js`, `component-library/src/i18n/config.ts`, `I18N_README.md`, `test-i18n.js`, `frontend/public/locales/`.
