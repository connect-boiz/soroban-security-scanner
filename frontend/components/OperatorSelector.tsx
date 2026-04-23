'use client';

import React from 'react';
import { ChevronDown } from 'lucide-react';
import { ComparisonOperator } from '@/types/invariant';

interface OperatorSelectorProps {
  selectedOperator: ComparisonOperator;
  variableType: 'string' | 'number' | 'address' | 'boolean';
  onOperatorChange: (operator: string) => void;
}

const OperatorSelector: React.FC<OperatorSelectorProps> = ({
  selectedOperator,
  variableType,
  onOperatorChange
}) => {
  const [isOpen, setIsOpen] = React.useState(false);
  const dropdownRef = React.useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Get available operators based on variable type
  const getAvailableOperators = (): ComparisonOperator[] => {
    const commonOperators: ComparisonOperator[] = ['equals', 'not_equals'];
    
    switch (variableType) {
      case 'number':
        return [
          ...commonOperators,
          'greater_than',
          'less_than',
          'greater_than_or_equal',
          'less_than_or_equal'
        ];
      
      case 'string':
        return [
          ...commonOperators,
          'contains',
          'not_contains',
          'is_empty',
          'is_not_empty'
        ];
      
      case 'address':
        return commonOperators;
      
      case 'boolean':
        return commonOperators;
      
      default:
        return commonOperators;
    }
  };

  const getOperatorDisplay = (operator: ComparisonOperator): string => {
    switch (operator) {
      case 'equals': return '==';
      case 'not_equals': return '!=';
      case 'greater_than': return '>';
      case 'less_than': return '<';
      case 'greater_than_or_equal': return '>=';
      case 'less_than_or_equal': return '<=';
      case 'contains': return 'contains';
      case 'not_contains': return 'not contains';
      case 'is_empty': return 'is empty';
      case 'is_not_empty': return 'is not empty';
      default: return operator;
    }
  };

  const getOperatorDescription = (operator: ComparisonOperator): string => {
    switch (operator) {
      case 'equals': return 'Exactly equal to';
      case 'not_equals': return 'Not equal to';
      case 'greater_than': return 'Greater than';
      case 'less_than': return 'Less than';
      case 'greater_than_or_equal': return 'Greater than or equal to';
      case 'less_than_or_equal': return 'Less than or equal to';
      case 'contains': return 'Contains the substring';
      case 'not_contains': return 'Does not contain the substring';
      case 'is_empty': return 'Is empty or null';
      case 'is_not_empty': return 'Is not empty';
      default: return '';
    }
  };

  const availableOperators = getAvailableOperators();

  const handleOperatorSelect = (operator: ComparisonOperator) => {
    onOperatorChange(operator);
    setIsOpen(false);
  };

  return (
    <div className="relative" ref={dropdownRef}>
      {/* Trigger */}
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-3 py-2 text-left border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 flex items-center justify-between"
      >
        <span className="text-sm text-gray-900">
          {getOperatorDisplay(selectedOperator)}
        </span>
        <ChevronDown className={`h-4 w-4 text-gray-400 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div className="absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg">
          <div className="max-h-60 overflow-y-auto">
            {availableOperators.map((operator) => (
              <button
                key={operator}
                type="button"
                onClick={() => handleOperatorSelect(operator)}
                className={`w-full px-3 py-2 text-left hover:bg-gray-50 focus:bg-gray-50 focus:outline-none border-b border-gray-100 last:border-b-0 ${
                  selectedOperator === operator ? 'bg-primary-50 text-primary-700' : 'text-gray-900'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-mono text-sm font-medium">
                        {getOperatorDisplay(operator)}
                      </span>
                    </div>
                    <div className="text-xs text-gray-500">
                      {getOperatorDescription(operator)}
                    </div>
                  </div>
                  {selectedOperator === operator && (
                    <div className="w-2 h-2 bg-primary-600 rounded-full" />
                  )}
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default OperatorSelector;
