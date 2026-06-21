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

## RTL Support

### CSS Classes
The RTL CSS provides utility classes for right-to-left layouts:

```css
[dir="rtl"] .text-start { text-align: right; }
[dir="rtl"] .ms-1 { margin-left: 0; margin-right: 0.25rem; }
[dir="rtl"] .border-start { border-left: none; border-right: 1px solid #dee2e6; }
```

### Automatic Direction Detection
The system automatically detects RTL languages and applies appropriate styling:

```tsx
// Document direction is automatically set
document.documentElement.dir = direction; // 'rtl' or 'ltr'
document.body.classList.add('rtl'); // or 'ltr'
```

## Adding New Languages

To add a new language:

1. **Create Translation File**
```bash
mkdir locales/fr
cp locales/en/common.json locales/fr/common.json
```

2. **Translate Content**
Edit `locales/fr/common.json` with French translations.

3. **Update Configuration**
Add the language to the supported languages list:

```javascript
// src/i18n/config.js
supportedLngs: ['en', 'es', 'ar', 'fr']
```

4. **Add RTL Support (if needed)**
If the new language is RTL, add it to the RTL languages list:

```javascript
// component-library/src/i18n/config.ts
const rtlLanguages = ['ar', 'he', 'fa', 'ur', 'fr']; // if French was RTL
```

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
