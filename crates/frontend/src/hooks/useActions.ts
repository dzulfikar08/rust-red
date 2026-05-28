/**
 * useActions — React hook that wraps the action store for component use.
 *
 * Provides a stable API surface and returns unregister functions
 * that clean up when the calling component unmounts.
 *
 * Replaces Node-RED's RED.actions.
 */

import { useCallback, useEffect, useRef } from "react";
import { useActionStore } from "../store/action-store";
import type { Action } from "../store/action-store";

export type { Action };

export interface UseActionsReturn {
  registerAction: (action: Action) => () => void;
  invokeAction: (id: string) => void;
  getAction: (id: string) => Action | undefined;
  getAllActions: () => Action[];
  getActionsByScope: (scope: string) => Action[];
}

export function useActions(): UseActionsReturn {
  const unregisterFns = useRef<(() => void)[]>([]);

  const store = useActionStore;

  const registerAction = useCallback(
    (action: Action): (() => void) => {
      store.getState().registerAction(action);
      const unregister = () => {
        store.getState().unregisterAction(action.id);
      };
      unregisterFns.current.push(unregister);
      return unregister;
    },
    [store],
  );

  const invokeAction = useCallback(
    (id: string) => {
      store.getState().invokeAction(id);
    },
    [store],
  );

  const getAction = useCallback(
    (id: string) => store.getState().getAction(id),
    [store],
  );

  const getAllActions = useCallback(() => store.getState().getAllActions(), [store]);

  const getActionsByScope = useCallback(
    (scope: string) => store.getState().getActionsByScope(scope),
    [store],
  );

  // Clean up any actions registered through this hook instance on unmount
  useEffect(() => {
    const fns = unregisterFns.current;
    return () => {
      fns.forEach((fn) => fn());
      fns.length = 0;
    };
  }, []);

  return {
    registerAction,
    invokeAction,
    getAction,
    getAllActions,
    getActionsByScope,
  };
}

export default useActions;
