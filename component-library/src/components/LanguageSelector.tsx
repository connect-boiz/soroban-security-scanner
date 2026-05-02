import React from 'react';
import { useTranslation } from 'react-i18next';
import { useLanguage, supportedLanguages } from '../i18n/config';

interface LanguageSelectorProps {
  className?: string;
  showFlags?: boolean;
}

export function LanguageSelector({ className, showFlags = true }: LanguageSelectorProps) {
  const { i18n } = useTranslation();
  const { currentLanguage, changeLanguage, direction } = useLanguage();

  const handleLanguageChange = async (languageCode: string) => {
    await changeLanguage(languageCode);
  };

  const getLanguageFlag = (languageCode: string) => {
    const flags: Record<string, string> = {
      'en': '🇺🇸',
      'es': '🇪🇸',
      'ar': '🇸🇦'
    };
    return flags[languageCode] || '';
  };

  return (
    <div 
      className={`language-selector ${className || ''}`}
      dir={direction}
    >
      <select
        value={currentLanguage}
        onChange={(e) => handleLanguageChange(e.target.value)}
        className="language-select"
        aria-label="Select language"
      >
        {supportedLanguages.map((lang) => (
          <option key={lang.code} value={lang.code}>
            {showFlags && `${getLanguageFlag(lang.code)} `}
            {lang.name}
          </option>
        ))}
      </select>
      
      {showFlags && (
        <div className="language-flags" role="group" aria-label="Quick language switch">
          {supportedLanguages.map((lang) => (
            <button
              key={lang.code}
              onClick={() => handleLanguageChange(lang.code)}
              className={`language-flag-button ${currentLanguage === lang.code ? 'active' : ''}`}
              title={lang.name}
              aria-label={`Switch to ${lang.name}`}
              aria-pressed={currentLanguage === lang.code}
            >
              {getLanguageFlag(lang.code)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default LanguageSelector;
