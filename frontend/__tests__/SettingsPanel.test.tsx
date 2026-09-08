import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import SettingsPanel from '../components/SettingsPanel';

describe('SettingsPanel', () => {
  beforeEach(() => {
    localStorage.clear();
    jest.useFakeTimers();
  });
  afterEach(() => {
    jest.useRealTimers();
  });

  /** Render and advance through the simulated initial loading. */
  const renderPanel = async () => {
    const utils = render(<SettingsPanel />);
    // The loading effect chains `await new Promise(setTimeout)` stages, so
    // advance in steps and flush microtasks between each one.
    for (let i = 0; i < 8; i++) {
      await act(async () => {
        jest.advanceTimersByTime(300);
        await Promise.resolve();
      });
    }
    return utils;
  };

  it('shows a skeleton until client-side loading completes', async () => {
    const { container } = render(<SettingsPanel />);
    expect(container.querySelector('.skeleton')).not.toBeNull();
    for (let i = 0; i < 8; i++) {
      await act(async () => {
        jest.advanceTimersByTime(300);
        await Promise.resolve();
      });
    }
    expect(screen.getAllByRole('heading', { name: 'Settings' }).length).toBeGreaterThan(0);
  });

  it('switches between tabs', async () => {
    await renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /security settings/i }));
    expect(screen.getAllByText('Security Settings').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: /theme customization/i }));
    expect(screen.getAllByText('Theme Customization').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: /account management/i }));
    expect(screen.getAllByText('Account Management').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: /user preferences/i }));
    expect(screen.getAllByText('User Preferences').length).toBeGreaterThan(0);
  });

  it('updates preference fields', async () => {
    await renderPanel();
    const [language, displayMode] = screen.getAllByRole('combobox');
    fireEvent.change(language, { target: { value: 'es' } });
    expect(language).toHaveValue('es');

    fireEvent.click(screen.getByLabelText('Enable notifications'));
    expect(screen.getByLabelText('Enable notifications')).not.toBeChecked();

    fireEvent.change(displayMode, { target: { value: 'compact' } });
    expect(displayMode).toHaveValue('compact');
  });

  it('updates security settings', async () => {
    await renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /security settings/i }));
    const [sensitivity] = screen.getAllByRole('combobox');
    fireEvent.change(sensitivity, { target: { value: 'high' } });
    expect(sensitivity).toHaveValue('high');

    fireEvent.change(screen.getByPlaceholderText('Enter your API key'), {
      target: { value: 'secret-key' },
    });
    fireEvent.click(screen.getByLabelText('Enable automatic scanning'));
    expect(screen.getByLabelText('Enable automatic scanning')).toBeChecked();

    fireEvent.change(screen.getByDisplayValue('24'), { target: { value: '48' } });
    expect(screen.getByDisplayValue('48')).toBeInTheDocument();
  });

  it('updates theme settings', async () => {
    await renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /theme customization/i }));
    const [themeMode] = screen.getAllByRole('combobox');
    fireEvent.change(themeMode, { target: { value: 'dark' } });
    expect(themeMode).toHaveValue('dark');
    fireEvent.click(screen.getByLabelText('High contrast mode'));
    expect(screen.getByLabelText('High contrast mode')).toBeChecked();
  });

  it('updates account settings', async () => {
    await renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /account management/i }));
    fireEvent.change(screen.getByDisplayValue('user'), { target: { value: 'alice' } });
    expect(screen.getByDisplayValue('alice')).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText('Allow data export'));
    expect(screen.getByLabelText('Allow data export')).toBeChecked();
  });

  it('saves settings to localStorage and shows a confirmation', async () => {
    await renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /save settings/i }));
    for (let i = 0; i < 6; i++) {
      await act(async () => {
        jest.advanceTimersByTime(300);
        await Promise.resolve();
      });
    }
    expect(screen.getByText('Settings saved successfully!')).toBeInTheDocument();
    expect(localStorage.getItem('userPreferences')).toContain('en');
    expect(localStorage.getItem('securitySettings')).toContain('medium');
    expect(localStorage.getItem('themeSettings')).toContain('system');
    expect(localStorage.getItem('accountSettings')).toContain('user@example.com');
  });

  it('resets settings to defaults', async () => {
    await renderPanel();
    const [language] = screen.getAllByRole('combobox');
    fireEvent.change(language, { target: { value: 'ar' } });
    expect(language).toHaveValue('ar');
    fireEvent.click(screen.getByRole('button', { name: /reset to defaults/i }));
    expect(screen.getByText('Settings reset to defaults!')).toBeInTheDocument();
    expect(screen.getAllByRole('combobox')[0]).toHaveValue('en');
  });

  it('restores saved settings from localStorage on mount', async () => {
    localStorage.setItem('userPreferences', JSON.stringify({ language: 'ar' }));
    await renderPanel();
    expect(screen.getAllByRole('combobox')[0]).toHaveValue('ar');
  });
});
