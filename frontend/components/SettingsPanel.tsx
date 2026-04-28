'use client';

import React, { useState, useEffect } from 'react';

interface UserPreferences {
  language: string;
  notifications: boolean;
  autoSave: boolean;
  displayMode: 'compact' | 'detailed';
}

interface SecuritySettings {
  scanSensitivity: 'low' | 'medium' | 'high';
  autoScan: boolean;
  scanInterval: number;
  apiKey: string;
  twoFactorAuth: boolean;
}

interface ThemeSettings {
  mode: 'light' | 'dark' | 'system';
  primaryColor: string;
  fontSize: 'small' | 'medium' | 'large';
  highContrast: boolean;
}

interface AccountSettings {
  username: string;
  email: string;
  lastLogin: string;
  sessionTimeout: number;
  exportData: boolean;
}

export default function SettingsPanel() {
  const [activeTab, setActiveTab] = useState('preferences');
  const [isClient, setIsClient] = useState(false);
  
  const [userPreferences, setUserPreferences] = useState<UserPreferences>({
    language: 'en',
    notifications: true,
    autoSave: true,
    displayMode: 'detailed'
  });

  const [securitySettings, setSecuritySettings] = useState<SecuritySettings>({
    scanSensitivity: 'medium',
    autoScan: false,
    scanInterval: 24,
    apiKey: '',
    twoFactorAuth: false
  });

  const [themeSettings, setThemeSettings] = useState<ThemeSettings>({
    mode: 'system',
    primaryColor: '#3B82F6',
    fontSize: 'medium',
    highContrast: false
  });

  const [accountSettings, setAccountSettings] = useState<AccountSettings>({
    username: 'user',
    email: 'user@example.com',
    lastLogin: new Date().toISOString(),
    sessionTimeout: 60,
    exportData: false
  });

  const [savedMessage, setSavedMessage] = useState('');

  useEffect(() => {
    setIsClient(true);
    // Load saved settings from localStorage
    const savedPreferences = localStorage.getItem('userPreferences');
    const savedSecurity = localStorage.getItem('securitySettings');
    const savedTheme = localStorage.getItem('themeSettings');
    const savedAccount = localStorage.getItem('accountSettings');

    if (savedPreferences) setUserPreferences(JSON.parse(savedPreferences));
    if (savedSecurity) setSecuritySettings(JSON.parse(savedSecurity));
    if (savedTheme) setThemeSettings(JSON.parse(savedTheme));
    if (savedAccount) setAccountSettings(JSON.parse(savedAccount));
  }, []);

  const saveSettings = () => {
    localStorage.setItem('userPreferences', JSON.stringify(userPreferences));
    localStorage.setItem('securitySettings', JSON.stringify(securitySettings));
    localStorage.setItem('themeSettings', JSON.stringify(themeSettings));
    localStorage.setItem('accountSettings', JSON.stringify(accountSettings));
    
    setSavedMessage('Settings saved successfully!');
    setTimeout(() => setSavedMessage(''), 3000);
  };

  const resetSettings = () => {
    setUserPreferences({
      language: 'en',
      notifications: true,
      autoSave: true,
      displayMode: 'detailed'
    });
    setSecuritySettings({
      scanSensitivity: 'medium',
      autoScan: false,
      scanInterval: 24,
      apiKey: '',
      twoFactorAuth: false
    });
    setThemeSettings({
      mode: 'system',
      primaryColor: '#3B82F6',
      fontSize: 'medium',
      highContrast: false
    });
    setAccountSettings({
      username: 'user',
      email: 'user@example.com',
      lastLogin: new Date().toISOString(),
      sessionTimeout: 60,
      exportData: false
    });
    
    setSavedMessage('Settings reset to defaults!');
    setTimeout(() => setSavedMessage(''), 3000);
  };

  const tabs = [
    { id: 'preferences', label: 'User Preferences', icon: '⚙️' },
    { id: 'security', label: 'Security Settings', icon: '🔒' },
    { id: 'theme', label: 'Theme Customization', icon: '🎨' },
    { id: 'account', label: 'Account Management', icon: '👤' }
  ];

  if (!isClient) return <div className="skeleton h-96 w-full rounded-lg" />;

  return (
    <div className="card shadow-md animate-fade-in">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-semibold text-gray-900">Settings</h2>
        {savedMessage && (
          <div className="bg-green-100 text-green-800 px-4 py-2 rounded-xl text-sm font-bold border border-green-200">
            {savedMessage}
          </div>
        )}
      </div>

      <div className="flex space-x-1 mb-6 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-6 py-2 text-sm font-bold rounded-t-xl transition-all ${
              activeTab === tab.id
                ? 'bg-blue-50 text-blue-600 border-b-2 border-blue-600'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            }`}
          >
            <span className="mr-2">{tab.icon}</span>
            {tab.label}
          </button>
        ))}
      </div>

      <div className="space-y-6">
        {activeTab === 'preferences' && (
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-gray-900">User Preferences</h3>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Language
                </label>
                <select
                  value={userPreferences.language}
                  onChange={(e) => setUserPreferences({...userPreferences, language: e.target.value})}
                  className="input"
                >
                  <option value="en">English</option>
                  <option value="es">Spanish</option>
                  <option value="ar">Arabic</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Display Mode
                </label>
                <select
                  value={userPreferences.displayMode}
                  onChange={(e) => setUserPreferences({...userPreferences, displayMode: e.target.value as 'compact' | 'detailed'})}
                  className="input"
                >
                  <option value="compact">Compact</option>
                  <option value="detailed">Detailed</option>
                </select>
              </div>
            </div>

            <div className="space-y-3">
              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={userPreferences.notifications}
                  onChange={(e) => setUserPreferences({...userPreferences, notifications: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">Enable notifications</span>
              </label>

              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={userPreferences.autoSave}
                  onChange={(e) => setUserPreferences({...userPreferences, autoSave: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">Auto-save scan results</span>
              </label>
            </div>
          </div>
        )}

        {activeTab === 'security' && (
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-gray-900">Security Settings</h3>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Scan Sensitivity
                </label>
                <select
                  value={securitySettings.scanSensitivity}
                  onChange={(e) => setSecuritySettings({...securitySettings, scanSensitivity: e.target.value as 'low' | 'medium' | 'high'})}
                  className="input"
                >
                  <option value="low">Low</option>
                  <option value="medium">Medium</option>
                  <option value="high">High</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Scan Interval (hours)
                </label>
                <input
                  type="number"
                  min="1"
                  max="168"
                  value={securitySettings.scanInterval}
                  onChange={(e) => setSecuritySettings({...securitySettings, scanInterval: parseInt(e.target.value)})}
                  className="input"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                API Key
              </label>
              <input
                type="password"
                value={securitySettings.apiKey}
                onChange={(e) => setSecuritySettings({...securitySettings, apiKey: e.target.value})}
                placeholder="Enter your API key"
                className="input"
              />
            </div>

            <div className="space-y-3">
              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={securitySettings.autoScan}
                  onChange={(e) => setSecuritySettings({...securitySettings, autoScan: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">Enable automatic scanning</span>
              </label>

              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={securitySettings.twoFactorAuth}
                  onChange={(e) => setSecuritySettings({...securitySettings, twoFactorAuth: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">Enable two-factor authentication</span>
              </label>
            </div>
          </div>
        )}

        {activeTab === 'theme' && (
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-gray-900">Theme Customization</h3>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Theme Mode
                </label>
                <select
                  value={themeSettings.mode}
                  onChange={(e) => setThemeSettings({...themeSettings, mode: e.target.value as 'light' | 'dark' | 'system'})}
                  className="input"
                >
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                  <option value="system">System</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Font Size
                </label>
                <select
                  value={themeSettings.fontSize}
                  onChange={(e) => setThemeSettings({...themeSettings, fontSize: e.target.value as 'small' | 'medium' | 'large'})}
                  className="input"
                >
                  <option value="small">Small</option>
                  <option value="medium">Medium</option>
                  <option value="large">Large</option>
                </select>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Primary Color
              </label>
              <div className="flex items-center space-x-3">
                <input
                  type="color"
                  value={themeSettings.primaryColor}
                  onChange={(e) => setThemeSettings({...themeSettings, primaryColor: e.target.value})}
                  className="w-16 h-10 border border-gray-300 rounded cursor-pointer"
                />
                <input
                  type="text"
                  value={themeSettings.primaryColor}
                  onChange={(e) => setThemeSettings({...themeSettings, primaryColor: e.target.value})}
                  className="input flex-1"
                />
              </div>
            </div>

            <div className="space-y-3">
              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={themeSettings.highContrast}
                  onChange={(e) => setThemeSettings({...themeSettings, highContrast: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">High contrast mode</span>
              </label>
            </div>
          </div>
        )}

        {activeTab === 'account' && (
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-gray-900">Account Management</h3>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Username
                </label>
                <input
                  type="text"
                  value={accountSettings.username}
                  onChange={(e) => setAccountSettings({...accountSettings, username: e.target.value})}
                  className="input"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Email
                </label>
                <input
                  type="email"
                  value={accountSettings.email}
                  onChange={(e) => setAccountSettings({...accountSettings, email: e.target.value})}
                  className="input"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Session Timeout (minutes)
              </label>
              <input
                type="number"
                min="5"
                max="480"
                value={accountSettings.sessionTimeout}
                onChange={(e) => setAccountSettings({...accountSettings, sessionTimeout: parseInt(e.target.value)})}
                className="input"
              />
            </div>

            <div className="bg-gray-50 p-4 rounded-md">
              <p className="text-sm text-gray-600">
                <strong>Last Login:</strong> {new Date(accountSettings.lastLogin).toLocaleString()}
              </p>
            </div>

            <div className="space-y-3">
              <label className="flex items-center space-x-3">
                <input
                  type="checkbox"
                  checked={accountSettings.exportData}
                  onChange={(e) => setAccountSettings({...accountSettings, exportData: e.target.checked})}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">Allow data export</span>
              </label>
            </div>

            <div className="flex space-x-3 pt-4">
              <button className="btn btn-secondary">
                Change Password
              </button>
              <button className="btn btn-primary">
                Export Data
              </button>
              <button className="btn btn-primary bg-red-600 hover:bg-red-700 border-none">
                Delete Account
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="flex justify-between items-center pt-6 mt-6 border-t">
        <button
          onClick={resetSettings}
          className="btn btn-secondary"
        >
          Reset to Defaults
        </button>
        <button
          onClick={saveSettings}
          className="btn btn-primary px-8"
        >
          Save Settings
        </button>
      </div>
    </div>
  );
}
