import { create } from 'zustand';
import { shallow } from 'zustand/shallow';

interface InvariantStore {
  currentProject: any;
  projects: any[];
  builderState: any;
  validationResult: any;
  isValidating: boolean;
  getCurrentProject: () => any;
  getAllProjects: () => any[];
  getProjectById: (id: string) => any;
  getProjectRules: (projectId: string) => any[];
  getActiveRules: (projectId: string) => any[];
  setCurrentProject: (project: any) => void;
  createProject: (project: any) => void;
  updateProject: (project: any) => void;
  deleteProject: (id: string) => void;
  addRule: (rule: any) => void;
  updateRule: (rule: any) => void;
  deleteRule: (id: string) => void;
  toggleRule: (id: string) => void;
  addCondition: (condition: any) => void;
  updateCondition: (condition: any) => void;
  removeCondition: (id: string) => void;
  moveCondition: (from: number, to: number) => void;
  setLogicOperator: (op: any) => void;
  clearBuilder: () => void;
  validateRule: (rule: any) => void;
  setValidationResult: (result: any) => void;
}

export const useInvariantStore = create<InvariantStore>()((set, get) => ({
  currentProject: null,
  projects: [],
  builderState: {},
  validationResult: null,
  isValidating: false,
  getCurrentProject: () => null,
  getAllProjects: () => [],
  getProjectById: (id: string) => null,
  getProjectRules: (projectId: string) => [],
  getActiveRules: (projectId: string) => [],
  setCurrentProject: project => set({ currentProject: project }),
  createProject: project => set(s => ({ projects: [...s.projects, project] })),
  updateProject: project =>
    set(s => ({
      projects: s.projects.map(p => (p.id === project.id ? project : p)),
    })),
  deleteProject: id =>
    set(s => ({
      projects: s.projects.filter(p => p.id !== id),
    })),
  addRule: rule => {},
  updateRule: rule => {},
  deleteRule: id => {},
  toggleRule: id => {},
  addCondition: condition => {},
  updateCondition: condition => {},
  removeCondition: id => {},
  moveCondition: (from, to) => {},
  setLogicOperator: op => {},
  clearBuilder: () => {},
  validateRule: rule => {},
  setValidationResult: result => set({ validationResult: result }),
}));
