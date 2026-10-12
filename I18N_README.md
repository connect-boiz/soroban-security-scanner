# Internationalization (i18n) Support

This document describes the internationalization features added to the Soroban Security Scanner.

## Overview

The Soroban Security Scanner now supports full internationalization with:
- **Multiple Languages**: English (en), Spanish (es), and Arabic (ar)
- **RTL Support**: Right-to-Left language support for Arabic
- **Currency & Date Formatting**: Localized formatting for different regions
- **CLI & UI**: Both command-line and React components are internationalized
- **Dynamic Language Switching**: Runtime language switching for web interface

## Supported Languages

| Language | Code | Direction | Status |
|----------|------|-----------|---------|
| English | `en` | LTR | ✅ Complete |
| Spanish | `es` | LTR | ✅ Complete |
| Arabic | `ar` | RTL | ✅ Complete |

## Features

### 1. Command Line Interface (CLI)

The CLI tool now supports multiple languages through environment variables:

```bash
# Set language for CLI
export LANG=es  # Spanish
export LANG=ar  # Arabic
export LANG=en  # English (default)

# Run scanner with localized output
soroban-scanner scan contract.rs
```

### 2. React Components

#### Language Selector Component
```tsx
import { LanguageSelector } from '@soroban-scanner/ui-components';

function App() {
  return <LanguageSelector showFlags={true} />;
}
```

#### RTL Support Hook
```tsx
import { useRTL, useRTLStyles } from '@soroban-scanner/ui-components';

function MyComponent() {
  const { direction, isRTL } = useRTL();
  const { getMarginStyle, getTextAlign } = useRTLStyles();
  
  return (
    <div dir={direction} style={{ textAlign: getTextAlign() }}>
      Content
    </div>
  );
}
```

#### Language Management Hook
```tsx
import { useLanguage } from '@soroban-scanner/ui-components';

function LanguageManager() {
  const { currentLanguage, changeLanguage, supportedLanguages } = useLanguage();
  
  const handleLanguageChange = (lang) => {
    await changeLanguage(lang);
  };
  
  return (
    <select value={currentLanguage} onChange={handleLanguageChange}>
      {supportedLanguages.map(lang => (
        <option key={lang.code} value={lang.code}>
          {lang.name}
        </option>
      ))}
    </select>
  );
}
```

### 3. Currency & Date Formatting

```tsx
import { formatCurrency, formatDate } from '@soroban-scanner/ui-components';

// Format currency for different locales
const price = formatCurrency(1234.56, 'ar', 'USD'); // $1,234.56 (Arabic formatting)
const priceES = formatCurrency(1234.56, 'es', 'EUR'); // €1.234,56 (Spanish formatting)

// Format dates for different locales
const date = formatDate(new Date(), 'ar'); // Arabic date format
const dateES = formatDate(new Date(), 'es'); // Spanish date format
```

### 4. Notification Service

Email and in-app notifications are now localized:

```javascript
// Templates automatically use the current language
const notification = await service.sendTemplatedNotification(
  'vulnerability_alert',
  recipient,
  context,
  channels,
  priority
);
```

### 5. Security Reports

Security scan reports are generated in the appropriate language:

```javascript
// Reports use localized text and formatting
const reporter = new SecurityReporter();
const report = reporter.generate(vulnerabilities, 'text');
```

## File Structure

```
soroban-security-scanner/
├── locales/
│   ├── en/
│   │   └── common.json
│   ├── es/
│   │   └── common.json
│   └── ar/
│       └── common.json
├── src/
│   └── i18n/
│       └── config.js
├── component-library/
│   ├── src/
│   │   ├── i18n/
│   │   │   └── config.ts
│   │   ├── components/
│   │   │   ├── LanguageSelector.tsx
│   │   │   └── I18nDemo.tsx
│   │   ├── hooks/
│   │   │   └── useRTL.ts
│   │   └── styles/
│   │       └── rtl.css
```

## Translation Keys

Translation keys are organized in a hierarchical structure:

```json
{
  "scanner": {
    "name": "Soroban Security Scanner",
    "description": "Security scanner for Soroban smart contracts"
  },
  "commands": {
    "scan": {
      "description": "Scan for security vulnerabilities",
      "starting": "🔍 Starting Soroban Security Scanner..."
    }
  },
  "reporter": {
    "recommendations": "📋 RECOMMENDATIONS:",
    "high_priority": "HIGH PRIORITY:"
  }
}
```

## RTL Support & Layout System

The application features full Right-to-Left (RTL) language and layout support for Arabic (`ar`), Hebrew (`he`), Persian (`fa`), and Urdu (`ur`).

### 1. Root Layout Direction (`layout.tsx`)
In `frontend/app/layout.tsx`, the `<html>` root element dynamically sets `dir="rtl"` and `lang="ar"` (or `he`) based on the `RTL_LOCALES` constant:
- **Locale Resolution**: Server-side resolution inspects `x-locale` / `x-lang` headers, cookies (`i18nextLng`, `NEXT_LOCALE`), and `Accept-Language`.
- **Direction Assignment**: Automatically applies `dir="rtl"` or `dir="ltr"` to `<html>`, ensuring browser layout engine and CSS logical properties immediately flow right-to-left.

### 2. Tailwind CSS RTL Variants (`tailwind.config.a11y.js`)
Tailwind CSS is configured with native `rtl:` and `ltr:` variant prefixes and bidirectional utility classes:
- **Variant Prefixes**: `rtl:mr-4`, `rtl:text-right`, `rtl:flex-row-reverse`, `rtl:border-r`.
- **Bidirectional Utilities**: `.text-start`, `.text-end`, `.flip-rtl` (mirrors directional icons), and `.no-flip-rtl`.
- **Safelist**: All `rtl:` and `ltr:` utility patterns are safelisted for dynamic rendering.

