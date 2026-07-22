'use client';

import React, { ReactNode, useEffect, useRef, useState } from 'react';
import { useFormValidation } from '@/hooks/useFormValidation';
import { FormConfig } from '@/utils/validation';
import { UseFormValidationOptions } from '@/hooks/useFormValidation';

export interface FormProps<T extends Record<string, any>> {
  config: FormConfig;
  onSubmit: (data: T) => void | Promise<void>;
  children: (props: {
    formData: T;
    errors: Record<string, string>;
    touched: Record<string, boolean>;
    isValidating: Record<string, boolean>;
    isValid: boolean;
    isDirty: boolean;
    isSubmitting: boolean;
    setFieldValue: (field: keyof T, value: any) => void;
    setFieldTouched: (field: keyof T, touched?: boolean) => void;
    validateField: (field: keyof T) => Promise<void>;
    resetForm: () => void;
    getFieldError: (field: keyof T) => string | undefined;
    isFieldValid: (field: keyof T) => boolean;
    isFieldInvalid: (field: keyof T) => boolean;
    isFieldTouched: (field: keyof T) => boolean;
    isFieldValidating: (field: keyof T) => boolean;
  }) => ReactNode;
  options?: UseFormValidationOptions;
  className?: string;
  noValidate?: boolean;
  /** Optional callback when form submission succeeds */
  onSuccessMessage?: string;
}

export function Form<T extends Record<string, any>>({
  config,
  onSubmit,
  children,
  options,
  className = '',
  noValidate = false,
  onSuccessMessage,
}: FormProps<T>) {
  const formValidation = useFormValidation<T>(config, options);
  const { handleSubmit, errors } = formValidation;
  const [announcement, setAnnouncement] = useState('');
  const prevErrorCountRef = useRef(0);

  // Announce error summary changes to screen readers
  const errorsKey = JSON.stringify(errors);
  useEffect(() => {
    const errorEntries = Object.entries(errors).filter(([, msg]) => !!msg);
    const currentErrorCount = errorEntries.length;

    if (currentErrorCount > prevErrorCountRef.current) {
      const fieldNames = errorEntries.map(([field]) => field.replace(/([A-Z])/g, ' $1').trim());
      setAnnouncement(
        `Form has ${currentErrorCount} error${currentErrorCount > 1 ? 's' : ''}: ${fieldNames.join(', ')}`
      );
    }

    prevErrorCountRef.current = currentErrorCount;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [errorsKey]);

  const wrappedOnSubmit = async (data: T) => {
    await onSubmit(data);
    if (onSuccessMessage) {
      setAnnouncement(onSuccessMessage);
    }
  };

  const onFormSubmit = handleSubmit(wrappedOnSubmit);

  return (
    <>
      {/* Visually hidden live region for screen reader announcements */}
      <div
        id="form-announcements"
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {announcement}
      </div>
      <form onSubmit={onFormSubmit} noValidate={noValidate} className={className}>
        {children(formValidation)}
      </form>
    </>
  );
}
