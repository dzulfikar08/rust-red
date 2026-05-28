import {
  useState,
  useCallback,
  useMemo,
  useRef,
  useEffect,
} from "react";
import {
  Trash2,
  Search,
  ChevronDown,
  ChevronRight,
  Copy,
  Check,
} from "lucide-react";
import {
  useDebugStore,
  selectFilteredMessages,
  selectUniqueFlows,
  type DebugMessage,
} from "../../store/debug-store";
import { commsClient } from "../../ws/comms";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleString();
}

/** Truncate a value preview to maxLength characters. */
function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength) + "...";
}

/** Get a short preview of msg.payload for display. */
function payloadPreview(msg: DebugMessage): string {
  const payload = msg.msg?.payload;
  if (payload === undefined) return "";
  if (typeof payload === "string") return truncate(payload, 80);
  if (typeof payload === "object" && payload !== null) {
    if (Array.isArray(payload)) return `Array[${payload.length}]`;
    const keys = Object.keys(payload as Record<string, unknown>);
    return `{ object with ${keys.length} key${keys.length !== 1 ? "s" : ""} }`;
  }
  return truncate(String(payload), 80);
}

// ---------------------------------------------------------------------------
// Tree View
// ---------------------------------------------------------------------------

function TreeValue({ value, depth }: { value: unknown; depth: number }) {
  if (value === null) {
    return <span className="text-gray-400 italic">null</span>;
  }
  if (value === undefined) {
    return <span className="text-gray-400 italic">undefined</span>;
  }
  if (typeof value === "string") {
    return <span className="text-green-600 dark:text-green-400">&quot;{value}&quot;</span>;
  }
  if (typeof value === "number") {
    return <span className="text-purple-600 dark:text-purple-400">{value}</span>;
  }
  if (typeof value === "boolean") {
    return <span className="text-orange-600 dark:text-orange-400">{String(value)}</span>;
  }
  if (typeof value === "object") {
    return <TreeNode label="" value={value} depth={depth} />;
  }
  return <span>{String(value)}</span>;
}

