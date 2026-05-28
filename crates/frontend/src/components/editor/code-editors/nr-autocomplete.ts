/**
 * Node-RED API autocomplete provider for Monaco Editor.
 *
 * Registers a completion item provider for JavaScript/TypeScript languages that
 * suggests Node-RED globals and their properties/methods:
 *
 *   - `msg` object (payload, topic, parts, etc.)
 *   - `flow` context API (flow.get, flow.set, flow.keys)
 *   - `global` context API (global.get, global.set, global.keys)
 *   - `context` API (context.get, context.set, context.keys)
 *   - `RED` utility functions
 *   - `node` object (node.id, node.name, node.log, node.warn, node.error,
 *     node.status, node.send)
 *
 * Call {@link registerNRAutocomplete} once when Monaco is loaded.  The
 * disposable it returns can be used to clean up the registration.
 */

import type * as Monaco from "monaco-editor";

// ---------------------------------------------------------------------------
// Completion item helpers
// ---------------------------------------------------------------------------

interface NRCompletion {
  label: string;
  kind: Monaco.languages.CompletionItemKind;
  insertText: string;
  detail?: string;
  documentation?: string;
}

function toItem(
  c: NRCompletion,
  range: Monaco.IRange,
): Monaco.languages.CompletionItem {
  return {
    label: c.label,
    kind: c.kind,
    insertText: c.insertText,
    detail: c.detail,
    documentation: c.documentation,
    range,
  };
}

// ---------------------------------------------------------------------------
// Static completions
// ---------------------------------------------------------------------------

const MSG_PROPERTIES: NRCompletion[] = [
  { label: "msg", kind: 6, insertText: "msg", detail: "Message object", documentation: "The message object passed between nodes." },
  { label: "msg.payload", kind: 6, insertText: "msg.payload", detail: "msg.payload", documentation: "The main contents of the message." },
  { label: "msg.topic", kind: 6, insertText: "msg.topic", detail: "msg.topic", documentation: "The topic the message was sent on." },
  { label: "msg.parts", kind: 6, insertText: "msg.parts", detail: "msg.parts", documentation: "Used by split/join nodes to track sequences." },
  { label: "msg._msgid", kind: 6, insertText: "msg._msgid", detail: "msg._msgid", documentation: "Unique identifier for the message." },
  { label: "msg.qos", kind: 6, insertText: "msg.qos", detail: "msg.qos", documentation: "MQTT quality of service level." },
  { label: "msg.retain", kind: 6, insertText: "msg.retain", detail: "msg.retain", documentation: "MQTT retain flag." },
];

const FLOW_METHODS: NRCompletion[] = [
  { label: "flow.get", kind: 1, insertText: "flow.get('${1:key}')", detail: "flow.get(key)", documentation: "Get a flow-scoped context property." },
  { label: "flow.set", kind: 1, insertText: "flow.set('${1:key}', ${2:value})", detail: "flow.set(key, value)", documentation: "Set a flow-scoped context property." },
  { label: "flow.keys", kind: 1, insertText: "flow.keys()", detail: "flow.keys()", documentation: "Return all keys in flow context." },
  { label: "flow", kind: 6, insertText: "flow", detail: "Flow context", documentation: "Flow-scoped context API." },
];

const GLOBAL_METHODS: NRCompletion[] = [
  { label: "global.get", kind: 1, insertText: "global.get('${1:key}')", detail: "global.get(key)", documentation: "Get a global-scoped context property." },
  { label: "global.set", kind: 1, insertText: "global.set('${1:key}', ${2:value})", detail: "global.set(key, value)", documentation: "Set a global-scoped context property." },
  { label: "global.keys", kind: 1, insertText: "global.keys()", detail: "global.keys()", documentation: "Return all keys in global context." },
  { label: "global", kind: 6, insertText: "global", detail: "Global context", documentation: "Global-scoped context API." },
];

const CONTEXT_METHODS: NRCompletion[] = [
  { label: "context.get", kind: 1, insertText: "context.get('${1:key}')", detail: "context.get(key)", documentation: "Get a node-scoped context property." },
  { label: "context.set", kind: 1, insertText: "context.set('${1:key}', ${2:value})", detail: "context.set(key, value)", documentation: "Set a node-scoped context property." },
  { label: "context.keys", kind: 1, insertText: "context.keys()", detail: "context.keys()", documentation: "Return all keys in node context." },
  { label: "context", kind: 6, insertText: "context", detail: "Node context", documentation: "Node-scoped context API." },
];

const RED_FUNCTIONS: NRCompletion[] = [
  { label: "RED.util", kind: 6, insertText: "RED.util", detail: "RED.util", documentation: "Node-RED utility functions." },
  { label: "RED", kind: 6, insertText: "RED", detail: "RED runtime API", documentation: "The Node-RED runtime API object." },
];

const NODE_PROPERTIES: NRCompletion[] = [
  { label: "node.id", kind: 6, insertText: "node.id", detail: "node.id", documentation: "The unique ID of the node." },
  { label: "node.name", kind: 6, insertText: "node.name", detail: "node.name", documentation: "The name of the node." },
  { label: "node.log", kind: 1, insertText: "node.log(${1:message})", detail: "node.log(message)", documentation: "Log a message at INFO level." },
  { label: "node.warn", kind: 1, insertText: "node.warn(${1:message})", detail: "node.warn(message)", documentation: "Log a warning message." },
  { label: "node.error", kind: 1, insertText: "node.error(${1:message})", detail: "node.error(message)", documentation: "Log an error message." },
  { label: "node.status", kind: 1, insertText: "node.status({ fill: '${1:green}', shape: '${2:dot}', text: '${3:}' })", detail: "node.status(opts)", documentation: "Set the node's status indicator." },
  { label: "node.send", kind: 1, insertText: "node.send(${1:msg})", detail: "node.send(msg)", documentation: "Send a message from the node." },
  { label: "node", kind: 6, insertText: "node", detail: "Node instance", documentation: "The current node instance." },
];

const ALL_COMPLETIONS: NRCompletion[] = [
  ...MSG_PROPERTIES,
  ...FLOW_METHODS,
  ...GLOBAL_METHODS,
  ...CONTEXT_METHODS,
  ...RED_FUNCTIONS,
  ...NODE_PROPERTIES,
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

let registered = false;

/**
 * Register the Node-RED autocomplete provider with Monaco.
 *
 * Should be called once after the Monaco editor is loaded.
 * Returns a disposable that can be used to clean up the registration.
 */
export function registerNRAutocomplete(
  monaco: typeof Monaco,
): Monaco.IDisposable {
  if (registered) {
    return { dispose() {} };
  }
  registered = true;

  const disposable = monaco.languages.registerCompletionItemProvider(
    ["javascript", "typescript"],
    {
      triggerCharacters: ["."],
      provideCompletionItems(model, position) {
        const word = model.getWordUntilPosition(position);
        const range: Monaco.IRange = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

        const suggestions = ALL_COMPLETIONS.map((c) => toItem(c, range));

        return { suggestions };
      },
    },
  );

  return {
    dispose() {
      registered = false;
      disposable.dispose();
    },
  };
}

/**
 * Reset the registration flag (useful in tests).
 */
export function _resetRegistration(): void {
  registered = false;
}

/**
 * Return all completion items (useful for testing).
 */
export function _getCompletions(): NRCompletion[] {
  return ALL_COMPLETIONS;
}
