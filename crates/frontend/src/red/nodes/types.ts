/**
 * Node-RED Node Type Definitions
 *
 * TypeScript types matching Node-RED's node definition format.
 * These types describe the shape of node type registrations,
 * flow data, and configuration used throughout the editor.
 */

// ---------------------------------------------------------------------------
// Node-definition types (what gets registered per node type)
// ---------------------------------------------------------------------------

export interface NodeDefault {
  /** Default value for this property */
  value: unknown;
  /** Whether this property is required */
  required?: boolean;
  /** Validation function or RED.validators reference string */
  validate?: string | ((value: unknown) => boolean);
  /** For typedInput fields – the expected sub-type */
  type?: string;
}

export interface NodeCredential {
  type: string;
  required?: boolean;
}

export interface NodeDefinition {
  /** Unique type identifier, e.g. "inject" */
  id: string;
  /** Node type name */
  type: string;
  /** Human-readable name */
  name: string;
  /** Palette category: common, function, network, storage, parser, sequence … */
  category: string;
  /** Category colour hex, e.g. '#a6bbcf' */
  color: string;
  /** Property defaults keyed by property name */
  defaults: Record<string, NodeDefault>;
  /** Credential definitions */
  credentials?: Record<string, NodeCredential>;
  /** Number of input ports (0 or 1) */
  inputs: number;
  /** Number of output ports (0-4) */
  outputs: number;
  /** Font Awesome icon class */
  icon?: string;
  /** Label – static string or function returning one */
  label?: string | ((node: Record<string, unknown>) => string);
  /** CSS class for label styling */
  labelStyle?: string;
  /** Palette label (shorter, shown in the palette) */
  paletteLabel?: string;
  /** Alignment of icon relative to label */
  align?: "left" | "right";
  /** Labels for each output port */
  outputLabels?: (string | ((index: number) => string))[];
  /** Labels for each input port */
  inputLabels?: (string | ((index: number) => string))[];
  /** Editor lifecycle hooks */
  button?: { onclick: () => void };
  oneditprepare?: () => void;
  oneditsave?: () => void;
  oneditcancel?: () => void;
  oneditdelete?: () => void;
  oneditresize?: (size: { width: number; height: number }) => void;
}

// ---------------------------------------------------------------------------
// Flow data model types (matching Node-RED flow JSON format)
// ---------------------------------------------------------------------------

export interface FlowNode {
  /** Unique node identifier */
  id: string;
  /** Node type, e.g. "inject" */
  type: string;
  /** Optional display name */
  name?: string;
  /** X canvas position */
  x: number;
  /** Y canvas position */
  y: number;
  /** Tab / flow ID this node belongs to */
  z: string;
  /** Output wires – array of arrays of connected node IDs */
  wires: string[][];
  /** Any additional node-type-specific properties */
  [key: string]: unknown;
}

export interface ConfigNode extends FlowNode {
  /** IDs of nodes that reference this config node */
  users: string[];
}

export interface SubflowPort {
  x: number;
  y: number;
  wires: string[][];
}

export interface Subflow {
  id: string;
  name: string;
  info?: string;
  category?: string;
  color?: string;
  icon?: string;
  in: SubflowPort[];
  out: SubflowPort[];
}

export interface Flow {
  id: string;
  label: string;
  type?: "tab" | "subflow";
  disabled?: boolean;
  info?: string;
  nodes: FlowNode[];
  configs: ConfigNode[];
  subflows?: Subflow[];
}
