/**
 * Node Editor Registry
 *
 * Pure registry logic -- no imports of editor components, so editors can
 * safely import from here without creating circular dependencies.
 */

import type { ComponentType } from "react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Props passed to every node editor component (custom or default).
 */
export interface NodeEditorProps {
  /** ID of the node being edited */
  nodeId: string;
  /** Node type string, e.g. "inject" */
  nodeType: string;
  /** Current property values for the node */
  values: Record<string, any>;
  /** Callback to update a single property */
  onChange: (key: string, value: any) => void;
  /** Validation errors keyed by property name */
  errors?: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Registry (module-scoped)
// ---------------------------------------------------------------------------

type NodeEditorComponent = ComponentType<NodeEditorProps>;

const editors = new Map<string, NodeEditorComponent>();

/**
 * Register a custom editor component for a node type.
 */
export function registerNodeEditor(type: string, component: NodeEditorComponent): void {
  editors.set(type, component);
}

/**
 * Get the custom editor component for a node type, or `null` if none is
 * registered.
 */
export function getNodeEditor(type: string): NodeEditorComponent | null {
  return editors.get(type) ?? null;
}

/**
 * Check whether a custom editor has been registered for a node type.
 */
export function hasCustomEditor(type: string): boolean {
  return editors.has(type);
}

/**
 * Remove a custom editor registration (useful for tests or hot-reload).
 */
export function unregisterNodeEditor(type: string): void {
  editors.delete(type);
}

/**
 * Remove all registrations (test utility).
 */
export function clearNodeEditors(): void {
  editors.clear();
}
