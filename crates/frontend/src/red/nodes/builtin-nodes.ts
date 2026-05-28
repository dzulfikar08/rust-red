/**
 * Built-in Node-RED Node Type Definitions
 *
 * These are the standard node types that ship with Node-RED.
 * Each entry is a `NodeDefinition` describing the palette appearance
 * and property defaults – NOT runtime behaviour.
 *
 * Category colours:
 *   common   #a6bbcf
 *   function #e2d96e
 *   network  #e2d96e
 *   storage  #e2d96e
 *   parser   #c0edc0
 *   sequence #87a669
 */

import type { NodeDefinition } from "./types";
import { nodeRegistry } from "./registry";

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function def(overrides: Partial<NodeDefinition> & Pick<NodeDefinition, "type" | "name" | "category" | "color" | "inputs" | "outputs">): NodeDefinition {
  return {
    id: overrides.type,
    paletteLabel: overrides.name,
    defaults: {},
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Common
// ---------------------------------------------------------------------------

const commonNodes: NodeDefinition[] = [
  def({
    type: "inject",
    name: "inject",
    category: "common",
    color: "#a6bbcf",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-arrow-right",
    defaults: {
      payload: { value: "" },
      payloadType: { value: "date" },
      topic: { value: "" },
      repeat: { value: "" },
      crontab: { value: "" },
      once: { value: false },
      onceDelay: { value: 0.1 },
    },
    label: (node) => (node.topic as string) || "inject",
  }),
  def({
    type: "debug",
    name: "debug",
    category: "common",
    color: "#a6bbcf",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-bug",
    defaults: {
      name: { value: "" },
      active: { value: true },
      tosidebar: { value: true },
      console: { value: false },
      tostatus: { value: false },
      complete: { value: "payload" },
      targetType: { value: "" },
    },
    label: (node) => (node.name as string) || "debug",
  }),
  def({
    type: "comment",
    name: "comment",
    category: "common",
    color: "#a6bbcf",
    inputs: 0,
    outputs: 0,
    icon: "fa-solid fa-comment",
    defaults: {
      name: { value: "" },
      info: { value: "" },
    },
    label: (node) => (node.name as string) || "comment",
  }),
  def({
    type: "link in",
    name: "link in",
    category: "common",
    color: "#a6bbcf",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-link",
    defaults: {
      name: { value: "" },
      links: { value: [] },
    },
    label: (node) => (node.name as string) || "link in",
    align: "left",
  }),
  def({
    type: "link out",
    name: "link out",
    category: "common",
    color: "#a6bbcf",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-link",
    defaults: {
      name: { value: "" },
      mode: { value: "link" },
      links: { value: [] },
    },
    label: (node) => (node.name as string) || "link out",
    align: "right",
  }),
  def({
    type: "link call",
    name: "link call",
    category: "common",
    color: "#a6bbcf",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-link",
    defaults: {
      name: { value: "" },
      link: { value: "" },
      links: { value: [] },
      timeout: { value: "30" },
    },
    label: (node) => (node.name as string) || "link call",
  }),
  def({
    type: "execute",
    name: "execute",
    category: "common",
    color: "#a6bbcf",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-play",
    defaults: {
      name: { value: "" },
      command: { value: "", required: true },
    },
    label: (node) => (node.name as string) || (node.command as string) || "execute",
  }),
];

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

const functionNodes: NodeDefinition[] = [
  def({
    type: "function",
    name: "function",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-code",
    defaults: {
      name: { value: "" },
      func: { value: "return msg;", required: true },
      outputs: { value: 1 },
      noerr: { value: 0 },
      initialize: { value: "" },
      finalize: { value: "" },
      libs: { value: [] },
    },
    label: (node) => (node.name as string) || "function",
  }),
  def({
    type: "switch",
    name: "switch",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-random",
    defaults: {
      name: { value: "" },
      property: { value: "payload" },
      propertyType: { value: "msg" },
      rules: { value: [] },
      checkall: { value: "true" },
      repair: { value: false },
      outputs: { value: 1 },
    },
    label: (node) => (node.name as string) || "switch",
  }),
  def({
    type: "change",
    name: "change",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-random",
    defaults: {
      name: { value: "" },
      rules: { value: [] },
    },
    label: (node) => (node.name as string) || "change",
  }),
  def({
    type: "range",
    name: "range",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-sliders-h",
    defaults: {
      name: { value: "" },
      action: { value: "scale" },
      min: { value: 0, required: true },
      max: { value: 1, required: true },
      outmin: { value: 0, required: true },
      outmax: { value: 1, required: true },
      round: { value: false },
    },
    label: (node) => (node.name as string) || "range",
  }),
  def({
    type: "template",
    name: "template",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-file-code",
    defaults: {
      name: { value: "" },
      field: { value: "payload" },
      fieldType: { value: "msg" },
      format: { value: "handlebars" },
      syntax: { value: "mustache" },
      template: { value: "", required: true },
      output: { value: "str" },
    },
    label: (node) => (node.name as string) || "template",
  }),
  def({
    type: "delay",
    name: "delay",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-clock",
    defaults: {
      name: { value: "" },
      pauseType: { value: "delay" },
      timeout: { value: "5" },
      timeoutUnits: { value: "seconds" },
      rate: { value: "1" },
      nbRateUnits: { value: "1" },
      rateUnits: { value: "second" },
      randomFirst: { value: "1" },
      randomLast: { value: "5" },
      randomUnits: { value: "seconds" },
      drop: { value: false },
    },
    label: (node) => (node.name as string) || "delay",
  }),
  def({
    type: "trigger",
    name: "trigger",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-bolt",
    defaults: {
      name: { value: "" },
      op1: { value: "" },
      op2: { value: "" },
      op1type: { value: "pay" },
      op2type: { value: "nul" },
      duration: { value: "250" },
      extend: { value: false },
      units: { value: "ms" },
      reset: { value: "" },
    },
    label: (node) => (node.name as string) || "trigger",
  }),
  def({
    type: "exec",
    name: "exec",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-terminal",
    defaults: {
      name: { value: "" },
      command: { value: "", required: true },
      addpay: { value: false },
      append: { value: "" },
      useSpawn: { value: false },
      timer: { value: "" },
      winHide: { value: false },
      oldrc: { value: false },
    },
    label: (node) => (node.name as string) || (node.command as string) || "exec",
  }),
  def({
    type: "rbe",
    name: "rbe",
    category: "function",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-filter",
    defaults: {
      name: { value: "" },
      func: { value: "rbe" },
      gap: { value: "" },
      start: { value: "" },
      inout: { value: "out" },
      property: { value: "payload" },
    },
    label: (node) => (node.name as string) || "rbe",
  }),
];

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

const networkNodes: NodeDefinition[] = [
  def({
    type: "httpin",
    name: "http in",
    category: "network",
    color: "#e2d96e",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-globe",
    defaults: {
      name: { value: "" },
      url: { value: "", required: true },
      method: { value: "get" },
      upload: { value: false },
      swaggerDoc: { value: "" },
    },
    label: (node) => `[${node.method as string}] ${(node.url as string) || "http in"}`,
  }),
  def({
    type: "httpresponse",
    name: "http response",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-globe",
    defaults: {
      name: { value: "" },
      statusCode: { value: "" },
      headers: { value: {} },
    },
    label: (node) => (node.name as string) || "http response",
  }),
  def({
    type: "httprequest",
    name: "http request",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-globe",
    defaults: {
      name: { value: "" },
      method: { value: "GET" },
      ret: { value: "txt" },
      paytoqs: { value: false },
      url: { value: "", required: true },
      tls: { value: "" },
      proxy: { value: "" },
    },
    label: (node) => (node.name as string) || "http request",
  }),
  def({
    type: "websocket in",
    name: "websocket in",
    category: "network",
    color: "#e2d96e",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-plug",
    defaults: {
      name: { value: "" },
      server: { value: "" },
      client: { value: "" },
    },
    label: (node) => (node.name as string) || "websocket in",
  }),
  def({
    type: "websocket out",
    name: "websocket out",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-plug",
    defaults: {
      name: { value: "" },
      server: { value: "" },
      client: { value: "" },
    },
    label: (node) => (node.name as string) || "websocket out",
  }),
  def({
    type: "mqtt in",
    name: "mqtt in",
    category: "network",
    color: "#e2d96e",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-rss",
    defaults: {
      name: { value: "" },
      topic: { value: "", required: true },
      qos: { value: "2" },
      datatype: { value: "auto" },
      broker: { value: "", type: "mqtt-broker" },
    },
    label: (node) => (node.name as string) || (node.topic as string) || "mqtt in",
  }),
  def({
    type: "mqtt out",
    name: "mqtt out",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-rss",
    defaults: {
      name: { value: "" },
      topic: { value: "" },
      qos: { value: "" },
      retain: { value: "" },
      broker: { value: "", type: "mqtt-broker" },
    },
    label: (node) => (node.name as string) || "mqtt out",
  }),
  def({
    type: "tcp in",
    name: "tcp in",
    category: "network",
    color: "#e2d96e",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-network-wired",
    defaults: {
      name: { value: "" },
      host: { value: "" },
      port: { value: "" },
      server: { value: "client" },
      datamode: { value: "stream" },
      datatype: { value: "buffer" },
      newline: { value: "" },
    },
    label: (node) => (node.name as string) || "tcp in",
  }),
  def({
    type: "tcp out",
    name: "tcp out",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-network-wired",
    defaults: {
      name: { value: "" },
      host: { value: "" },
      port: { value: "" },
      server: { value: "client" },
    },
    label: (node) => (node.name as string) || "tcp out",
  }),
  def({
    type: "udp in",
    name: "udp in",
    category: "network",
    color: "#e2d96e",
    inputs: 0,
    outputs: 1,
    icon: "fa-solid fa-network-wired",
    defaults: {
      name: { value: "" },
      host: { value: "" },
      port: { value: "" },
      multicast: { value: "false" },
      group: { value: "" },
      datatype: { value: "buffer" },
    },
    label: (node) => (node.name as string) || "udp in",
  }),
  def({
    type: "udp out",
    name: "udp out",
    category: "network",
    color: "#e2d96e",
    inputs: 1,
    outputs: 0,
    icon: "fa-solid fa-network-wired",
    defaults: {
      name: { value: "" },
      host: { value: "" },
      port: { value: "" },
      multicast: { value: "false" },
    },
    label: (node) => (node.name as string) || "udp out",
  }),
];

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

const storageNodes: NodeDefinition[] = [
  def({
    type: "file",
    name: "file",
    category: "storage",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-file",
    defaults: {
      name: { value: "" },
      filename: { value: "", required: true },
      appendNewline: { value: true },
      createDir: { value: false },
      overwriteFile: { value: "false" },
      encoding: { value: "none" },
    },
    label: (node) => (node.name as string) || (node.filename as string) || "file",
  }),
  def({
    type: "file in",
    name: "file in",
    category: "storage",
    color: "#e2d96e",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-file-import",
    defaults: {
      name: { value: "" },
      filename: { value: "", required: true },
      format: { value: "" },
      chunk: { value: false },
      sendError: { value: false },
      encoding: { value: "none" },
      allProps: { value: false },
    },
    label: (node) => (node.name as string) || (node.filename as string) || "file in",
  }),
];

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

const parserNodes: NodeDefinition[] = [
  def({
    type: "json",
    name: "json",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-database",
    defaults: {
      name: { value: "" },
      action: { value: "" },
      property: { value: "payload" },
    },
    label: (node) => (node.name as string) || "json",
  }),
  def({
    type: "xml",
    name: "xml",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-code",
    defaults: {
      name: { value: "" },
      action: { value: "" },
      property: { value: "payload" },
    },
    label: (node) => (node.name as string) || "xml",
  }),
  def({
    type: "csv",
    name: "csv",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-table",
    defaults: {
      name: { value: "" },
      sep: { value: "," },
      hdrin: { value: "" },
      hdrout: { value: "" },
      multi: { value: "one" },
      ret: { value: "\\n" },
      temp: { value: "" },
      skip: { value: "0" },
      strings: { value: true },
      include_empty_strings: { value: "" },
      include_null: { value: "" },
    },
    label: (node) => (node.name as string) || "csv",
  }),
  def({
    type: "html",
    name: "html",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-code",
    defaults: {
      name: { value: "" },
      property: { value: "payload" },
      outproperty: { value: "payload" },
      tag: { value: "" },
      ret: { value: "html" },
      as: { value: "single" },
    },
    label: (node) => (node.name as string) || "html",
  }),
  def({
    type: "yaml",
    name: "yaml",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-code",
    defaults: {
      name: { value: "" },
      property: { value: "payload" },
      action: { value: "" },
    },
    label: (node) => (node.name as string) || "yaml",
  }),
  def({
    type: "markdown",
    name: "markdown",
    category: "parser",
    color: "#c0edc0",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-code",
    defaults: {
      name: { value: "" },
      property: { value: "payload" },
      outproperty: { value: "payload" },
    },
    label: (node) => (node.name as string) || "markdown",
  }),
];

// ---------------------------------------------------------------------------
// Sequence
// ---------------------------------------------------------------------------

const sequenceNodes: NodeDefinition[] = [
  def({
    type: "split",
    name: "split",
    category: "sequence",
    color: "#87a669",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-scissors",
    defaults: {
      name: { value: "" },
      splt: { value: "\\n" },
      spltType: { value: "str" },
      arraySplt: { value: 1 },
      arraySpltType: { value: "len" },
      stream: { value: false },
      addname: { value: "" },
      property: { value: "" },
    },
    label: (node) => (node.name as string) || "split",
  }),
  def({
    type: "join",
    name: "join",
    category: "sequence",
    color: "#87a669",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-object-group",
    defaults: {
      name: { value: "" },
      mode: { value: "auto" },
      build: { value: "array" },
      property: { value: "" },
      propertyType: { value: "msg" },
      key: { value: "topic" },
      joiner: { value: "\\n" },
      joinerType: { value: "str" },
      accumulate: { value: false },
      timeout: { value: "" },
      count: { value: "" },
    },
    label: (node) => (node.name as string) || "join",
  }),
  def({
    type: "sort",
    name: "sort",
    category: "sequence",
    color: "#87a669",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-sort-amount-down",
    defaults: {
      name: { value: "" },
      order: { value: "ascending" },
      as_num: { value: false },
      targetName: { value: "payload" },
      targetType: { value: "msg" },
      msgKey: { value: "" },
      msgKeyType: { value: "elem" },
      seqKey: { value: "payload" },
      seqKeyType: { value: "msg" },
    },
    label: (node) => (node.name as string) || "sort",
  }),
  def({
    type: "batch",
    name: "batch",
    category: "sequence",
    color: "#87a669",
    inputs: 1,
    outputs: 1,
    icon: "fa-solid fa-layer-group",
    defaults: {
      name: { value: "" },
      mode: { value: "count" },
      count: { value: "10" },
      overlap: { value: "0" },
      interval: { value: "1" },
      allowEmptySequence: { value: false },
      topics: { value: [] },
      topic: { value: "payload" },
      topicType: { value: "msg" },
    },
    label: (node) => (node.name as string) || "batch",
  }),
];

// ---------------------------------------------------------------------------
// Registration helper
// ---------------------------------------------------------------------------

const allBuiltinNodes = [
  ...commonNodes,
  ...functionNodes,
  ...networkNodes,
  ...storageNodes,
  ...parserNodes,
  ...sequenceNodes,
];

/**
 * Register all built-in Node-RED node types into the global registry.
 */
export function registerBuiltinNodes(): void {
  for (const nodeDef of allBuiltinNodes) {
    nodeRegistry.registerType(nodeDef.type, nodeDef);
  }
}

/**
 * Return the list of all built-in definitions (useful for palette rendering).
 */
export function getBuiltinNodeList(): NodeDefinition[] {
  return allBuiltinNodes;
}
