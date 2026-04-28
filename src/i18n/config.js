const { Backend } = require('i18next-fs-backend');
const i18next = require('i18next');

const i18nConfig = {
  // Supported languages
  supportedLngs: ['en', 'es', 'ar'],
  
  // Fallback language
  fallbackLng: 'en',
  
  // Debug mode (disable in production)
  debug: process.env.NODE_ENV === 'development',
  
  // Backend configuration
  backend: {
    loadPath: './locales/{{lng}}/{{ns}}.json',
    addPath: './locales/{{lng}}/{{ns}}.json'
  },
  
  // Interpolation configuration
  interpolation: {
    escapeValue: false, // React already escapes
    format: function(value, format, lng) {
      if (format === 'currency') {
        const currencyFormats = {
          'en': 'en-US',
          'es': 'es-ES', 
          'ar': 'ar-SA'
        };
        return new Intl.NumberFormat(currencyFormats[lng] || 'en-US', {
          style: 'currency',
          currency: 'USD'
        }).format(value);
      }
      
      if (format === 'date') {
        const dateFormats = {
          'en': 'en-US',
          'es': 'es-ES',
          'ar': 'ar-SA'
        };
        return new Intl.DateTimeFormat(dateFormats[lng] || 'en-US').format(new Date(value));
      }
      
      return value;
    }
  },
  
  // Language detection
  detection: {
    // Order of detection methods
    order: ['querystring', 'env', 'header'],
    
    // Query string parameter name
    lookupQuerystring: 'lang',
    
    // Environment variable name
    lookupFromEnv: 'LANG',
    
    // Header name
    lookupHeader: 'accept-language',
    
    // Cache
    caches: false
  }
};

/**
 * Initialize i18next with the configuration
 */
async function initializeI18n() {
  try {
    await i18next
      .use(Backend)
      .init(i18nConfig);
    
    console.log('✅ i18next initialized successfully');
    return i18next;
  } catch (error) {
    console.error('❌ Failed to initialize i18next:', error);
    throw error;
  }
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
    'en': 'en-US',
    'es': 'es-ES',
    'ar': 'ar-SA'
  };
  
  return new Intl.NumberFormat(currencyFormats[language] || 'en-US', {
    style: 'currency',
    currency: currency
  }).format(amount);
}

/**
 * Format date for language
 */
function formatDate(date, language = 'en', options = {}) {
  const dateFormats = {
    'en': 'en-US',
    'es': 'es-ES', 
    'ar': 'ar-SA'
  };
  
  const defaultOptions = {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  };
  
  return new Intl.DateTimeFormat(
    dateFormats[language] || 'en-US',
    { ...defaultOptions, ...options }
  ).format(new Date(date));
}

module.exports = {
  initializeI18n,
  t,
  i18nConfig,
  isRTL,
  getTextDirection,
  formatCurrency,
  formatDate,
  i18next
};
