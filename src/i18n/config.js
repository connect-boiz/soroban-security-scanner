const i18next = require('i18next');

// Load translation resources synchronously for test/CLI usage
const enCommon = require('../../locales/en/common.json');
const esCommon = require('../../locales/es/common.json');
const arCommon = require('../../locales/ar/common.json');

const i18nConfig = {
  // Supported languages
  supportedLngs: ['en', 'es', 'ar'],

  // Fallback language
  fallbackLng: 'en',

  // Debug mode (disable in production)
  lng: (process.env.LANG || 'en').split('.')[0].split('_')[0],
  debug: process.env.NODE_ENV === 'development',

  // Resources loaded directly (no fs-backend needed)
  resources: {
    en: { common: enCommon },
    es: { common: esCommon },
    ar: { common: arCommon },
  },

  // Interpolation configuration
  interpolation: {
    escapeValue: false,
    format: function (value, format, lng) {
      if (format === 'currency') {
        const currencyFormats = {
          en: 'en-US',
          es: 'es-ES',
          ar: 'ar-SA',
        };
        return new Intl.NumberFormat(currencyFormats[lng] || 'en-US', {
          style: 'currency',
          currency: 'USD',
        }).format(value);
      }

      if (format === 'date') {
        const dateFormats = {
          en: 'en-US',
          es: 'es-ES',
          ar: 'ar-SA',
        };
        return new Intl.DateTimeFormat(dateFormats[lng] || 'en-US').format(new Date(value));
      }

      return value;
    },
  },
};

// Initialize i18next synchronously
i18next.init(i18nConfig);

/**
 * Initialize i18next (synchronous, already done above)
 */
async function initializeI18n() {
  return i18next;
}

/**
 * Get localized text
 */
function t(key, options = {}) {
  return i18next.t(key, options);
}

/**
 * Check if language is RTL
 */
function isRTL(language) {
  const rtlLanguages = ['ar', 'he', 'fa', 'ur'];
  return rtlLanguages.includes(language);
}

/**
 * Get text direction for language
 */
function getTextDirection(language) {
  return isRTL(language) ? 'rtl' : 'ltr';
}

/**
 * Format currency for language
 */
function formatCurrency(amount, language = 'en', currency = 'USD') {
  const currencyFormats = {
    en: 'en-US',
    es: 'es-ES',
    ar: 'ar-SA',
  };

  return new Intl.NumberFormat(currencyFormats[language] || 'en-US', {
    style: 'currency',
    currency: currency,
  }).format(amount);
}

/**
 * Format date for language
 */
function formatDate(date, language = 'en', options = {}) {
  const dateFormats = {
    en: 'en-US',
    es: 'es-ES',
    ar: 'ar-SA',
  };

  const defaultOptions = {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  };

  return new Intl.DateTimeFormat(dateFormats[language] || 'en-US', {
    ...defaultOptions,
    ...options,
  }).format(new Date(date));
}

module.exports = {
  initializeI18n,
  t,
  i18nConfig,
  isRTL,
  getTextDirection,
  formatCurrency,
  formatDate,
  i18next,
};
