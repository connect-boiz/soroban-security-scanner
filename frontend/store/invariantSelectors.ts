import { useMemo } from 'react';
import { useInvariantStore } from './invariantStore';

// Optimized selectors using shallow comparison to prevent unnecessary re-renders
export const useCurrentProject = () => {
  return useInvariantStore(state => state.getCurrentProject());
};

export const useAllProjects = () => {
  return useInvariantStore(state => state.getAllProjects());
};

export const useProjectById = (id: string) => {
  return useMemo(() => {
    return useInvariantStore.getState().getProjectById(id);
  }, [id]);
};

export const useProjectRules = (projectId: string) => {
  return useInvariantStore(state => state.getProjectRules(projectId));
};

export const useActiveRules = (projectId: string) => {
  return useInvariantStore(state => state.getActiveRules(projectId));
};

export const useBuilderState = () => {
  return useInvariantStore(state => state.builderState);
};

export const useProjectActions = () => {
  return useInvariantStore(state => ({
    setCurrentProject: state.setCurrentProject,
    createProject: state.createProject,
    updateProject: state.updateProject,
    deleteProject: state.deleteProject,
  }));
};

export const useRuleActions = () => {
  return useInvariantStore(state => ({
    addRule: state.addRule,
    updateRule: state.updateRule,
    deleteRule: state.deleteRule,
    toggleRule: state.toggleRule,
  }));
};

export const useBuilderActions = () => {
  return useInvariantStore(state => ({
    addCondition: state.addCondition,
    updateCondition: state.updateCondition,
    removeCondition: state.removeCondition,
    moveCondition: state.moveCondition,
    setLogicOperator: state.setLogicOperator,
    clearBuilder: state.clearBuilder,
  }));
};

export const useValidationState = () => {
  return useInvariantStore(state => ({
    validationResult: state.validationResult,
    isValidating: state.isValidating,
    validateRule: state.validateRule,
    setValidationResult: state.setValidationResult,
  }));
};