function TreeNode({
  label,
  value,
  depth,
}: {
  label: string;
  value: unknown;
  depth: number;
}) {
  const [expanded, setExpanded] = useState(depth < 1);

  if (value === null || value === undefined || typeof value !== "object") {
    return (
      <div className="flex gap-1" style={{ paddingLeft: depth * 12 }}>
        {label && (
          <span className="text-gray-500 dark:text-gray-400">{label}: </span>
        )}
        <TreeValue value={value} depth={depth} />
      </div>
    );
  }

  const isArr = Array.isArray(value);
  const entries = isArr
    ? (value as unknown[]).map((v, i) => [String(i), v] as [string, unknown])
    : Object.entries(value as Record<string, unknown>);

  const summary = isArr
    ? `Array[${entries.length}]`
    : `{${entries.length} keys}`;

  return (
    <div>
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1 hover:bg-gray-50 dark:hover:bg-gray-700/50 rounded px-0.5"
        style={{ paddingLeft: depth * 12 }}
      >
        {expanded ? (
          <ChevronDown size={10} className="text-gray-400 flex-shrink-0" />
        ) : (
          <ChevronRight size={10} className="text-gray-400 flex-shrink-0" />
        )}
        <span className="text-gray-500 dark:text-gray-400">
          {label ? `${label}: ` : ""}
          <span className="text-gray-400">{summary}</span>
        </span>
      </button>
      {expanded && (
        <div>
          {entries.map(([key, val]) => (
            <TreeNode key={key} label={key} value={val} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Debug Message Row
// ---------------------------------------------------------------------------

function DebugMessageRow({
  message,
  onCopy,
}: {
  message: DebugMessage;
  onCopy: (text: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const preview = payloadPreview(message);
  const levelColor =
    message.level === "error"
      ? "text-red-600 dark:text-red-400"
      : message.level === "warn"
        ? "text-amber-600 dark:text-amber-400"
        : "text-blue-600 dark:text-blue-400";

  const handleCopy = useCallback(() => {
    onCopy(JSON.stringify(message.msg, null, 2));
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [message.msg, onCopy]);

  return (
    <div
      className="border-b border-gray-100 dark:border-gray-700 last:border-b-0"
      data-testid="debug-message-row"
    >
      {/* Header line */}
      <div className="flex items-center gap-2 px-3 py-1.5">
        {/* Expand toggle */}
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-gray-400 p-0.5 flex-shrink-0"
          data-testid="debug-expand-btn"
        >
          {expanded ? (
            <ChevronDown size={10} />
          ) : (
            <ChevronRight size={10} />
          )}
        </button>

        {/* Timestamp */}
        <span
          className="text-gray-400 text-[10px] flex-shrink-0"
          data-testid="debug-timestamp"
        >
          {formatTimestamp(message.timestamp)}
        </span>

        {/* Source node/flow label */}
        <span className={`text-xs ${levelColor}`} data-testid="debug-source">
          {message.sourceNode?.name || message.topic || "debug"}
          {message.sourceFlow ? `/${message.sourceFlow}` : ""}
        </span>

        {/* Payload preview */}
        {preview && (
          <span
            className="text-gray-600 dark:text-gray-300 text-xs truncate"
            data-testid="debug-preview"
          >
            {preview}
          </span>
        )}

        {/* Copy button */}
        <button
          type="button"
          onClick={handleCopy}
          className="ml-auto p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-600 flex-shrink-0"
          title="Copy msg to clipboard"
          data-testid="debug-copy-btn"
        >
          {copied ? (
            <Check size={10} className="text-green-500" />
          ) : (
            <Copy size={10} className="text-gray-400" />
          )}
        </button>
      </div>

      {/* Expanded tree view */}
      {expanded && (
        <div
          className="px-3 pb-2 font-mono text-[11px]"
          data-testid="debug-tree-view"
        >
          <TreeNode label="msg" value={message.msg} depth={0} />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main DebugPanel Component
// ---------------------------------------------------------------------------

export function DebugPanel() {
  const messages = useDebugStore((s) => s.messages);
  const filter = useDebugStore((s) => s.filter);
  const setFilter = useDebugStore((s) => s.setFilter);
  const flowFilter = useDebugStore((s) => s.flowFilter);
  const setFlowFilter = useDebugStore((s) => s.setFlowFilter);
  const clearMessages = useDebugStore((s) => s.clearMessages);
  const addMessage = useDebugStore((s) => s.addMessage);

  // Derive filtered messages with useMemo to avoid infinite re-renders
  const filteredMessages = useMemo(
    () => selectFilteredMessages(useDebugStore.getState()),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [messages, filter, flowFilter],
  );

  const uniqueFlows = useMemo(
    () => selectUniqueFlows(useDebugStore.getState()),
    [messages],
  );

  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  // ---- WebSocket integration ----
  useEffect(() => {
    const handleDebug = (data: unknown) => {
      try {
        const d = data as Record<string, unknown>;
        addMessage({
          id: String(d.id ?? crypto.randomUUID()),
          timestamp: typeof d.timestamp === "number" ? d.timestamp : Date.now(),
          topic: typeof d.topic === "string" ? d.topic : undefined,
          msg:
            typeof d.msg === "object" && d.msg !== null
              ? (d.msg as Record<string, unknown>)
              : { payload: d.msg },
          sourceNode:
            d.sourceNode &&
            typeof d.sourceNode === "object" &&
            "id" in (d.sourceNode as object)
              ? (d.sourceNode as { id: string; name: string; type: string })
              : undefined,
          sourceFlow:
            typeof d.sourceFlow === "string" ? d.sourceFlow : undefined,
          level:
            d.level === "debug" ||
            d.level === "warn" ||
            d.level === "error" ||
            d.level === "info"
              ? d.level
              : undefined,
        });
      } catch {
        // Ignore malformed debug data
      }
    };

    const unsub = commsClient.on("debug", handleDebug);
    commsClient.subscribe("debug");
    return () => {
      unsub();
    };
  }, [addMessage]);

  // ---- Auto-scroll ----
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
  }, [filteredMessages.length, autoScroll]);

  // ---- Detect manual scroll (pause auto-scroll when user scrolls down) ----
  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;
    // Auto-scroll is "pinned to top". If user scrolls away from top, disable.
    if (scrollRef.current.scrollTop > 30) {
      setAutoScroll(false);
    }
  }, []);

  const handleCopy = useCallback((text: string) => {
    navigator.clipboard.writeText(text).catch(() => {});
  }, []);

  const selectedFlowLabel = useMemo(() => {
    if (flowFilter === null) return "All flows";
    return flowFilter;
  }, [flowFilter]);

  return (
    <div
      className="flex flex-col h-full bg-white dark:bg-gray-800 overflow-hidden"
      data-testid="debug-panel"
    >
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
        {/* Search / filter input */}
        <div className="flex items-center gap-1 bg-gray-100 dark:bg-gray-700 rounded px-2 py-0.5 flex-1 min-w-0">
          <Search size={12} className="text-gray-400 flex-shrink-0" />
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter messages..."
            className="bg-transparent text-xs text-gray-700 dark:text-gray-200 placeholder-gray-400 outline-none w-full"
            data-testid="debug-filter-input"
          />
        </div>

        {/* Flow filter dropdown */}
        <div className="relative">
          <select
            value={flowFilter ?? ""}
            onChange={(e) =>
              setFlowFilter(e.target.value === "" ? null : e.target.value)
            }
            className="appearance-none bg-gray-100 dark:bg-gray-700 text-xs text-gray-700 dark:text-gray-200 rounded px-2 py-0.5 pr-5 outline-none cursor-pointer border-none"
            data-testid="debug-flow-filter"
          >
            <option value="">{selectedFlowLabel}</option>
            {uniqueFlows
              .filter((f) => f !== flowFilter)
              .map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
          </select>
          <ChevronDown
            size={10}
            className="absolute right-1.5 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
          />
        </div>

        {/* Message count */}
        <span className="text-[10px] text-gray-400 flex-shrink-0" data-testid="debug-count">
          {filteredMessages.length > 0 && (
            <>
              {filteredMessages.length}
              {filter || flowFilter ? `/${messages.length}` : ""}
            </>
          )}
        </span>

        {/* Clear button */}
        <button
          type="button"
          onClick={clearMessages}
          className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors flex-shrink-0"
          title="Clear all debug messages"
          data-testid="debug-clear-btn"
        >
          <Trash2 size={12} className="text-gray-500" />
        </button>
      </div>

      {/* Message list */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto font-mono text-xs"
        data-testid="debug-message-list"
      >
        {filteredMessages.length === 0 ? (
          <div className="p-4 text-gray-400 text-center" data-testid="debug-empty">
            No debug messages
          </div>
        ) : (
          filteredMessages.map((msg) => (
            <DebugMessageRow
              key={msg.id}
              message={msg}
              onCopy={handleCopy}
            />
          ))
        )}
      </div>
    </div>
  );
}
