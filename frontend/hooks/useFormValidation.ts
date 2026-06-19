'use client';

import { useState, useCallback, useEffect, useRef } from 'react';
import { FormValidator, FormConfig, ValidationResult, ValidationState } from '@/utils/validation';

export interface UseFormValidationOptions {
  validateOnChange?: boolean;
  validateOnBlur?: boolean;
  validateOnSubmit?: boolean;
  initialData?: Record<string, any>;
}

interface UseFormValidationReturn<T extends Record<string, any>> {
  formData: T;
  errors: Record<string, string>;
  touched: Record<string, boolean>;
  isValidating: Record<string, boolean>;
  isValid: boolean;
  isDirty: boolean;
  isSubmitting: boolean;

  // Field methods
  setFieldValue: (field: keyof T, value: any) => void;
  setFieldTouched: (field: keyof T, touched?: boolean) => void;
  validateField: (field: keyof T) => Promise<void>;

  // Form methods
  setFormData: (data: Partial<T>) => void;
  resetForm: () => void;
  validateForm: () => Promise<ValidationResult>;
  handleSubmit: (
    onSubmit: (data: T) => void | Promise<void>
  ) => (e?: React.FormEvent) => Promise<void>;

  // Utility methods
  getFieldError: (field: keyof T) => string | undefined;
  isFieldValid: (field: keyof T) => boolean;
  isFieldInvalid: (field: keyof T) => boolean;
  isFieldTouched: (field: keyof T) => boolean;
  isFieldValidating: (field: keyof T) => boolean;
}

