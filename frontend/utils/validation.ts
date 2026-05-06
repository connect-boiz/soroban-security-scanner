export interface ValidationRule {
  name: string;
  validator: (value: any, formData?: Record<string, any>) => boolean | string;
  message: string;
  debounceMs?: number;
}

export interface FieldValidation {
  rules: ValidationRule[];
  validateOnChange?: boolean;
  validateOnBlur?: boolean;
}

export interface FormConfig {
  [fieldName: string]: FieldValidation;
}

export interface ValidationResult {
  isValid: boolean;
  errors: Record<string, string[]>;
  fieldErrors: Record<string, string>;
}

export interface ValidationState {
  errors: Record<string, string[]>;
  touched: Record<string, boolean>;
  isValidating: Record<string, boolean>;
}

// Built-in validation rules
export const ValidationRules = {
  required: (message?: string): ValidationRule => ({
    name: 'required',
    validator: (value) => {
      if (typeof value === 'string') {
        return value.trim().length > 0;
      }
      if (Array.isArray(value)) {
        return value.length > 0;
      }
      return value !== null && value !== undefined;
    },
    message: message || 'This field is required'
  }),

  minLength: (min: number, message?: string): ValidationRule => ({
    name: 'minLength',
    validator: (value) => !value || typeof value !== 'string' || value.length >= min,
    message: message || `Must be at least ${min} characters`
  }),

  maxLength: (max: number, message?: string): ValidationRule => ({
    name: 'maxLength',
    validator: (value) => !value || typeof value !== 'string' || value.length <= max,
    message: message || `Must be no more than ${max} characters`
  }),

  email: (message?: string): ValidationRule => ({
    name: 'email',
    validator: (value) => {
      if (!value || typeof value !== 'string') return true;
      const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
      return emailRegex.test(value);
    },
    message: message || 'Please enter a valid email address'
  }),

  url: (message?: string): ValidationRule => ({
    name: 'url',
    validator: (value) => {
      if (!value || typeof value !== 'string') return true;
      try {
        new URL(value);
        return true;
      } catch {
        return false;
      }
    },
    message: message || 'Please enter a valid URL'
  }),

  numeric: (message?: string): ValidationRule => ({
    name: 'numeric',
    validator: (value) => {
      if (!value) return true;
      return !isNaN(Number(value)) && Number(value) >= 0;
    },
    message: message || 'Please enter a valid number'
  }),

  min: (min: number, message?: string): ValidationRule => ({
    name: 'min',
    validator: (value) => !value || Number(value) >= min,
    message: message || `Must be at least ${min}`
  }),

  max: (max: number, message?: string): ValidationRule => ({
    name: 'max',
    validator: (value) => !value || Number(value) <= max,
    message: message || `Must be no more than ${max}`
  }),

  pattern: (regex: RegExp, message?: string): ValidationRule => ({
    name: 'pattern',
    validator: (value) => !value || typeof value !== 'string' || regex.test(value),
    message: message || 'Invalid format'
  }),

  stellarPublicKey: (message?: string): ValidationRule => ({
    name: 'stellarPublicKey',
    validator: (value) => {
      if (!value || typeof value !== 'string') return true;
      // Stellar public keys are 56 characters starting with 'G'
      return /^[G][A-Za-z0-9]{55}$/.test(value);
    },
    message: message || 'Please enter a valid Stellar public key (starts with G, 56 characters)'
  }),

  xlmAmount: (message?: string): ValidationRule => ({
    name: 'xlmAmount',
    validator: (value) => {
      if (!value) return true;
      const num = Number(value);
      return !isNaN(num) && num >= 0.0000001 && num <= 1000000;
    },
    message: message || 'XLM amount must be between 0.0000001 and 1,000,000'
  }),

  custom: (validator: (value: any, formData?: Record<string, any>) => boolean | string, message: string, name?: string): ValidationRule => ({
    name: name || 'custom',
    validator,
    message
  }),

  async: (
    validator: (value: any, formData?: Record<string, any>) => Promise<boolean | string>,
    message: string,
    name?: string
  ): ValidationRule => ({
    name: name || 'async',
    validator: validator as any,
    message,
    debounceMs: 500
  })
};

export class FormValidator {
  private config: FormConfig;
  private debounceTimers: Record<string, NodeJS.Timeout> = {};

  constructor(config: FormConfig) {
    this.config = config;
  }

  async validateField(
    fieldName: string,
    value: any,
    formData: Record<string, any> = {}
  ): Promise<string[]> {
    const fieldConfig = this.config[fieldName];
    if (!fieldConfig) return [];

    const errors: string[] = [];
    
    for (const rule of fieldConfig.rules) {
      try {
        const result = await rule.validator(value, formData);
        if (result === false || typeof result === 'string') {
          errors.push(typeof result === 'string' ? result : rule.message);
        }
      } catch (error) {
        console.error(`Validation error for field ${fieldName}:`, error);
        errors.push('Validation failed');
      }
    }

    return errors;
  }

  async validateForm(formData: Record<string, any>): Promise<ValidationResult> {
    const errors: Record<string, string[]> = {};
    const fieldErrors: Record<string, string> = {};

    for (const fieldName of Object.keys(this.config)) {
      const fieldErrors = await this.validateField(fieldName, formData[fieldName], formData);
      if (fieldErrors.length > 0) {
        errors[fieldName] = fieldErrors;
        fieldErrors[fieldName] = fieldErrors[0]; // First error for display
      }
    }

    return {
      isValid: Object.keys(errors).length === 0,
      errors,
      fieldErrors
    };
  }

  validateFieldDebounced(
    fieldName: string,
    value: any,
    formData: Record<string, any>,
    callback: (errors: string[]) => void
  ): void {
    const fieldConfig = this.config[fieldName];
    const debounceMs = fieldConfig?.rules.find(r => r.debounceMs)?.debounceMs || 300;

    if (this.debounceTimers[fieldName]) {
      clearTimeout(this.debounceTimers[fieldName]);
    }

    this.debounceTimers[fieldName] = setTimeout(async () => {
      const errors = await this.validateField(fieldName, value, formData);
      callback(errors);
    }, debounceMs);
  }

  clearDebounce(fieldName?: string): void {
    if (fieldName) {
      if (this.debounceTimers[fieldName]) {
        clearTimeout(this.debounceTimers[fieldName]);
        delete this.debounceTimers[fieldName];
      }
    } else {
      Object.keys(this.debounceTimers).forEach(key => {
        clearTimeout(this.debounceTimers[key]);
      });
      this.debounceTimers = {};
    }
  }
}
