import { useMemo } from 'react';
import { useInvariantStore } from './invariantStore';
import { shallow } from 'zustand/shallow';

// Optimized selectors using shallow comparison to prevent unnecessary re-renders
export const useCurrentProject = () => {
  return (useInvariantStore as any)((state: any) => state.getCurrentProject(), shallow);
};

export const useAllProjects = () => {
  return (useInvariantStore as any)((state: any) => state.getAllProjects(), shallow);
};

export const useProjectById = (id: string) => {
  return useMemo(() => {
    return useInvariantStore.getState().getProjectById(id);
  }, [id]);
};

export const useProjectRules = (projectId: string) => {
  return (useInvariantStore as any)((state: any) => state.getProjectRules(projectId), shallow);
};

export const useActiveRules = (projectId: string) => {
  return (useInvariantStore as any)((state: any) => state.getActiveRules(projectId), shallow);
};

export const useBuilderState = () => {
  return (useInvariantStore as any)((state: any) => state.builderState, shallow);
};

export const useProjectActions = () => {
  return (useInvariantStore as any)(
    (state: any) => ({
      setCurrentProject: state.setCurrentProject,
      createProject: state.createProject,
      updateProject: state.updateProject,
      deleteProject: state.deleteProject,
    }),
    shallow
  );
};

export const useRuleActions = () => {
  return (useInvariantStore as any)(
    (state: any) => ({
      addRule: state.addRule,
      updateRule: state.updateRule,
      deleteRule: state.deleteRule,
      toggleRule: state.toggleRule,
    }),
    shallow
  );
};

export const useBuilderActions = () => {
  return (useInvariantStore as any)(
    (state: any) => ({
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
  return (useInvariantStore as any)(
    (state: any) => ({
      validationResult: state.validationResult,
      isValidating: state.isValidating,
      validateRule: state.validateRule,
      setValidationResult: state.setValidationResult,
    }),
    shallow
  );
};
