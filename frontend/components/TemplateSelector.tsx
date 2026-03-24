'use client';

import React from 'react';
import { Copy, BookOpen, Zap, Shield, DollarSign } from 'lucide-react';
import { InvariantTemplate } from '@/types/invariant';
import { DEFI_TEMPLATES } from '@/data/invariants';

interface TemplateSelectorProps {
  selectedTemplate: string | null;
  onTemplateSelect: (templateId: string) => void;
}

const TemplateSelector: React.FC<TemplateSelectorProps> = ({
  selectedTemplate,
  onTemplateSelect
}) => {
  const [selectedCategory, setSelectedCategory] = React.useState<string>('all');

  // Get unique categories
  const categories = React.useMemo(() => {
    const cats = ['all', ...new Set(DEFI_TEMPLATES.map(t => t.category))];
    return cats;
  }, []);

  // Filter templates by category
  const filteredTemplates = React.useMemo(() => {
    if (selectedCategory === 'all') {
      return DEFI_TEMPLATES;
    }
    return DEFI_TEMPLATES.filter(t => t.category === selectedCategory);
  }, [selectedCategory]);

  // Get category icon
  const getCategoryIcon = (category: string) => {
    switch (category.toLowerCase()) {
      case 'token economics': return <DollarSign className="h-4 w-4" />;
      case 'balance safety': return <Shield className="h-4 w-4" />;
      case 'token safety': return <Zap className="h-4 w-4" />;
      case 'access control': return <BookOpen className="h-4 w-4" />;
      case 'emergency controls': return <Shield className="h-4 w-4" />;
      case 'defi safety': return <DollarSign className="h-4 w-4" />;
      default: return <Copy className="h-4 w-4" />;
    }
  };

  // Get category color
  const getCategoryColor = (category: string) => {
    switch (category.toLowerCase()) {
      case 'token economics': return 'bg-green-100 text-green-800 border-green-200';
      case 'balance safety': return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'token safety': return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'access control': return 'bg-purple-100 text-purple-800 border-purple-200';
      case 'emergency controls': return 'bg-red-100 text-red-800 border-red-200';
      case 'defi safety': return 'bg-indigo-100 text-indigo-800 border-indigo-200';
      default: return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const handleTemplateSelect = (templateId: string) => {
    onTemplateSelect(templateId);
  };

  return (
    <div>
      {/* Category Filter */}
      <div className="mb-4">
        <div className="flex flex-wrap gap-2">
          {categories.map((category) => (
            <button
              key={category}
              onClick={() => setSelectedCategory(category)}
              className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
                selectedCategory === category
                  ? 'bg-primary-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {category === 'all' ? 'All Templates' : category}
            </button>
          ))}
        </div>
      </div>

      {/* Template Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {filteredTemplates.map((template) => (
          <div
            key={template.id}
            onClick={() => handleTemplateSelect(template.id)}
            className={`border rounded-lg p-4 cursor-pointer transition-all hover:shadow-md ${
              selectedTemplate === template.id
                ? 'border-primary-500 bg-primary-50'
                : 'border-gray-200 bg-white hover:border-gray-300'
            }`}
          >
            {/* Header */}
            <div className="flex items-start justify-between mb-3">
              <div className="flex items-center space-x-2">
                {getCategoryIcon(template.category)}
                <h3 className="font-semibold text-gray-900 text-sm">
                  {template.name}
                </h3>
              </div>
              {selectedTemplate === template.id && (
                <div className="w-2 h-2 bg-primary-600 rounded-full" />
              )}
            </div>

            {/* Category Badge */}
            <div className="mb-3">
              <span className={`inline-block px-2 py-1 text-xs font-medium rounded-full border ${getCategoryColor(template.category)}`}>
                {template.category}
              </span>
            </div>

            {/* Description */}
            <p className="text-sm text-gray-600 mb-3">
              {template.description}
            </p>

            {/* Use Case */}
            <div className="mb-3">
              <span className="text-xs font-medium text-gray-700">Use Case:</span>
              <p className="text-xs text-gray-600 mt-1">
                {template.useCase}
              </p>
            </div>

            {/* Conditions Preview */}
            <div className="border-t border-gray-100 pt-3">
              <div className="text-xs font-medium text-gray-700 mb-2">
                Conditions ({template.conditions.length}):
              </div>
              <div className="space-y-1">
                {template.conditions.slice(0, 2).map((condition, index) => (
                  <div key={index} className="text-xs text-gray-600 font-mono">
                    {condition.variable.name} {condition.operator} {condition.value}
                    {index < Math.min(template.conditions.length - 1, 1) && (
                      <span className="mx-1 text-gray-400">{template.logicOperator}</span>
                    )}
                  </div>
                ))}
                {template.conditions.length > 2 && (
                  <div className="text-xs text-gray-400 italic">
                    ...and {template.conditions.length - 2} more
                  </div>
                )}
              </div>
            </div>

            {/* Logic Operator */}
            <div className="mt-3 flex items-center justify-between">
              <span className="text-xs text-gray-500">
                Logic: <span className="font-mono font-medium">{template.logicOperator}</span>
              </span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleTemplateSelect(template.id);
                }}
                className="text-xs text-primary-600 hover:text-primary-700 font-medium"
              >
                Use Template
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* No Templates Found */}
      {filteredTemplates.length === 0 && (
        <div className="text-center py-8 text-gray-500">
          <Copy className="h-12 w-12 mx-auto mb-3 text-gray-400" />
          <p className="text-sm">No templates found in this category</p>
          <button
            onClick={() => setSelectedCategory('all')}
            className="mt-2 text-sm text-primary-600 hover:text-primary-700"
          >
            View all templates
          </button>
        </div>
      )}

      {/* Template Count */}
      <div className="mt-4 text-xs text-gray-500 text-center">
        Showing {filteredTemplates.length} of {DEFI_TEMPLATES.length} templates
      </div>
    </div>
  );
};

export default TemplateSelector;