export function useFormValidation<T extends Record<string, any>>(
  config: FormConfig,
  options: UseFormValidationOptions = {}
): UseFormValidationReturn<T> {
  const {
    validateOnChange = true,
    validateOnBlur = true,
    validateOnSubmit = true,
    initialData = {} as T,
  } = options;

  const validatorRef = useRef(new FormValidator(config));

  const [formData, setFormData] = useState<T>(initialData as T);
  const [validationState, setValidationState] = useState<ValidationState>({
    errors: {},
    touched: {},
    isValidating: {},
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isValid, setIsValid] = useState(false);
  const [isDirty, setIsDirty] = useState(false);

  // Calculate derived state
  const errors = validationState.errors;
  const touched = validationState.touched;
  const isValidating = validationState.isValidating;

  // Validate single field
  const validateField = useCallback(
    async (field: keyof T) => {
      const fieldName = field as string;

      setValidationState(prev => ({
        ...prev,
        isValidating: { ...prev.isValidating, [fieldName]: true },
      }));

      try {
        const fieldErrors = await validatorRef.current.validateField(
          fieldName,
          formData[fieldName],
          formData
        );

        setValidationState(prev => ({
          ...prev,
          errors: {
            ...prev.errors,
            [fieldName]: fieldErrors,
          },
          isValidating: { ...prev.isValidating, [fieldName]: false },
        }));
      } catch (error) {
        console.error(`Field validation failed for ${fieldName}:`, error);
        setValidationState(prev => ({
          ...prev,
          errors: {
            ...prev.errors,
            [fieldName]: ['Validation failed'],
          },
          isValidating: { ...prev.isValidating, [fieldName]: false },
        }));
      }
    },
    [formData]
  );

  // Validate entire form
  const validateForm = useCallback(async (): Promise<ValidationResult> => {
    const result = await validatorRef.current.validateForm(formData);

    setValidationState(prev => ({
      ...prev,
      errors: result.errors,
      touched: Object.keys(formData).reduce((acc, key) => ({ ...acc, [key]: true }), {}),
      isValidating: {},
    }));

    setIsValid(result.isValid);
    return result;
  }, [formData]);

  // Set field value
  const setFieldValue = useCallback(
    (field: keyof T, value: any) => {
      setFormData(prev => ({ ...prev, [field]: value }));
      setIsDirty(true);

      if (validateOnChange) {
        const fieldName = field as string;
        validatorRef.current.validateFieldDebounced(
          fieldName,
          value,
          { ...formData, [field]: value },
          fieldErrors => {
            setValidationState(prev => ({
              ...prev,
              errors: {
                ...prev.errors,
                [fieldName]: fieldErrors,
              },
            }));
          }
        );
      }
    },
    [formData, validateOnChange]
  );

  // Set field touched
  const setFieldTouched = useCallback(
    (field: keyof T, touched = true) => {
      const fieldName = field as string;
      setValidationState(prev => ({
        ...prev,
        touched: { ...prev.touched, [fieldName]: touched },
      }));

      if (touched && validateOnBlur) {
        validateField(field);
      }
    },
    [validateField, validateOnBlur]
  );

  // Set multiple form data
  const setFormDataValues = useCallback((data: Partial<T>) => {
    setFormData(prev => ({ ...prev, ...data }) as T);
    setIsDirty(true);
  }, []);

  // Reset form
  const resetForm = useCallback(() => {
    setFormData(initialData as T);
    setValidationState({
      errors: {},
      touched: {},
      isValidating: {},
    });
    setIsSubmitting(false);
    setIsValid(false);
    setIsDirty(false);
    validatorRef.current.clearDebounce();
  }, [initialData]);

  // Handle form submission
  const handleSubmit = useCallback(
    (onSubmit: (data: T) => void | Promise<void>) => {
      return async (e?: React.FormEvent) => {
        e?.preventDefault();

        if (isSubmitting) return;

        setIsSubmitting(true);

        let validationResult: ValidationResult;
        if (validateOnSubmit) {
          validationResult = await validateForm();
        } else {
          validationResult = { isValid: true, errors: {}, fieldErrors: {} };
        }

        if (validationResult.isValid) {
          try {
            await onSubmit(formData);
          } catch (error) {
            console.error('Form submission error:', error);
            // You could handle submission errors here
          }
        }

        setIsSubmitting(false);
      };
    },
    [formData, isSubmitting, validateForm, validateOnSubmit]
  );

  // Utility methods
  const getFieldError = useCallback(
    (field: keyof T): string | undefined => {
      const fieldName = field as string;
      const fieldErrors = errors[fieldName];
      return fieldErrors && fieldErrors.length > 0 ? fieldErrors[0] : undefined;
    },
    [errors]
  );

  const isFieldValid = useCallback(
    (field: keyof T): boolean => {
      const fieldName = field as string;
      return touched[fieldName] && (!errors[fieldName] || errors[fieldName].length === 0);
    },
    [touched, errors]
  );

  const isFieldInvalid = useCallback(
    (field: keyof T): boolean => {
      const fieldName = field as string;
      return touched[fieldName] && errors[fieldName] && errors[fieldName].length > 0;
    },
    [touched, errors]
  );

  const isFieldTouched = useCallback(
    (field: keyof T): boolean => {
      const fieldName = field as string;
      return touched[fieldName] || false;
    },
    [touched]
  );

  const isFieldValidating = useCallback(
    (field: keyof T): boolean => {
      const fieldName = field as string;
      return isValidating[fieldName] || false;
    },
    [isValidating]
  );

  // Update form validity when errors change
  useEffect(() => {
    const hasErrors = Object.values(errors).some(
      fieldErrors => fieldErrors && fieldErrors.length > 0
    );
    setIsValid(!hasErrors);
  }, [errors]);

  // Cleanup debounce timers on unmount
  useEffect(() => {
    return () => {
      validatorRef.current.clearDebounce();
    };
  }, []);

  return {
    formData,
    errors: Object.keys(errors).reduce(
      (acc, key) => ({
        ...acc,
        [key]: errors[key]?.[0] || '',
      }),
      {}
    ),
    touched,
    isValidating,
    isValid,
    isDirty,
    isSubmitting,

    setFieldValue,
    setFieldTouched,
    validateField,

    setFormData: setFormDataValues,
    resetForm,
    validateForm,
    handleSubmit,

    getFieldError,
    isFieldValid,
    isFieldInvalid,
    isFieldTouched,
    isFieldValidating,
  } as UseFormValidationReturn<T>;
}
