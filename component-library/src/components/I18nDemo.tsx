import React from 'react';
import { useTranslation } from 'react-i18next';
import { LanguageSelector } from './LanguageSelector';
import { useRTLStyles } from '../hooks/useRTL';

export function I18nDemo() {
  const { t } = useTranslation('common');
  const { getStyle, getTextAlign, getMarginStyle, direction } = useRTLStyles();

  const currentDate = new Date();
  const sampleAmount = 1234.56;

  return (
    <div className="i18n-demo" dir={direction} style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto' }}>
      <header style={{ ...getMarginStyle(0, 'auto'), textAlign: 'center', marginBottom: '2rem' }}>
        <h1>{t('scanner.name')}</h1>
        <p>{t('scanner.description')}</p>
      </header>

      <section style={{ marginBottom: '2rem' }}>
        <h2>{t('commands.scan.description')}</h2>
        <p>{t('commands.scan.starting')}</p>
        <p>{t('commands.scan.emergency_stop_enabled')}</p>
        <p>{t('commands.scan.notifications_enabled')}</p>
      </section>

      <section style={{ marginBottom: '2rem' }}>
        <h2>Language Selection</h2>
        <LanguageSelector showFlags={true} />
      </section>

      <section style={{ marginBottom: '2rem' }}>
        <h2>Localization Examples</h2>
        
        <div style={{ marginBottom: '1rem' }}>
          <h3>Date Formatting:</h3>
          <p>{currentDate.toLocaleDateString()}</p>
        </div>

        <div style={{ marginBottom: '1rem' }}>
          <h3>Currency Formatting:</h3>
          <p>{sampleAmount.toLocaleString('en-US', { style: 'currency', currency: 'USD' })}</p>
        </div>

        <div style={{ marginBottom: '1rem' }}>
          <h3>Text Direction:</h3>
          <p style={{ textAlign: getTextAlign() }}>
            Current text direction: {direction}
          </p>
        </div>
      </section>

      <section style={{ marginBottom: '2rem' }}>
        <h2>{t('reporter.recommendations')}</h2>
        <ul style={{ textAlign: getTextAlign() }}>
          <li>{t('reporter.timestamp_validation')}</li>
          <li>{t('reporter.timestamp_bounds')}</li>
          <li>{t('reporter.block_heights')}</li>
          <li>{t('reporter.replay_protection')}</li>
        </ul>
      </section>

      <section style={{ marginBottom: '2rem' }}>
        <h2>RTL Support Test</h2>
        <div style={{
          ...getMarginStyle('1rem', '1rem'),
          padding: '1rem',
          border: '1px solid #ddd',
          borderRadius: '4px',
          textAlign: getTextAlign()
        }}>
          <p>This is a test of RTL language support.</p>
          <p>Text alignment and margins should adjust based on language direction.</p>
          <p>Current direction: <strong>{direction}</strong></p>
        </div>
      </section>

      <section style={{ marginBottom: '2rem' }}>
        <h2>Emergency Stop Commands</h2>
        <div style={{ textAlign: getTextAlign() }}>
          <p>{t('commands.emergency_stop.description')}</p>
          <p>{t('commands.emergency_stop.testing')}</p>
          <p>{t('commands.emergency_stop.test_passed')}</p>
        </div>
      </section>

      <footer style={{ 
        ...getMarginStyle('2rem', 0), 
        textAlign: 'center', 
        borderTop: '1px solid #ddd',
        paddingTop: '1rem'
      }}>
        <p>{t('scanner.name')} - {t('scanner.version')}</p>
      </footer>
    </div>
  );
}

export default I18nDemo;
