'use client';

import React from 'react';
import { ChevronDown } from 'lucide-react';
import { SorobanVariable } from '@/types/invariant';
import { getVariablesByCategory } from '@/data/invariants';

interface VariableSelectorProps {
  selectedVariable: SorobanVariable;
  onVariableChange: (variable: SorobanVariable) => void;
}

const VariableSelector: React.FC<VariableSelectorProps> = ({
  selectedVariable,
  onVariableChange
}) => {
  const [isOpen, setIsOpen] = React.useState(false);
  const [searchTerm, setSearchTerm] = React.useState('');
  
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

  // Get all variables grouped by category
  const variablesByCategory = React.useMemo(() => {
    const categories = ['balance', 'token', 'contract', 'custom'];
    return categories.map(category => ({
      category,
      variables: getVariablesByCategory(category)
    }));
  }, []);

  // Filter variables based on search
  const filteredVariables = React.useMemo(() => {
    if (!searchTerm) return variablesByCategory;
    
    return variablesByCategory.map(({ category, variables }) => ({
      category,
      variables: variables.filter(variable =>
        variable.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        variable.description.toLowerCase().includes(searchTerm.toLowerCase())
      )
    })).filter(({ variables }) => variables.length > 0);
  }, [variablesByCategory, searchTerm]);

  const handleVariableSelect = (variable: SorobanVariable) => {
    onVariableChange(variable);
    setIsOpen(false);
    setSearchTerm('');
  };

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'balance': return 'bg-blue-100 text-blue-800';
      case 'token': return 'bg-green-100 text-green-800';
      case 'contract': return 'bg-purple-100 text-purple-800';
      case 'custom': return 'bg-gray-100 text-gray-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'number': return '🔢';
      case 'string': return '📝';
      case 'address': return '📍';
      case 'boolean': return '☑️';
      default: return '❓';
    }
  };

  return (
    <div className="relative" ref={dropdownRef}>
      {/* Trigger */}
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-3 py-2 text-left border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 flex items-center justify-between"
      >
        <div className="flex items-center space-x-2">
          <span>{getTypeIcon(selectedVariable.type)}</span>
          <span className="text-sm text-gray-900">{selectedVariable.name}</span>
        </div>
        <ChevronDown className={`h-4 w-4 text-gray-400 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div className="absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-80 overflow-hidden">
          {/* Search */}
          <div className="p-3 border-b border-gray-200">
            <input
              type="text"
              placeholder="Search variables..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              autoFocus
            />
          </div>

          {/* Variable List */}
          <div className="overflow-y-auto max-h-60">
            {filteredVariables.length === 0 ? (
              <div className="p-4 text-center text-gray-500 text-sm">
                No variables found
              </div>
            ) : (
              filteredVariables.map(({ category, variables }) => (
                <div key={category} className="border-b border-gray-100 last:border-b-0">
                  {/* Category Header */}
                  <div className="px-3 py-2 bg-gray-50 border-b border-gray-200">
                    <span className={`inline-block px-2 py-1 text-xs font-medium rounded-full ${getCategoryColor(category)}`}>
                      {category.charAt(0).toUpperCase() + category.slice(1)}
                    </span>
                  </div>

                  {/* Variables in Category */}
                  {variables.map((variable) => (
                    <button
                      key={variable.id}
                      type="button"
                      onClick={() => handleVariableSelect(variable)}
                      className="w-full px-3 py-2 text-left hover:bg-gray-50 focus:bg-gray-50 focus:outline-none border-b border-gray-100 last:border-b-0"
                    >
                      <div className="flex items-start space-x-2">
                        <span className="text-sm mt-0.5">{getTypeIcon(variable.type)}</span>
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium text-gray-900 truncate">
                            {variable.name}
                          </div>
                          <div className="text-xs text-gray-500 truncate">
                            {variable.description}
                          </div>
                          {variable.examples.length > 0 && (
                            <div className="text-xs text-gray-400 mt-1">
                              Examples: {variable.examples.slice(0, 2).join(', ')}
                              {variable.examples.length > 2 && '...'}
                            </div>
                          )}
                        </div>
                      </div>
                    </button>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default VariableSelector;
