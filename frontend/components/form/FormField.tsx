'use client';

import React, { forwardRef, useState } from 'react';
import { AlertCircle, CheckCircle, Loader2 } from 'lucide-react';

export interface FormFieldProps {
  name: string;
  label?: string;
  type?: 'text' | 'email' | 'password' | 'number' | 'textarea' | 'select';
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
  error?: string;
  helperText?: string;
  className?: string;
  children?: React.ReactNode;
  options?: Array<{ value: string; label: string }>;
  rows?: number;
  step?: string;
  min?: string;
  max?: string;
  value?: any;
  onChange?: (value: any) => void;
  onBlur?: () => void;
  onFocus?: () => void;
  isValid?: boolean;
  isInvalid?: boolean;
  isValidating?: boolean;
  showValidationIcon?: boolean;
}

const FormFieldComponent = forwardRef<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement, FormFieldProps>(
  (
    {
      name,
      label,
      type = 'text',
      placeholder,
      required = false,
      disabled = false,
      error,
      helperText,
      className = '',
      children,
      options,
      rows = 4,
      step,
      min,
      max,
      value,
      onChange,
      onBlur,
      onFocus,
      isValid,
      isInvalid,
      isValidating,
      showValidationIcon = true,
      ...props
    },
    ref
  ) => {
    const [focused, setFocused] = useState(false);

    const handleFocus = (e: React.FocusEvent) => {
      setFocused(true);
      onFocus?.();
    };

    const handleBlur = (e: React.FocusEvent) => {
      setFocused(false);
      onBlur?.();
    };

    const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
      const newValue = type === 'number' ? 
        (e.target.value === '' ? '' : Number(e.target.value)) : 
        e.target.value;
      onChange?.(newValue);
    };

    const getInputClasses = () => {
      const baseClasses = 'w-full px-3 py-2 border rounded-lg transition-colors focus:outline-none focus:ring-2 focus:border-transparent';
      const stateClasses = disabled
        ? 'bg-gray-100 border-gray-300 cursor-not-allowed'
        : isValid
        ? 'border-green-500 focus:ring-green-500 bg-green-50'
        : isInvalid
        ? 'border-red-500 focus:ring-red-500 bg-red-50'
        : focused
        ? 'border-primary-500 focus:ring-primary-500'
        : 'border-gray-300 hover:border-gray-400';
      
      return `${baseClasses} ${stateClasses} ${className}`;
    };

    const renderInput = () => {
      const commonProps = {
        id: name,
        name,
        placeholder,
        disabled,
        value: value || '',
        onChange: handleChange,
        onFocus: handleFocus,
        onBlur: handleBlur,
        className: getInputClasses(),
        ...props
      };

      if (type === 'textarea') {
        return (
          <textarea
            ref={ref as React.RefObject<HTMLTextAreaElement>}
            rows={rows}
            {...commonProps}
          />
        );
      }

      if (type === 'select') {
        return (
          <select
            ref={ref as React.RefObject<HTMLSelectElement>}
            {...commonProps}
          >
            <option value="">{placeholder || 'Select an option'}</option>
            {options?.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        );
      }

      return (
        <input
          ref={ref as React.RefObject<HTMLInputElement>}
          type={type}
          step={step}
          min={min}
          max={max}
          {...commonProps}
        />
      );
    };

    const renderValidationIcon = () => {
      if (!showValidationIcon) return null;
      
      if (isValidating) {
        return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
      }
      
      if (isValid) {
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      }
      
      if (isInvalid) {
        return <AlertCircle className="h-4 w-4 text-red-500" />;
      }
      
      return null;
    };

    return (
      <div className="space-y-1">
        {label && (
          <label 
            htmlFor={name} 
            className="block text-sm font-medium text-gray-700"
          >
            {label}
            {required && <span className="text-red-500 ml-1">*</span>}
          </label>
        )}
        
        <div className="relative">
          {renderInput()}
          {showValidationIcon && (
            <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
              {renderValidationIcon()}
            </div>
          )}
        </div>

        {(error || helperText) && (
          <div className="space-y-1">
            {error && (
              <p className="text-sm text-red-600 flex items-center">
                <AlertCircle className="h-4 w-4 mr-1 flex-shrink-0" />
                {error}
              </p>
            )}
            {helperText && !error && (
              <p className="text-sm text-gray-500">{helperText}</p>
            )}
          </div>
        )}
      </div>
    );
  }
);

FormFieldComponent.displayName = 'FormField';

export const FormField = FormFieldComponent;
