'use client';

import React from 'react';
import { SorobanVariable } from '@/types/invariant';

interface ValueInputProps {
  value: string;
  valueType: 'string' | 'number' | 'address' | 'boolean';
  variable: SorobanVariable;
  onValueChange: (value: string) => void;
}

const ValueInput: React.FC<ValueInputProps> = ({
  value,
  valueType,
  variable,
  onValueChange
}) => {
  const [isFocused, setIsFocused] = React.useState(false);

  // Handle input changes based on type
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    
    // Type-specific validation
    if (valueType === 'number') {
      // Allow numbers and basic arithmetic expressions
      const sanitizedValue = newValue.replace(/[^0-9+\-*/.() ]/g, '');
      onValueChange(sanitizedValue);
    } else if (valueType === 'address') {
      // Basic Stellar address validation (starts with G and is 56 chars)
      const sanitizedValue = newValue.toUpperCase().replace(/[^G0-9]/g, '');
      onValueChange(sanitizedValue);
    } else {
      onValueChange(newValue);
    }
  };

  // Render different input types based on valueType
  const renderInput = () => {
    switch (valueType) {
      case 'boolean':
        return (
          <select
            value={value || 'true'}
            onChange={handleInputChange}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 text-sm"
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
          >
            <option value="true">True</option>
            <option value="false">False</option>
          </select>
        );

      case 'number':
        return (
          <input
            type="text"
            value={value}
            onChange={handleInputChange}
            placeholder="e.g., 100, 0.5, 1000000"
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 text-sm font-mono"
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
          />
        );

      case 'address':
        return (
          <div className="relative">
            <input
              type="text"
              value={value}
              onChange={handleInputChange}
              placeholder="G..."
              maxLength={56}
              className="w-full px-3 py-2 pr-8 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 text-sm font-mono"
              onFocus={() => setIsFocused(true)}
              onBlur={() => setIsFocused(false)}
            />
            {value && value.length > 0 && (
              <div className="absolute right-2 top-2.5">
                <span className={`text-xs px-1 py-0.5 rounded ${
                  value.length === 56 && value.startsWith('G')
                    ? 'bg-green-100 text-green-700'
                    : 'bg-yellow-100 text-yellow-700'
                }`}>
                  {value.length}/56
                </span>
              </div>
            )}
          </div>
        );

      case 'string':
      default:
        return (
          <textarea
            value={value}
            onChange={handleInputChange}
            placeholder="Enter value..."
            rows={1}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 text-sm resize-none"
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
            onKeyPress={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                e.currentTarget.blur();
              }
            }}
          />
        );
    }
  };

  // Get helper text based on variable and type
  const getHelperText = () => {
    if (!isFocused) return '';

    switch (valueType) {
      case 'number':
        return 'Enter a number or arithmetic expression (e.g., 100, 0.5, total_supply * 0.1)';
      
      case 'address':
        return 'Enter a Stellar address (starts with G, 56 characters)';
      
      case 'boolean':
        return 'Select true or false';
      
      case 'string':
        return 'Enter a string value (use quotes for exact matches)';
      
      default:
        return '';
    }
  };

  // Get validation status
  const getValidationStatus = () => {
    if (!value) return null;

    switch (valueType) {
      case 'address':
        if (value.length === 56 && value.startsWith('G')) {
          return { valid: true, message: 'Valid Stellar address' };
        } else if (value.length > 0) {
          return { valid: false, message: 'Invalid address format' };
        }
        break;
      
      case 'number':
        try {
          // Try to evaluate the expression
          const result = Function('"use strict"; return (' + value + ')')();
          if (typeof result === 'number' && !isNaN(result)) {
            return { valid: true, message: `Result: ${result}` };
          }
        } catch (e) {
          return { valid: false, message: 'Invalid number expression' };
        }
        break;
    }

    return null;
  };

  const validationStatus = getValidationStatus();

  return (
    <div>
      {renderInput()}
      
      {/* Helper text */}
      {isFocused && (
        <div className="mt-1 text-xs text-gray-500">
          {getHelperText()}
        </div>
      )}

      {/* Validation status */}
      {validationStatus && (
        <div className={`mt-1 text-xs ${
          validationStatus.valid ? 'text-green-600' : 'text-red-600'
        }`}>
          {validationStatus.message}
        </div>
      )}

      {/* Variable examples */}
      {!isFocused && variable.examples.length > 0 && (
        <div className="mt-1 text-xs text-gray-400">
          Examples: {variable.examples.slice(0, 2).join(', ')}
          {variable.examples.length > 2 && '...'}
        </div>
      )}
    </div>
  );
};

export default ValueInput;