### 3. Text Direction Hook (`useTextDirection`)
The `useTextDirection()` hook in `frontend/hooks/useTextDirection.ts` returns `'ltr'` or `'rtl'`:
```tsx
import { useTextDirection } from '@/hooks/useTextDirection';

function Navigation() {
  const direction = useTextDirection(); // 'ltr' | 'rtl'
  const isRtl = direction === 'rtl';

  return <nav dir={direction}>{/* ... */}</nav>;
}
```
The hook reacts in real-time to changes in `document.documentElement.dir` or `document.documentElement.lang` using `MutationObserver`.

### 4. RTL Layout Primitives (`@soroban-scanner/ui-components`)
- **`RtlContainer`**: Layout primitive wrapper that enforces `dir` and automatically flips flex direction (`flex-row` becomes `flex-row-reverse`) in RTL layouts:
  ```tsx
  import { RtlContainer } from '@soroban-scanner/ui-components';

  <RtlContainer className="flex flex-row items-center gap-4">
    <Avatar />
    <UserDetails />
  </RtlContainer>
  ```
- **`RtlAwareIcon`**: Automatically mirrors directional icons (arrows, chevrons, forward/back) horizontally in RTL mode:
  ```tsx
  import { RtlAwareIcon } from '@soroban-scanner/ui-components';

  <button>
    <span>Next</span>
    <RtlAwareIcon>
      <ArrowRightIcon />
    </RtlAwareIcon>
  </button>
  ```

### 5. Accessible Directional Primitives (`A11yPrimitives.tsx`)
- **`SkipLink`**: Positions focus ring at `top-4 right-4` in RTL and `top-4 left-4` in LTR.
- **`AccessibleFormField`**: Form labels right-align and required asterisk marker margin swaps to the left (`mr-1` in RTL).
- **Directional ARIA & Keyboard Navigation**: `getDirectionalAria()`, `DirectionalNavigationButton`, and `DirectionalToolbar` swap `ArrowLeft`/`ArrowRight` roving tabindex and placement attributes based on text direction.

---

## Adding New RTL Locales

Follow these steps to add a new right-to-left language (e.g. `ps` for Pashto or `yi` for Yiddish):

1. **Add to `RTL_LOCALES` Constants**:
   Add the 2-letter language code to the `RTL_LOCALES` array in:
   - `frontend/app/layout.tsx`: `export const RTL_LOCALES = ['ar', 'he', 'fa', 'ur', '<code-here>'] as const;`
   - `frontend/hooks/useTextDirection.ts`: `export const RTL_LOCALES = ['ar', 'he', 'fa', 'ur', '<code-here>'] as const;`
   - `component-library/src/i18n/config.ts`: `export const RTL_LOCALES = ['ar', 'he', 'fa', 'ur', '<code-here>'] as const;`
   - `src/i18n/config.js`: `const RTL_LOCALES = ['ar', 'he', 'fa', 'ur', '<code-here>'];`

2. **Register in Supported Languages**:
   In `component-library/src/i18n/config.ts`:
   ```typescript
   export const supportedLanguages = [
     // ...
     { code: '<code-here>', name: '<Native Name>', dir: 'rtl' }
   ];
   ```

3. **Create Translation Resource**:
   ```bash
   mkdir locales/<code-here>
   cp locales/en/common.json locales/<code-here>/common.json
   ```
   Translate the UI strings in `locales/<code-here>/common.json`.

4. **Verify Direction & Visuals**:
   - Verify `RootLayout` renders `<html dir="rtl" lang="<code-here>">`.
   - Ensure directional icons wrapped with `RtlAwareIcon` are horizontally mirrored.
   - Verify sidebar and navigation flow from right to left using `RtlContainer`.


## Testing

Run the i18n test to verify the implementation:

```bash
node test-i18n.js
```

## Browser Compatibility

- **Modern Browsers**: Full support for all features
- **Legacy Browsers**: Basic i18n support (no RTL auto-detection)
- **Node.js**: Full CLI internationalization support

## Performance Considerations

- Translation files are loaded on-demand
- RTL styles are conditionally applied
- Language detection is cached
- Minimal runtime overhead

## Accessibility

- Proper `dir` attributes for screen readers
- Semantic HTML structure maintained
- ARIA labels for language controls
- High contrast mode support

## Troubleshooting

### Language Not Switching
- Check that translation files exist
- Verify language codes are correct
- Ensure i18n is initialized before use

### RTL Not Working
- Confirm language is in RTL list
- Check CSS classes are applied
- Verify document direction attribute

### Missing Translations
- Translation keys fall back to English
- Check console for missing key warnings
- Verify JSON structure is correct

## Contributing

When contributing to i18n:

1. **New Features**: Add translation keys for all supported languages
2. **Text Changes**: Update all translation files, not just English
3. **Testing**: Test with all languages, especially RTL
4. **Documentation**: Update this README for new features

## Future Enhancements

- **More Languages**: Easy to add additional language support
- **Pluralization**: Advanced plural rules for different languages
- **Gender**: Gender-specific translations where applicable
- **Region-Specific**: Country-specific formatting (en-US vs en-GB)
- **Dynamic Loading**: Lazy loading of translation files
- **Browser Detection**: Automatic language detection from browser settings
