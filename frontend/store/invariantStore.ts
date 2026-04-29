import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { shallow } from 'zustand/shallow';
import { InvariantRule, RuleCondition, ProjectProfile, BlockBuilderState, ValidationResult } from '@/types/invariant';
import { DEFI_TEMPLATES } from '@/data/invariants';
import { useMemo } from 'react';

interface InvariantStore {
  // Normalized state
  projects: Record<string, ProjectProfile>;
  currentProjectId: string | null;
  
  // Rule builder state
  builderState: BlockBuilderState;
  
  // UI state
  selectedTemplate: string | null;
  isConfigPanelOpen: boolean;
  validationResult: ValidationResult | null;
  isValidating: boolean;
  
  // Actions
  setCurrentProject: (projectId: string | null) => void;
  createProject: (name: string, description: string) => string;
  updateProject: (projectId: string, updates: Partial<ProjectProfile>) => void;
  deleteProject: (projectId: string) => void;
  
  // Rule management
  addRule: (projectId: string, rule: Omit<InvariantRule, 'id' | 'createdAt' | 'updatedAt'>) => void;
  updateRule: (projectId: string, ruleId: string, updates: Partial<InvariantRule>) => void;
  deleteRule: (projectId: string, ruleId: string) => void;
  toggleRule: (projectId: string, ruleId: string) => void;
  
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
  
  // Selectors
  getCurrentProject: () => ProjectProfile | null;
  getProjectById: (id: string) => ProjectProfile | undefined;
  getAllProjects: () => ProjectProfile[];
  getProjectRules: (projectId: string) => InvariantRule[];
  getActiveRules: (projectId: string) => InvariantRule[];
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
      // Initial normalized state
      projects: {},
      currentProjectId: null,
      
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
      setCurrentProject: (projectId) => set({ currentProjectId: projectId }),
      
      createProject: (name, description) => {
        const newProject = createNewProject(name, description);
        set((state) => ({
          projects: { ...state.projects, [newProject.id]: newProject },
          currentProjectId: newProject.id,
        }));
        return newProject.id;
      },
      
      updateProject: (projectId, updates) => set((state) => {
        const project = state.projects[projectId];
        if (!project) return state;
        
        const updatedProject = { ...project, ...updates, updatedAt: new Date() };
        return {
          projects: { ...state.projects, [projectId]: updatedProject },
        };
      }),
      
      deleteProject: (projectId) => set((state) => {
        const newProjects = { ...state.projects };
        delete newProjects[projectId];
        
        return {
          projects: newProjects,
          currentProjectId: state.currentProjectId === projectId ? null : state.currentProjectId,
        };
      }),
      
      // Rule management
      addRule: (projectId, ruleData) => {
        const newRule = createNewRule(ruleData);
        const { projects } = get();
        const project = projects[projectId];
        
        if (project) {
          const updatedProject = {
            ...project,
            rules: [...project.rules, newRule],
            updatedAt: new Date(),
          };
          get().updateProject(projectId, updatedProject);
        }
      },
      
      updateRule: (projectId, ruleId, updates) => {
        const { projects } = get();
        const project = projects[projectId];
        
        if (project) {
          const updatedRules = project.rules.map(rule =>
            rule.id === ruleId ? { ...rule, ...updates, updatedAt: new Date() } : rule
          );
          get().updateProject(projectId, { rules: updatedRules });
        }
      },
      
      deleteRule: (projectId, ruleId) => {
        const { projects } = get();
        const project = projects[projectId];
        
        if (project) {
          const updatedRules = project.rules.filter(rule => rule.id !== ruleId);
          get().updateProject(projectId, { rules: updatedRules });
        }
      },
      
      toggleRule: (projectId, ruleId) => {
        const { projects } = get();
        const project = projects[projectId];
        
        if (project) {
          const updatedRules = project.rules.map(rule =>
            rule.id === ruleId ? { ...rule, isActive: !rule.isActive, updatedAt: new Date() } : rule
          );
          get().updateProject(projectId, { rules: updatedRules });
        }
      },
      
      // Block builder actions (optimized)
      addCondition: (condition) => set((state) => {
        const newConditions = [...state.builderState.conditions, condition];
        return {
          builderState: {
            ...state.builderState,
            conditions: newConditions,
          },
        };
      }),
      
      updateCondition: (index, condition) => set((state) => {
        const newConditions = state.builderState.conditions.map((c, i) => i === index ? condition : c);
        return {
          builderState: {
            ...state.builderState,
            conditions: newConditions,
          },
        };
      }),
      
      removeCondition: (index) => set((state) => {
        const newConditions = state.builderState.conditions.filter((_, i) => i !== index);
        return {
          builderState: {
            ...state.builderState,
            conditions: newConditions,
          },
        };
      }),
      
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
            Array.from(builderState.conditions.map(condition => [
              `  - variable: ${condition.variable.name}`,
              `    operator: ${condition.operator}`,
              `    value: ${condition.value}`,
              `    valueType: ${condition.valueType}`,
            ])).join('\n'),
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
      
      // Selectors
      getCurrentProject: () => {
        const { projects, currentProjectId } = get();
        return currentProjectId ? projects[currentProjectId] : null;
      },
      
      getProjectById: (id) => {
        const { projects } = get();
        return projects[id];
      },
      
      getAllProjects: () => {
        const { projects } = get();
        return Object.values(projects);
      },
      
      getProjectRules: (projectId) => {
        const { projects } = get();
        const project = projects[projectId];
        return project ? project.rules : [];
      },
      
      getActiveRules: (projectId) => {
        const { projects } = get();
        const project = projects[projectId];
        return project ? project.rules.filter(rule => rule.isActive) : [];
      },
    }),
    {
      name: 'invariant-store',
      partialize: (state) => ({
        projects: state.projects,
        currentProjectId: state.currentProjectId,
      }),
    }
  )
);
