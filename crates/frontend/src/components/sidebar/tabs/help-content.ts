/**
 * Help content for built-in Node-RED node types.
 *
 * Each entry contains documentation sections displayed in the Help tab when a
 * node of that type is selected.  Content can contain basic HTML (inline tags
 * only) for formatting.
 *
 * This static map can be replaced later with dynamic help content fetched from
 * the runtime API.
 */

export interface HelpEntry {
  /** Short description of what the node does. */
  description: string;
  /** Key property names with type annotations. */
  properties?: { name: string; type: string; description: string }[];
  /** Output port descriptions. */
  outputs?: { name: string; type: string; description: string }[];
  /** Extended usage details (may contain HTML). */
  details?: string;
  /** Links / references (label + URL). */
  references?: { label: string; url?: string }[];
}

// ---------------------------------------------------------------------------
// Built-in node help content
// ---------------------------------------------------------------------------

export const helpContentMap: Record<string, HelpEntry> = {
  inject: {
    description:
      "Injects a message into a flow either manually or at regular intervals. The message payload can be a variety of types, including strings, JavaScript objects, the current time, or a JSONata expression result.",
    properties: [
      {
        name: "payload",
        type: "string | number | boolean | json | date | jsonata",
        description:
          "The payload of the message to inject. The type determines how the value is interpreted.",
      },
      {
        name: "topic",
        type: "string",
        description: "An optional MQTT topic to set on the message.",
      },
      {
        name: "repeat",
        type: "string",
        description:
          "The interval at which to inject the message. Can be an interval in seconds, a specific time, or based on days/times.",
      },
      {
        name: "once",
        type: "boolean",
        description:
          "Whether to inject the message once when the flow is started.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The configured payload of the output message.",
      },
    ],
    details: `<p>The Inject node can initiate a flow with a specific payload value.
The default payload is a timestamp of the current time in milliseconds since epoch.</p>
<p>When using JSONata to set the payload, the expression is evaluated against the
currently blank message, so only functions like <code>$now()</code> are useful.</p>
<p>The node also supports a <b>once</b> option to automatically inject once when
flows are started, and a <b>repeat</b> option to inject at regular intervals.</p>`,
    references: [
      { label: "JSONata", url: "https://jsonata.org/" },
      { label: "JavaScript", url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript" },
    ],
  },

  debug: {
    description:
      "Used to display messages in the debug sidebar tab and optionally in the system console. By default, it displays the msg.payload, but can be configured to display any message property, the full message, or a JSONata expression result.",
    properties: [
      {
        name: "output",
        type: "string",
        description:
          "Where to display the debug output: debug tab, console, or both.",
      },
      {
        name: "to",
        type: "string",
        description:
          "The message property to display. Default is msg.payload.",
      },
    ],
    outputs: [],
    details: `<p>The Debug node logs selected message properties in the debug sidebar.
By default, it displays <code>msg.payload</code>, but can be set to display
any property, the complete message object, or the result of a JSONata expression.</p>
<p>Debug messages can also be sent to the system console by enabling that option.</p>
<p>The sidebar displays the timestamp, the node that generated the message, and the
message content. You can filter messages by node using the filter buttons.</p>`,
    references: [],
  },

  function: {
    description:
      "A JavaScript function block to run against the messages being received by the node. The messages are provided as a JavaScript object called `msg` and can modify the message before returning it.",
    properties: [
      {
        name: "func",
        type: "string",
        description:
          "The JavaScript code to execute when the node receives a message.",
      },
      {
        name: "outputs",
        type: "number",
        description: "The number of output ports for this node.",
      },
      {
        name: "timeout",
        type: "number",
        description:
          "Maximum time (seconds) the function is allowed to run before being forcefully terminated.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description:
          "The return value(s) from the function. Can return one or multiple messages.",
      },
    ],
    details: `<p>The Function node allows you to run arbitrary JavaScript code against
the messages passing through it.</p>
<p>The function receives a <code>msg</code> object and must return a message object
(or an array of message objects) to pass on to the next node in the flow.</p>
<p>The function has access to the following special variables:</p>
<ul>
<li><code>msg</code> - The message object passed in.</li>
<li><code>node</code> - The node object.</li>
<li><code>context</code> - The node's context.</li>
<li><code>flow</code> - The flow-level context.</li>
<li><code>global</code> - The global context.</li>
<li><code>RED</code> - The Node-RED runtime API.</li>
</ul>
<p>For multiple outputs, return an array of arrays, where each inner array contains
the messages for the corresponding output port.</p>`,
    references: [
      { label: "Writing Functions", url: "https://nodered.org/docs/writing-functions" },
    ],
  },

  change: {
    description:
      "Set, change, delete, or move properties of a message, flow context, or global context without requiring a Function node.",
    properties: [
      {
        name: "rules",
        type: "array",
        description:
          "An array of rules defining what changes to make. Each rule can set, change, delete, or move a property.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The modified message.",
      },
    ],
    details: `<p>The Change node provides a set of operations for modifying message properties
and context values without needing to write any code.</p>
<p>Available operations:</p>
<ul>
<li><b>Set</b> - Set a property to a value, another property, or a JSONata expression result.</li>
<li><b>Change</b> - Search and replace parts of a string property.</li>
<li><b>Delete</b> - Remove a property entirely.</li>
<li><b>Move</b> - Move a property to a new location.</li>
</ul>
<p>Properties can be in <code>msg</code>, flow context, or global context.</p>`,
    references: [
      { label: "JSONata", url: "https://jsonata.org/" },
    ],
  },

  switch: {
    description:
      "Route messages to different output ports based on their property values. Supports multiple rules that can test against a range of conditions.",
    properties: [
      {
        name: "property",
        type: "string",
        description:
          "The message property to evaluate. Can also be a JSONata expression.",
      },
      {
        name: "rules",
        type: "array",
        description:
          "The set of rules to test the property value against. Each rule corresponds to an output port.",
      },
      {
        name: "checkall",
        type: "boolean",
        description:
          "If true, all rules are evaluated. If false, only the first matching rule triggers.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description:
          "Messages are routed to the output corresponding to the first matching rule.",
      },
    ],
    details: `<p>The Switch node evaluates the configured rules against the specified
message property and routes the message to the matching output port.</p>
<p>Supported comparison types include:</p>
<ul>
<li>Equal to / Not equal to</li>
<li>Greater than / Less than</li>
<li>Contains / Matches regex</li>
<li>Is true / Is false / Is null / Is not null</li>
<li>Is between / Has a property</li>
</ul>
<p>A 'otherwise' output port catches messages that match none of the rules.</p>`,
    references: [
      { label: "JSONata", url: "https://jsonata.org/" },
    ],
  },

  template: {
    description:
      "Sets a message property based on a provided template. Uses the Mustache templating format to generate the output from the incoming message.",
    properties: [
      {
        name: "field",
        type: "string",
        description: "The message property to set with the template output.",
      },
      {
        name: "template",
        type: "string",
        description: "The Mustache template string to render.",
      },
      {
        name: "syntax",
        type: "string",
        description: "The template syntax to use: mustache, plain, or jsonata.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "string",
        description: "The rendered template output.",
      },
    ],
    details: `<p>The Template node generates text using a template. By default it uses
the <b>Mustache</b> templating language, but can also be set to plain text mode
or JSONata.</p>
<p>When using Mustache, the template can reference any property of the incoming
message using <code>{{property}}</code> syntax.</p>`,
    references: [
      { label: "Mustache", url: "https://mustache.github.io/" },
    ],
  },

  "http request": {
    description:
      "Sends HTTP requests and returns the response. Supports GET, POST, PUT, DELETE, PATCH, and HEAD methods. Can handle various content types and authentication.",
    properties: [
      {
        name: "method",
        type: "string",
        description:
          "The HTTP method to use: GET, POST, PUT, DELETE, PATCH, or HEAD.",
      },
      {
        name: "url",
        type: "string",
        description: "The URL for the request. Can be set in the node or via msg.url.",
      },
      {
        name: "headers",
        type: "object",
        description:
          "HTTP headers to include in the request. Can be set via msg.headers.",
      },
      {
        name: "payload",
        type: "varies",
        description: "The body of the request for POST/PUT/PATCH methods.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "string | object",
        description: "The response body. Parsed as JSON if the content-type indicates it.",
      },
      {
        name: "statusCode",
        type: "number",
        description: "The HTTP status code of the response.",
      },
      {
        name: "headers",
        type: "object",
        description: "The response headers.",
      },
    ],
    details: `<p>The HTTP Request node provides a flexible way to make HTTP requests.
The URL and method can be configured in the node, or set dynamically via the
incoming message.</p>
<p>Set <code>msg.payload</code> for the request body and <code>msg.headers</code>
for custom headers.</p>
<p>The node supports basic authentication, digest authentication, and OAuth2.
It can also handle TLS/SSL connections with custom certificates.</p>`,
    references: [
      { label: "HTTP Methods", url: "https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods" },
    ],
  },

  "http in": {
    description:
      "Creates an HTTP endpoint that can be used to receive HTTP requests and generate responses. Works in conjunction with the HTTP Response node.",
    properties: [
      {
        name: "method",
        type: "string",
        description: "The HTTP method to listen for: GET, POST, PUT, DELETE, or PATCH.",
      },
      {
        name: "url",
        type: "string",
        description:
          "The URL path for the endpoint, relative to the Node-RED root. Supports path parameters like /:id.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description:
          "For GET requests, the query parameters. For POST/PUT, the request body.",
      },
    ],
    details: `<p>The HTTP In node creates an HTTP endpoint that listens for incoming requests.
Combined with the HTTP Response node, you can build REST APIs and webhooks.</p>
<p>The URL path can include named parameters using <code>:param</code> syntax,
which are available in <code>msg.req.params</code>.</p>`,
    references: [],
  },

  "http response": {
    description:
      "Sends a response back to an HTTP request received by an HTTP In node. Sets the status code, headers, and response body.",
    properties: [
      {
        name: "statusCode",
        type: "number",
        description:
          "The HTTP status code for the response. Defaults to 200.",
      },
      {
        name: "headers",
        type: "object",
        description: "HTTP headers to include in the response.",
      },
    ],
    outputs: [],
    details: `<p>The HTTP Response node sends responses to requests received by the HTTP In node.
The status code defaults to 200 if not set.</p>`,
    references: [],
  },

  delay: {
    description:
      "Delays messages passing through the node, or limits the rate at which they pass through. Supports fixed delays, random delays, and rate limiting.",
    properties: [
      {
        name: "pauseType",
        type: "string",
        description:
          "Type of delay: delay, rate, random, or queue.",
      },
      {
        name: "timeout",
        type: "string",
        description:
          "The delay duration in seconds, milliseconds, minutes, or hours.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The delayed message.",
      },
    ],
    details: `<p>The Delay node can delay messages by a fixed amount, a random amount,
or rate-limit them to a specified number per time period.</p>`,
    references: [],
  },

  trigger: {
    description:
      "Sends a message, then optionally waits for a reset message before sending a second message. Useful for creating watchdog or timeout flows.",
    properties: [
      {
        name: "op1",
        type: "string",
        description: "The first message payload to send.",
      },
      {
        name: "op2",
        type: "string",
        description: "The second message payload to send after the delay.",
      },
      {
        name: "duration",
        type: "string",
        description:
          "The time to wait before sending the second message.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description:
          "First sends op1, then sends op2 after the configured duration (if not reset).",
      },
    ],
    details: `<p>The Trigger node sends a message when triggered and can optionally send
a second message after a delay. If a reset message is received before the delay
expires, the second message is not sent.</p>`,
    references: [],
  },

  comment: {
    description:
      "A node you can use to add comments to your flows. Does not affect the flow execution. Useful for documentation and flow organization.",
    properties: [],
    outputs: [],
    details: `<p>The Comment node is purely for documentation purposes. It has no inputs
or outputs and does not process any messages. Use it to annotate and explain
your flows.</p>`,
    references: [],
  },

  "link in": {
    description:
      "Creates a virtual wire between flows. Messages arriving at a Link In node will be sent to all Link Out nodes that share the same name.",
    properties: [],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The message received from the linked flow.",
      },
    ],
    details: `<p>The Link In/Out nodes create virtual wires between flows without
requiring a physical connection on the canvas.</p>`,
    references: [],
  },

  "link out": {
    description:
      "Sends messages to all Link In nodes that share the same name, creating virtual wires between flows.",
    properties: [],
    outputs: [],
    details: `<p>The Link Out node sends received messages to all connected Link In nodes.
Links can connect across different tabs in the editor.</p>`,
    references: [],
  },

  catch: {
    description:
      "Catches errors thrown by nodes in the same tab. When a node throws an error, the Catch node receives a message with the error details.",
    properties: [
      {
        name: "scope",
        type: "array",
        description: "Optional list of node IDs to catch errors from.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "object",
        description:
          "An error object containing the error message and source node information.",
      },
    ],
    details: `<p>The Catch node catches errors thrown by nodes on the same flow tab.
If configured to catch 'all' errors, it will catch from all nodes. Otherwise,
it can be scoped to specific nodes.</p>`,
    references: [],
  },

  status: {
    description:
      "Captures status messages from other nodes on the same tab, allowing you to create custom status handling.",
    properties: [
      {
        name: "scope",
        type: "array",
        description: "Optional list of node IDs to capture status from.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "object",
        description:
          "The status message object with text, fill, and shape properties.",
      },
    ],
    details: `<p>The Status node receives status updates from other nodes in the same flow.
This allows creating custom visual feedback or logging based on node statuses.</p>`,
    references: [],
  },

  complete: {
    description:
      "Fires when a node finishes processing a message. Can be used to track when specific nodes have completed their work.",
    properties: [
      {
        name: "scope",
        type: "array",
        description: "List of node IDs to monitor for completion.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The message that was being processed when the node completed.",
      },
    ],
    details: `<p>The Complete node fires when a watched node finishes processing a message.
This is useful for creating flow completion tracking and error handling patterns.</p>`,
    references: [],
  },

  exec: {
    description:
      "Runs a system command and returns its output. Supports three modes: spawn, exec, and fork.",
    properties: [
      {
        name: "command",
        type: "string",
        description: "The system command to execute.",
      },
      {
        name: "addpay",
        type: "boolean",
        description: "Whether to append msg.payload to the command.",
      },
    ],
    outputs: [
      {
        name: "stdout",
        type: "string",
        description: "The standard output from the command.",
      },
      {
        name: "stderr",
        type: "string",
        description: "The standard error output from the command.",
      },
      {
        name: "returnCode",
        type: "number",
        description: "The return code from the command.",
      },
    ],
    details: `<p>The Exec node runs system commands. In exec mode, it waits for the command
to complete. In spawn mode, it streams stdout and stderr as the command runs.</p>`,
    references: [],
  },

  join: {
    description:
      "Joins sequences of messages into a single message. Can combine messages in various ways: manually, by counting, by time, or by grouping.",
    properties: [
      {
        name: "mode",
        type: "string",
        description:
          "The join mode: auto, manual, reduce, or custom.",
      },
      {
        name: "build",
        type: "string",
        description:
          "How to combine the messages: as an array, object, merged object, or key/value.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "array | object",
        description: "The combined result of the joined messages.",
      },
    ],
    details: `<p>The Join node combines multiple messages into a single message.
It can collect a specified number of messages, wait for a specified time, or
combine based on message properties.</p>`,
    references: [],
  },

  split: {
    description:
      "Splits a message into a sequence of messages. Can split arrays, strings, and objects.",
    properties: [
      {
        name: "splt",
        type: "string",
        description: "The string to split on (for string splitting).",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "Each individual element from the split source.",
      },
    ],
    details: `<p>The Split node splits a message payload into multiple messages.
Arrays are split into individual elements, strings are split by a separator,
and objects are split into key/value pairs.</p>`,
    references: [],
  },

  sort: {
    description:
      "Sorts message sequences based on a property value or JSONata expression.",
    properties: [
      {
        name: "as_num",
        type: "boolean",
        description: "Whether to sort numerically instead of alphabetically.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "varies",
        description: "The sorted sequence of messages.",
      },
    ],
    details: `<p>The Sort node sorts a sequence of messages. The sort key can be a message
property or the result of a JSONata expression.</p>`,
    references: [],
  },

  batch: {
    description:
      "Creates batches of messages to release in groups. Useful for grouping messages for bulk processing.",
    properties: [
      {
        name: "mode",
        type: "string",
        description:
          "The batching mode: count, interval, or concat.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "array",
        description: "A batch of messages.",
      },
    ],
    details: `<p>The Batch node groups messages into batches. It can group by message count,
time interval, or by concatenating sequences.</p>`,
    references: [],
  },

  mqtt: {
    description:
      "Connects to an MQTT broker and can publish and subscribe to topics.",
    properties: [
      {
        name: "topic",
        type: "string",
        description: "The MQTT topic to subscribe to or publish to.",
      },
      {
        name: "broker",
        type: "string",
        description: "The MQTT broker configuration node reference.",
      },
    ],
    outputs: [
      {
        name: "payload",
        type: "string | buffer",
        description: "The message payload received from the MQTT broker.",
      },
    ],
    details: `<p>The MQTT node connects to an MQTT broker. It can subscribe to topics
and publish messages to topics.</p>`,
    references: [
      { label: "MQTT Protocol", url: "https://mqtt.org/" },
    ],
  },
};

