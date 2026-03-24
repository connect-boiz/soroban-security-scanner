// Invariant Rule Builder Types

export interface InvariantRule {
  id: string;
  name: string;
  description: string;
  category: 'token' | 'balance' | 'contract' | 'custom';
  conditions: RuleCondition[];
  logicOperator: 'AND' | 'OR';
  isActive: boolean;
  createdAt: Date;
  updatedAt: Date;
}

export interface RuleCondition {
  id: string;
  variable: SorobanVariable;
  operator: ComparisonOperator;
  value: string | number;
  valueType: 'string' | 'number' | 'address' | 'boolean';
}

export interface SorobanVariable {
  id: string;
  name: string;
  category: 'balance' | 'token' | 'contract' | 'custom';
  type: 'string' | 'number' | 'address' | 'boolean';
  description: string;
  examples: string[];
}

export interface InvariantTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  conditions: Omit<RuleCondition, 'id'>[];
  logicOperator: 'AND' | 'OR';
  useCase: string;
}

export type ComparisonOperator = 
  | 'equals' 
  | 'not_equals' 
  | 'greater_than' 
  | 'less_than' 
  | 'greater_than_or_equal' 
  | 'less_than_or_equal'
  | 'contains'
  | 'not_contains'
  | 'is_empty'
  | 'is_not_empty';

export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  generatedConfig?: RuleConfig;
}

export interface RuleConfig {
  format: 'json' | 'yaml';
  content: string;
  syntax: string;
}

export interface ProjectProfile {
  id: string;
  name: string;
  description: string;
  rules: InvariantRule[];
  createdAt: Date;
  updatedAt: Date;
}

// Drag and Drop types
export interface DragItem {
  type: 'variable' | 'operator' | 'condition';
  data: SorobanVariable | ComparisonOperator | RuleCondition;
  index?: number;
}

export interface BlockBuilderState {
  conditions: RuleCondition[];
  logicOperator: 'AND' | 'OR';
  draggedItem: DragItem | null;
  draggedOverIndex: number | null;
}
