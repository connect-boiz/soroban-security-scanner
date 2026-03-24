import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { InvariantRule, RuleCondition, ProjectProfile, BlockBuilderState, ValidationResult } from '@/types/invariant';
import { DEFI_TEMPLATES } from '@/data/invariants';

interface InvariantStore {
  // Current project profile
  currentProject: ProjectProfile | null;
  projects: ProjectProfile[];
  
  // Rule builder state
  builderState: BlockBuilderState;
  
  // UI state
  selectedTemplate: string | null;
  isConfigPanelOpen: boolean;
  validationResult: ValidationResult | null;
  isValidating: boolean;
  
  // Actions
  setCurrentProject: (project: ProjectProfile | null) => void;
  createProject: (name: string, description: string) => ProjectProfile;
  updateProject: (project: ProjectProfile) => void;
  deleteProject: (projectId: string) => void;
  
  // Rule management
  addRule: (rule: Omit<InvariantRule, 'id' | 'createdAt' | 'updatedAt'>) => void;
  updateRule: (ruleId: string, updates: Partial<InvariantRule>) => void;
  deleteRule: (ruleId: string) => void;
  toggleRule: (ruleId: string) => void;
  
  // Block builder actions
  addCondition: (condition: RuleCondition) => void;
  updateCondition: (index: number, condition: RuleCondition) => void;
  removeCondition: (index: number) => void;
  moveCondition: (fromIndex: number, toIndex: number) => void;
  setLogicOperator: (operator: 'AND' | 'OR') => void;
  clearBuilder: () => void;
  
  // Template actions
  loadTemplate: (templateId: string) => void;
  setSelectedTemplate: (templateId: string | null) => void;
  
  // Validation actions
  validateRule: (rule: InvariantRule) => Promise<ValidationResult>;
  setValidationResult: (result: ValidationResult | null) => void;
  setIsValidating: (validating: boolean) => void;
  
  // Configuration actions
  generateConfig: (format: 'json' | 'yaml') => string;
  setIsConfigPanelOpen: (open: boolean) => void;
  
  // Drag and drop actions
  setDraggedItem: (item: any) => void;
  setDraggedOverIndex: (index: number | null) => void;
}

const generateId = () => Math.random().toString(36).substr(2, 9);

const createNewProject = (name: string, description: string): ProjectProfile => ({
  id: generateId(),
  name,
  description,
  rules: [],
  createdAt: new Date(),
  updatedAt: new Date(),
});

const createNewRule = (ruleData: Omit<InvariantRule, 'id' | 'createdAt' | 'updatedAt'>): InvariantRule => ({
  ...ruleData,
  id: generateId(),
  createdAt: new Date(),
  updatedAt: new Date(),
});

