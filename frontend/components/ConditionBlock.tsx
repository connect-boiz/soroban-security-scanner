'use client';

import React from 'react';
import { GripVertical, Trash2, Edit } from 'lucide-react';
import { RuleCondition, SorobanVariable } from '@/types/invariant';
import VariableSelector from './VariableSelector';
import OperatorSelector from './OperatorSelector';
import ValueInput from './ValueInput';

interface ConditionBlockProps {
  condition: RuleCondition;
  index: number;
  onUpdate: (index: number, updates: Partial<RuleCondition>) => void;
  onRemove: (index: number) => void;
  isDraggable?: boolean;
}

const ConditionBlock: React.FC<ConditionBlockProps> = ({
  condition,
  index,
  onUpdate,
  onRemove,
  isDraggable = true
}) => {
  const handleVariableChange = (variable: SorobanVariable) => {
    onUpdate(index, { 
      variable,
      valueType: variable.type,
      value: variable.type === 'boolean' ? 'true' : ''
    });
  };

  const handleOperatorChange = (operator: string) => {
    onUpdate(index, { operator: operator as any });
  };

  const handleValueChange = (value: string) => {
    onUpdate(index, { value });
  };

  return (
    <div className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start space-x-3">
        {/* Drag Handle */}
        {isDraggable && (
          <div className="flex items-center justify-center mt-1">
            <GripVertical className="h-5 w-5 text-gray-400 cursor-move" />
          </div>
        )}

        {/* Condition Content */}
        <div className="flex-1">
          <div className="flex items-center space-x-2 mb-3">
            <span className="text-sm font-medium text-gray-500">Condition {index + 1}</span>
            <div className="h-px bg-gray-300 flex-1" />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {/* Variable Selector */}
            <div>
              <label className="block text-xs font-medium text-gray-700 mb-1">
                Variable
              </label>
              <VariableSelector
                selectedVariable={condition.variable}
                onVariableChange={handleVariableChange}
              />
            </div>

            {/* Operator Selector */}
            <div>
              <label className="block text-xs font-medium text-gray-700 mb-1">
                Operator
              </label>
              <OperatorSelector
                selectedOperator={condition.operator}
                variableType={condition.variable.type}
                onOperatorChange={handleOperatorChange}
              />
            </div>

            {/* Value Input */}
            <div>
              <label className="block text-xs font-medium text-gray-700 mb-1">
                Value
              </label>
              <ValueInput
                value={condition.value}
                valueType={condition.valueType}
                variable={condition.variable}
                onValueChange={handleValueChange}
              />
            </div>
          </div>

          {/* Variable Description */}
          {condition.variable.description && (
            <div className="mt-3 p-2 bg-gray-50 rounded text-xs text-gray-600">
              <strong>Variable:</strong> {condition.variable.description}
            </div>
          )}

          {/* Examples */}
          {condition.variable.examples.length > 0 && (
            <div className="mt-2">
              <span className="text-xs text-gray-500">Examples: </span>
              <span className="text-xs text-gray-600">
                {condition.variable.examples.slice(0, 3).join(', ')}
                {condition.variable.examples.length > 3 && '...'}
              </span>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center space-x-1">
          <button
            onClick={() => onRemove(index)}
            className="p-1 text-gray-400 hover:text-red-600 transition-colors"
            title="Remove condition"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
};

export default ConditionBlock;
