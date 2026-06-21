'use client';

import React, { ReactNode } from 'react';
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
}

export function Form<T extends Record<string, any>>({
  config,
  onSubmit,
  children,
  options,
  className = '',
  noValidate = false,
}: FormProps<T>) {
  const formValidation = useFormValidation<T>(config, options);
  const { handleSubmit } = formValidation;

  const onFormSubmit = handleSubmit(onSubmit);

  return (
    <form onSubmit={onFormSubmit} noValidate={noValidate} className={className}>
      {children(formValidation)}
    </form>
  );
}