// ---------------------------------------------------------------------------
// General help content (shown when no node is selected)
// ---------------------------------------------------------------------------

export const generalHelpContent: HelpEntry = {
  description:
    "Node-RED is a flow-based programming tool for wiring together hardware devices, APIs, and online services.",
  properties: [],
  outputs: [],
  details: `<h3>Keyboard Shortcuts</h3>
<ul>
<li><b>Ctrl-Space</b> - Open quick-add dialog</li>
<li><b>Ctrl-Z</b> - Undo</li>
<li><b>Ctrl-Y</b> - Redo</li>
<li><b>Ctrl-A</b> - Select all nodes</li>
<li><b>Delete / Backspace</b> - Delete selected</li>
<li><b>Ctrl-C</b> - Copy selected</li>
<li><b>Ctrl-V</b> - Paste</li>
<li><b>Ctrl-X</b> - Cut selected</li>
<li><b>Ctrl-D</b> - Deploy</li>
<li><b>?</b> - Show keyboard shortcuts</li>
</ul>
<h3>Getting Started</h3>
<p>Select a node in the workspace to view its documentation here. Drag nodes from the palette onto the workspace and connect them with wires to create flows.</p>
<p>Double-click a node to edit its properties. Click Deploy to activate your flow.</p>`,
  references: [
    { label: "Node-RED Documentation", url: "https://nodered.org/docs/" },
    { label: "Flow Library", url: "https://flows.nodered.org/" },
  ],
};

/**
 * Look up help content for a node type. Returns the general help content if
 * the type is not found in the map.
 */
export function getHelpForType(
  type: string | null | undefined,
): HelpEntry {
  if (type && helpContentMap[type]) {
    return helpContentMap[type];
  }
  return generalHelpContent;
}
