import { useMemo } from 'react';
import { useInvariantStore } from './invariantStore';
import { shallow } from 'zustand/shallow';

// Optimized selectors using shallow comparison to prevent unnecessary re-renders
export const useCurrentProject = () => {
  return useInvariantStore(
    (state) => state.getCurrentProject(),
    shallow
  );
};

export const useAllProjects = () => {
  return useInvariantStore(
    (state) => state.getAllProjects(),
    shallow
  );
};

export const useProjectById = (id: string) => {
  return useMemo(() => {
    return useInvariantStore.getState().getProjectById(id);
  }, [id]);
};

export const useProjectRules = (projectId: string) => {
  return useInvariantStore(
    (state) => state.getProjectRules(projectId),
    shallow
  );
};

export const useActiveRules = (projectId: string) => {
  return useInvariantStore(
    (state) => state.getActiveRules(projectId),
    shallow
  );
};

export const useBuilderState = () => {
  return useInvariantStore(
    (state) => state.builderState,
    shallow
  );
};

export const useProjectActions = () => {
  return useInvariantStore(
    (state) => ({
      setCurrentProject: state.setCurrentProject,
      createProject: state.createProject,
      updateProject: state.updateProject,
      deleteProject: state.deleteProject,
    }),
    shallow
  );
};

export const useRuleActions = () => {
  return useInvariantStore(
    (state) => ({
      addRule: state.addRule,
      updateRule: state.updateRule,
      deleteRule: state.deleteRule,
      toggleRule: state.toggleRule,
    }),
    shallow
  );
};

export const useBuilderActions = () => {
  return useInvariantStore(
    (state) => ({
      addCondition: state.addCondition,
      updateCondition: state.updateCondition,
      removeCondition: state.removeCondition,
      moveCondition: state.moveCondition,
      setLogicOperator: state.setLogicOperator,
      clearBuilder: state.clearBuilder,
    }),
    shallow
  );
};

export const useValidationState = () => {
  return useInvariantStore(
    (state) => ({
      validationResult: state.validationResult,
      isValidating: state.isValidating,
      validateRule: state.validateRule,
      setValidationResult: state.setValidationResult,
    }),
    shallow
  );
};
