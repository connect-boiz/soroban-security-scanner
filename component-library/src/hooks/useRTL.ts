import { useEffect } from 'react';
import { useLanguage } from '../i18n/config';

interface RTLConfig {
  direction: 'rtl' | 'ltr';
  isRTL: boolean;
  textAlign: 'right' | 'left';
  marginStart: 'marginLeft' | 'marginRight';
  marginEnd: 'marginRight' | 'marginLeft';
  paddingStart: 'paddingLeft' | 'paddingRight';
  paddingEnd: 'paddingRight' | 'paddingLeft';
  borderStart: 'borderLeft' | 'borderRight';
  borderEnd: 'borderRight' | 'borderLeft';
}

/**
 * Hook for RTL (Right-to-Left) language support
 * Provides utilities for handling RTL layouts and styling
 */
export function useRTL(): RTLConfig {
  const { direction, isRTLDirection } = useLanguage();

  useEffect(() => {
    // Update document direction
    document.documentElement.dir = direction;
    document.documentElement.lang = useLanguage().currentLanguage;
    
    // Add RTL class to body for CSS targeting
    if (isRTLDirection) {
      document.body.classList.add('rtl');
      document.body.classList.remove('ltr');
    } else {
      document.body.classList.add('ltr');
      document.body.classList.remove('rtl');
    }
  }, [direction, isRTLDirection]);

  const config: RTLConfig = {
    direction,
    isRTL: isRTLDirection,
    textAlign: isRTLDirection ? 'right' : 'left',
    marginStart: isRTLDirection ? 'marginLeft' : 'marginRight',
    marginEnd: isRTLDirection ? 'marginRight' : 'marginLeft',
    paddingStart: isRTLDirection ? 'paddingLeft' : 'paddingRight',
    paddingEnd: isRTLDirection ? 'paddingRight' : 'paddingLeft',
    borderStart: isRTLDirection ? 'borderLeft' : 'borderRight',
    borderEnd: isRTLDirection ? 'borderRight' : 'borderLeft'
  };

  return config;
}

/**
 * Hook for getting RTL-aware styles
 */
export function useRTLStyles() {
  const rtl = useRTL();

  const getStyle = (property: string, ltrValue: any, rtlValue: any) => {
    return rtl.isRTL ? rtlValue : ltrValue;
  };

  const getMarginStyle = (start: number, end: number) => {
    return {
      [rtl.marginStart]: start,
      [rtl.marginEnd]: end
    };
  };

  const getPaddingStyle = (start: number, end: number) => {
    return {
      [rtl.paddingStart]: start,
      [rtl.paddingEnd]: end
    };
  };

  const getBorderStyle = (start: string, end: string) => {
    return {
      [rtl.borderStart]: start,
      [rtl.borderEnd]: end
    };
  };

  const getTextAlign = () => rtl.textAlign;

  return {
    ...rtl,
    getStyle,
    getMarginStyle,
    getPaddingStyle,
    getBorderStyle,
    getTextAlign
  };
}

export default useRTL;
