'use client';

import React, { useState, useCallback } from 'react';
import { 
  Plus, 
  Trash2, 
  Copy, 
  Download, 
  Upload, 
  Settings, 
  CheckCircle, 
  XCircle, 
  AlertTriangle,
  GripVertical,
  Play,
  Save,
  FileText,
  Layers
} from 'lucide-react';
import { useInvariantStore } from '@/store/invariantStore';
import { RuleCondition, ComparisonOperator, SorobanVariable } from '@/types/invariant';
import { SOROBAN_VARIABLES, DEFI_TEMPLATES } from '@/data/invariants';

// Components
import VariableSelector from './VariableSelector';
import OperatorSelector from './OperatorSelector';
import ValueInput from './ValueInput';
import ConditionBlock from './ConditionBlock';
import TemplateSelector from './TemplateSelector';
import ConfigPanel from './ConfigPanel';
import ValidationPanel from './ValidationPanel';

const InvariantRuleBuilder: React.FC = () => {
  const {
    builderState,
    currentProject,
    selectedTemplate,
    isConfigPanelOpen,
    validationResult,
    isValidating,
    addCondition,
    updateCondition,
    removeCondition,
    moveCondition,
    setLogicOperator,
    clearBuilder,
    loadTemplate,
    setSelectedTemplate,
    validateRule,
    generateConfig,
    setIsConfigPanelOpen,
    setDraggedItem,
    setDraggedOverIndex,
    addRule
  } = useInvariantStore();

  const [ruleName, setRuleName] = useState('');
  const [ruleDescription, setRuleDescription] = useState('');
  const [draggedCondition, setDraggedCondition] = useState<number | null>(null);

  // Handle drag and drop
  const handleDragStart = (e: React.DragEvent, index: number) => {
    setDraggedCondition(index);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDraggedOverIndex(index);
  };

  const handleDrop = (e: React.DragEvent, dropIndex: number) => {
    e.preventDefault();
    if (draggedCondition !== null && draggedCondition !== dropIndex) {
      moveCondition(draggedCondition, dropIndex);
    }
    setDraggedCondition(null);
    setDraggedOverIndex(null);
  };

  const handleDragEnd = () => {
    setDraggedCondition(null);
    setDraggedOverIndex(null);
  };

  // Add new condition
  const handleAddCondition = useCallback(() => {
    const newCondition: RuleCondition = {
      id: Math.random().toString(36).substr(2, 9),
      variable: SOROBAN_VARIABLES[0], // Default to first variable
      operator: 'equals',
      value: '',
      valueType: 'string'
    };
    addCondition(newCondition);
  }, [addCondition]);

  // Update condition
  const handleUpdateCondition = useCallback((index: number, updates: Partial<RuleCondition>) => {
    const currentCondition = builderState.conditions[index];
    if (currentCondition) {
      const updatedCondition = { ...currentCondition, ...updates };
      updateCondition(index, updatedCondition);
    }
  }, [builderState.conditions, updateCondition]);

  // Remove condition
  const handleRemoveCondition = useCallback((index: number) => {
    removeCondition(index);
  }, [removeCondition]);

  // Validate current rule
  const handleValidateRule = useCallback(async () => {
    if (!ruleName.trim()) {
      return;
    }

    const rule = {
      id: 'temp',
      name: ruleName,
      description: ruleDescription,
      category: 'custom' as const,
      conditions: builderState.conditions,
      logicOperator: builderState.logicOperator,
      isActive: true,
      createdAt: new Date(),
      updatedAt: new Date()
    };

    await validateRule(rule);
  }, [ruleName, ruleDescription, builderState, validateRule]);

  // Save rule to project
  const handleSaveRule = useCallback(() => {
    if (!ruleName.trim() || !currentProject) {
      return;
    }

    if (validationResult && !validationResult.isValid) {
      return;
    }

    const rule = {
      name: ruleName,
      description: ruleDescription,
      category: 'custom' as const,
      conditions: builderState.conditions,
      logicOperator: builderState.logicOperator,
      isActive: true
    };

    addRule(rule);
    
    // Reset form
    setRuleName('');
    setRuleDescription('');
    clearBuilder();
  }, [ruleName, ruleDescription, builderState, currentProject, validationResult, addRule, clearBuilder]);

  // Load template
  const handleLoadTemplate = useCallback((templateId: string) => {
    loadTemplate(templateId);
    const template = DEFI_TEMPLATES.find(t => t.id === templateId);
    if (template) {
      setRuleName(template.name);
      setRuleDescription(template.description);
    }
  }, [loadTemplate]);

  // Export configuration
  const handleExportConfig = useCallback((format: 'json' | 'yaml') => {
    const config = generateConfig(format);
    const blob = new Blob([config], { type: format === 'json' ? 'application/json' : 'text/yaml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `invariant-rule.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [generateConfig]);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center space-x-3">
              <Layers className="h-6 w-6 text-primary-600" />
              <h1 className="text-2xl font-bold text-gray-900">
                Invariant Rule Builder
              </h1>
            </div>
            <div className="flex items-center space-x-2">
              <button
                onClick={() => handleExportConfig('json')}
                className="btn-secondary flex items-center space-x-2"
              >
                <Download className="h-4 w-4" />
                <span>Export JSON</span>
              </button>
              <button
                onClick={() => handleExportConfig('yaml')}
                className="btn-secondary flex items-center space-x-2"
              >
                <FileText className="h-4 w-4" />
                <span>Export YAML</span>
              </button>
              <button
                onClick={() => setIsConfigPanelOpen(true)}
                className="btn-secondary flex items-center space-x-2"
              >
                <Settings className="h-4 w-4" />
                <span>Config</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Left Panel - Rule Builder */}
          <div className="lg:col-span-2 space-y-6">
            {/* Rule Details */}
            <div className="card">
              <div className="p-6">
                <h2 className="text-lg font-semibold text-gray-900 mb-4">
                  Rule Details
                </h2>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Rule Name
                    </label>
                    <input
                      type="text"
                      value={ruleName}
                      onChange={(e) => setRuleName(e.target.value)}
                      placeholder="Enter rule name..."
                      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Description
                    </label>
                    <textarea
                      value={ruleDescription}
                      onChange={(e) => setRuleDescription(e.target.value)}
                      placeholder="Describe what this rule checks for..."
                      rows={3}
                      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                  </div>
                </div>
              </div>
            </div>

            {/* Template Selector */}
            <div className="card">
              <div className="p-6">
                <h2 className="text-lg font-semibold text-gray-900 mb-4">
                  Start with Template
                </h2>
                <TemplateSelector
                  selectedTemplate={selectedTemplate}
                  onTemplateSelect={handleLoadTemplate}
                />
              </div>
            </div>

            {/* Rule Builder */}
            <div className="card">
              <div className="p-6">
                <div className="flex justify-between items-center mb-4">
                  <h2 className="text-lg font-semibold text-gray-900">
                    Rule Conditions
                  </h2>
                  <div className="flex items-center space-x-2">
                    <select
                      value={builderState.logicOperator}
                      onChange={(e) => setLogicOperator(e.target.value as 'AND' | 'OR')}
                      className="px-3 py-1 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                    >
                      <option value="AND">AND</option>
                      <option value="OR">OR</option>
                    </select>
                    <button
                      onClick={handleAddCondition}
                      className="btn-primary flex items-center space-x-1"
                    >
                      <Plus className="h-4 w-4" />
                      <span>Add Condition</span>
                    </button>
                  </div>
                </div>

                {/* Conditions */}
                <div className="space-y-3">
                  {builderState.conditions.length === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                      <Layers className="h-12 w-12 mx-auto mb-3 text-gray-400" />
                      <p>No conditions added yet</p>
                      <p className="text-sm">Click "Add Condition" to start building your rule</p>
                    </div>
                  ) : (
                    builderState.conditions.map((condition, index) => (
                      <div
                        key={condition.id}
                        draggable
                        onDragStart={(e) => handleDragStart(e, index)}
                        onDragOver={(e) => handleDragOver(e, index)}
                        onDrop={(e) => handleDrop(e, index)}
                        onDragEnd={handleDragEnd}
                        className={`relative ${
                          draggedOverIndex === index ? 'border-2 border-primary-500' : ''
                        }`}
                      >
                        <ConditionBlock
                          condition={condition}
                          index={index}
                          onUpdate={handleUpdateCondition}
                          onRemove={handleRemoveCondition}
                          isDraggable={builderState.conditions.length > 1}
                        />
                        {index < builderState.conditions.length - 1 && (
                          <div className="flex justify-center my-2">
                            <span className="px-3 py-1 bg-gray-100 text-gray-600 text-sm rounded-full">
                              {builderState.logicOperator}
                            </span>
                          </div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="card">
              <div className="p-6">
                <div className="flex justify-between items-center">
                  <div className="flex space-x-2">
                    <button
                      onClick={handleValidateRule}
                      disabled={!ruleName.trim() || builderState.conditions.length === 0 || isValidating}
                      className="btn-secondary flex items-center space-x-2"
                    >
                      {isValidating ? (
                        <div className="animate-spin h-4 w-4 border-2 border-primary-600 border-t-transparent rounded-full" />
                      ) : (
                        <Play className="h-4 w-4" />
                      )}
                      <span>{isValidating ? 'Validating...' : 'Validate Rule'}</span>
                    </button>
                    <button
                      onClick={() => clearBuilder()}
                      className="btn-secondary flex items-center space-x-2"
                    >
                      <Trash2 className="h-4 w-4" />
                      <span>Clear</span>
                    </button>
                  </div>
                  <button
                    onClick={handleSaveRule}
                    disabled={!ruleName.trim() || !currentProject || builderState.conditions.length === 0}
                    className="btn-primary flex items-center space-x-2"
                  >
                    <Save className="h-4 w-4" />
                    <span>Save Rule</span>
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Right Panel */}
          <div className="space-y-6">
            {/* Variable Library */}
            <div className="card">
              <div className="p-6">
                <h3 className="text-lg font-semibold text-gray-900 mb-4">
                  Variable Library
                </h3>
                <div className="space-y-3">
                  {['balance', 'token', 'contract', 'custom'].map((category) => (
                    <div key={category} className="border border-gray-200 rounded-lg p-3">
                      <h4 className="font-medium text-gray-900 capitalize mb-2">
                        {category} Variables
                      </h4>
                      <div className="space-y-1">
                        {SOROBAN_VARIABLES
                          .filter(v => v.category === category)
                          .map((variable) => (
                            <div
                              key={variable.id}
                              className="text-sm text-gray-600 p-2 hover:bg-gray-50 rounded cursor-pointer"
                              draggable
                              onDragStart={(e) => {
                                setDraggedItem({ type: 'variable', data: variable });
                              }}
                            >
                              <div className="font-medium text-gray-900">
                                {variable.name}
                              </div>
                              <div className="text-xs text-gray-500">
                                {variable.description}
                              </div>
                            </div>
                          ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Validation Results */}
            {validationResult && (
              <ValidationPanel result={validationResult} />
            )}
          </div>
        </div>
      </div>

      {/* Config Panel Modal */}
      {isConfigPanelOpen && (
        <ConfigPanel
          onClose={() => setIsConfigPanelOpen(false)}
          onExport={handleExportConfig}
        />
      )}
    </div>
  );
};

export default InvariantRuleBuilder;