export const useInvariantStore = create<InvariantStore>()(
  persist(
    (set, get) => ({
      // Initial state
      currentProject: null,
      projects: [],
      
      builderState: {
        conditions: [],
        logicOperator: 'AND',
        draggedItem: null,
        draggedOverIndex: null,
      },
      
      selectedTemplate: null,
      isConfigPanelOpen: false,
      validationResult: null,
      isValidating: false,
      
      // Project management
      setCurrentProject: (project) => set({ currentProject: project }),
      
      createProject: (name, description) => {
        const newProject = createNewProject(name, description);
        set((state) => ({
          projects: [...state.projects, newProject],
          currentProject: newProject,
        }));
        return newProject;
      },
      
      updateProject: (project) => set((state) => ({
        projects: state.projects.map(p => p.id === project.id ? { ...project, updatedAt: new Date() } : p),
        currentProject: state.currentProject?.id === project.id ? { ...project, updatedAt: new Date() } : state.currentProject,
      })),
      
      deleteProject: (projectId) => set((state) => ({
        projects: state.projects.filter(p => p.id !== projectId),
        currentProject: state.currentProject?.id === projectId ? null : state.currentProject,
      })),
      
      // Rule management
      addRule: (ruleData) => {
        const newRule = createNewRule(ruleData);
        const { currentProject } = get();
        
        if (currentProject) {
          const updatedProject = {
            ...currentProject,
            rules: [...currentProject.rules, newRule],
            updatedAt: new Date(),
          };
          get().updateProject(updatedProject);
        }
      },
      
      updateRule: (ruleId, updates) => {
        const { currentProject } = get();
        
        if (currentProject) {
          const updatedRules = currentProject.rules.map(rule =>
            rule.id === ruleId ? { ...rule, ...updates, updatedAt: new Date() } : rule
          );
          const updatedProject = {
            ...currentProject,
            rules: updatedRules,
            updatedAt: new Date(),
          };
          get().updateProject(updatedProject);
        }
      },
      
      deleteRule: (ruleId) => {
        const { currentProject } = get();
        
        if (currentProject) {
          const updatedRules = currentProject.rules.filter(rule => rule.id !== ruleId);
          const updatedProject = {
            ...currentProject,
            rules: updatedRules,
            updatedAt: new Date(),
          };
          get().updateProject(updatedProject);
        }
      },
      
      toggleRule: (ruleId) => {
        const { currentProject } = get();
        
        if (currentProject) {
          const updatedRules = currentProject.rules.map(rule =>
            rule.id === ruleId ? { ...rule, isActive: !rule.isActive, updatedAt: new Date() } : rule
          );
          const updatedProject = {
            ...currentProject,
            rules: updatedRules,
            updatedAt: new Date(),
          };
          get().updateProject(updatedProject);
        }
      },
      
      // Block builder actions
      addCondition: (condition) => set((state) => ({
        builderState: {
          ...state.builderState,
          conditions: [...state.builderState.conditions, condition],
        },
      })),
      
      updateCondition: (index, condition) => set((state) => ({
        builderState: {
          ...state.builderState,
          conditions: state.builderState.conditions.map((c, i) => i === index ? condition : c),
        },
      })),
      
      removeCondition: (index) => set((state) => ({
        builderState: {
          ...state.builderState,
          conditions: state.builderState.conditions.filter((_, i) => i !== index),
        },
      })),
      
      moveCondition: (fromIndex, toIndex) => set((state) => {
        const { conditions } = state.builderState;
        const newConditions = [...conditions];
        const [moved] = newConditions.splice(fromIndex, 1);
        newConditions.splice(toIndex, 0, moved);
        
        return {
          builderState: {
            ...state.builderState,
            conditions: newConditions,
          },
        };
      }),
      
      setLogicOperator: (operator) => set((state) => ({
        builderState: {
          ...state.builderState,
          logicOperator: operator,
        },
      })),
      
      clearBuilder: () => set((state) => ({
        builderState: {
          ...state.builderState,
          conditions: [],
          logicOperator: 'AND',
        },
        selectedTemplate: null,
        validationResult: null,
      })),
      
      // Template actions
      loadTemplate: (templateId) => {
        const template = DEFI_TEMPLATES.find(t => t.id === templateId);
        if (template) {
          const conditionsWithIds = template.conditions.map(condition => ({
            ...condition,
            id: generateId(),
          }));
          
          set((state) => ({
            builderState: {
              ...state.builderState,
              conditions: conditionsWithIds,
              logicOperator: template.logicOperator,
            },
            selectedTemplate: templateId,
          }));
        }
      },
      
      setSelectedTemplate: (templateId) => set({ selectedTemplate: templateId }),
      
      // Validation actions
      validateRule: async (rule) => {
        set({ isValidating: true, validationResult: null });
        
        try {
          // Simulate API call to validate rule
          const response = await fetch('/api/invariants/validate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(rule),
          });
          
          const result: ValidationResult = await response.json();
          set({ validationResult: result });
          return result;
        } catch (error) {
          const errorResult: ValidationResult = {
            isValid: false,
            errors: ['Failed to validate rule. Please try again.'],
            warnings: [],
          };
          set({ validationResult: errorResult });
          return errorResult;
        } finally {
          set({ isValidating: false });
        }
      },
      
      setValidationResult: (result) => set({ validationResult: result }),
      setIsValidating: (validating) => set({ isValidating: validating }),
      
      // Configuration actions
      generateConfig: (format) => {
        const { builderState } = get();
        const config = {
          name: 'Generated Invariant Rule',
          logicOperator: builderState.logicOperator,
          conditions: builderState.conditions.map(({ id, ...condition }) => condition),
        };
        
        if (format === 'json') {
          return JSON.stringify(config, null, 2);
        } else {
          // Simple YAML generation (in production, use a proper YAML library)
          const yaml = [
            'name: Generated Invariant Rule',
            `logicOperator: ${builderState.logicOperator}`,
            'conditions:',
            ...builderState.conditions.map(condition => [
              `  - variable: ${condition.variable.name}`,
              `    operator: ${condition.operator}`,
              `    value: ${condition.value}`,
              `    valueType: ${condition.valueType}`,
            ]).join('\n'),
          ].join('\n');
          
          return yaml;
        }
      },
      
      setIsConfigPanelOpen: (open) => set({ isConfigPanelOpen: open }),
      
      // Drag and drop actions
      setDraggedItem: (item) => set((state) => ({
        builderState: {
          ...state.builderState,
          draggedItem: item,
        },
      })),
      
      setDraggedOverIndex: (index) => set((state) => ({
        builderState: {
          ...state.builderState,
          draggedOverIndex: index,
        },
      })),
    }),
    {
      name: 'invariant-store',
      partialize: (state) => ({
        projects: state.projects,
        currentProject: state.currentProject,
      }),
    }
  )
);
