'use client';

import { useState } from 'react';
import { Bars3Icon, XMarkIcon } from '@heroicons/react/24/outline';

export default function Header() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  return (
    <header className="bg-white shadow-sm border-b border-gray-200">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          <div className="flex items-center">
            <div className="flex-shrink-0">
              <h1 className="text-xl font-bold text-primary-600">
                Soroban Scanner
              </h1>
            </div>
            <nav className="hidden md:ml-10 md:flex space-x-8">
              <a href="#" className="text-gray-900 hover:text-primary-600 px-3 py-2 text-sm font-medium">
                Dashboard
              </a>
              <a href="#" className="text-gray-500 hover:text-primary-600 px-3 py-2 text-sm font-medium">
                Scanner
              </a>
              <a href="#" className="text-gray-500 hover:text-primary-600 px-3 py-2 text-sm font-medium">
                Reports
              </a>
              <a href="#" className="text-gray-500 hover:text-primary-600 px-3 py-2 text-sm font-medium">
                Bounty
              </a>
            </nav>
          </div>
          
          <div className="hidden md:flex items-center space-x-4">
            <button className="btn-secondary">Sign In</button>
            <button className="btn-primary">Get Started</button>
          </div>
          
          <div className="md:hidden">
            <button
              type="button"
              className="text-gray-500 hover:text-gray-900"
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            >
              {mobileMenuOpen ? (
                <XMarkIcon className="h-6 w-6" />
              ) : (
                <Bars3Icon className="h-6 w-6" />
              )}
            </button>
          </div>
        </div>
        
        {/* Mobile menu */}
        {mobileMenuOpen && (
          <div className="md:hidden">
            <div className="px-2 pt-2 pb-3 space-y-1 sm:px-3">
              <a href="#" className="text-gray-900 hover:bg-gray-50 block px-3 py-2 rounded-md text-base font-medium">
                Dashboard
              </a>
              <a href="#" className="text-gray-500 hover:bg-gray-50 block px-3 py-2 rounded-md text-base font-medium">
                Scanner
              </a>
              <a href="#" className="text-gray-500 hover:bg-gray-50 block px-3 py-2 rounded-md text-base font-medium">
                Reports
              </a>
              <a href="#" className="text-gray-500 hover:bg-gray-50 block px-3 py-2 rounded-md text-base font-medium">
                Bounty
              </a>
            </div>
            <div className="pt-4 pb-3 border-t border-gray-200">
              <div className="px-2 space-y-1">
                <button className="btn-secondary w-full">Sign In</button>
                <button className="btn-primary w-full">Get Started</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </header>
  );
}
