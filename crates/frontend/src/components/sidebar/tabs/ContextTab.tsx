/**
 * ContextTab -- Flow and global context data browser.
 *
 * Shows two expandable sections ("Flow Context" and "Global Context") that
 * display key-value pairs as a tree. Objects and arrays are recursively
 * expandable. Values are rendered with type-appropriate formatting:
 *   - strings  -> quoted, green
 *   - numbers  -> as-is, blue
 *   - booleans -> colored (true = green, false = red)
 *   - null     -> grey italic
 *   - objects/arrays -> expandable tree node
 *
 * Toolbar: Refresh (reload from API), Copy All (JSON to clipboard).
 */

import { useCallback, useEffect, useState } from "react";
import { RefreshCw, Copy, ChevronDown, ChevronRight } from "lucide-react";
import { useContextStore } from "../../../store/context-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TreeNodeProps {
  /** Property key (omitted for root-level array items). */
  k?: string;
  /** The value to render. */
  value: unknown;
  /** Nesting depth for indentation. */
  depth: number;
}

interface SectionProps {
  label: string;
  data: Record<string, unknown> | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Check if a value is a plain object (not null, not array). */
function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** Check if a value is an array. */
function isArray(v: unknown): v is unknown[] {
  return Array.isArray(v);
}

/** Check if a value is expandable (object or non-empty array). */
function isExpandable(v: unknown): boolean {
  if (isPlainObject(v)) return Object.keys(v).length > 0;
  if (isArray(v)) return v.length > 0;
  return false;
}

/** Copy text to clipboard, gracefully handling failures. */
async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Value rendering
// ---------------------------------------------------------------------------

/** Render a primitive value with type-appropriate colour. */
function ValueSpan({ value }: { value: unknown }) {
  if (value === null) {
    return <span className="text-gray-400 italic">null</span>;
  }
  if (value === undefined) {
    return <span className="text-gray-400 italic">undefined</span>;
  }
  switch (typeof value) {
    case "string":
      return (
        <span className="text-green-600 dark:text-green-400">
          &quot;{value}&quot;
        </span>
      );
    case "number":
      return (
        <span className="text-blue-600 dark:text-blue-400">{String(value)}</span>
      );
    case "boolean":
      return (
        <span
          className={
            value
              ? "text-green-600 dark:text-green-400"
              : "text-red-600 dark:text-red-400"
          }
        >
          {String(value)}
        </span>
      );
    default:
      return <span className="text-gray-500">{String(value)}</span>;
  }
}

// ---------------------------------------------------------------------------
// TreeNode -- recursive expandable tree row
// ---------------------------------------------------------------------------

function TreeNode({ k, value, depth }: TreeNodeProps) {
  const [expanded, setExpanded] = useState(false);
  const expandable = isExpandable(value);

  const toggle = useCallback(() => {
    if (expandable) setExpanded((prev) => !prev);
  }, [expandable]);

  // -- label with key name --------------------------------------------------

  const indent = depth * 16;

  // Primitive (leaf) row
  if (!expandable) {
    return (
      <div
        className="flex items-start gap-1 py-0.5 px-1 text-xs font-mono hover:bg-gray-50 dark:hover:bg-gray-700/40 rounded"
        style={{ paddingLeft: indent + 4 }}
      >
        {/* spacer where chevron would be */}
        <span className="w-4 shrink-0" />
        {k !== undefined && (
          <span className="text-gray-700 dark:text-gray-300">{k}: </span>
        )}
        <ValueSpan value={value} />
      </div>
    );
  }

  // Expandable row
  const childEntries = isPlainObject(value)
    ? Object.entries(value)
    : isArray(value)
      ? value.map((v, i) => [String(i), v] as [string, unknown])
      : [];

  return (
    <div>
      {/* Current row */}
      <button
        type="button"
        onClick={toggle}
        className="flex items-start gap-1 py-0.5 px-1 text-xs font-mono w-full text-left hover:bg-gray-50 dark:hover:bg-gray-700/40 rounded"
        style={{ paddingLeft: indent + 4 }}
        data-testid={`context-tree-toggle-${k ?? "root"}`}
      >
        <span className="w-4 shrink-0 mt-0.5">
          {expanded ? (
            <ChevronDown className="w-3 h-3 text-gray-400" />
          ) : (
            <ChevronRight className="w-3 h-3 text-gray-400" />
          )}
        </span>
        {k !== undefined && (
          <span className="text-gray-700 dark:text-gray-300">{k}: </span>
        )}
        <span className="text-gray-400">
          {isPlainObject(value)
            ? `{${Object.keys(value).length}}`
            : `[${(value as unknown[]).length}]`}
        </span>
      </button>

      {/* Children */}
      {expanded &&
        childEntries.map(([ck, cv]) => (
          <TreeNode key={ck} k={ck} value={cv} depth={depth + 1} />
        ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section -- expandable "Flow Context" / "Global Context" block
// ---------------------------------------------------------------------------

function Section({ label, data }: SectionProps) {
  const [open, setOpen] = useState(true);
  const entries = data ? Object.entries(data) : [];

  return (
    <div className="border-b border-gray-200 dark:border-gray-700">
      {/* Section header */}
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex items-center gap-1.5 w-full px-3 py-2 text-xs font-semibold text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50"
        data-testid={`context-section-${label.toLowerCase().replace(/\s/g, "-")}`}
      >
        {open ? (
          <ChevronDown className="w-3.5 h-3.5" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5" />
        )}
        {label}
      </button>

      {/* Section body */}
      {open && (
        <div className="pb-2">
          {entries.length === 0 ? (
            <p className="text-xs text-gray-400 px-7 py-1 italic">
              No data
            </p>
          ) : (
            entries.map(([key, val]) => (
              <TreeNode key={key} k={key} value={val} depth={0} />
            ))
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// ContextTab (main export)
// ---------------------------------------------------------------------------

export function ContextTab() {
  const flowContext = useContextStore((s) => s.flowContext);
  const globalContext = useContextStore((s) => s.globalContext);
  const isLoading = useContextStore((s) => s.isLoading);
  const refresh = useContextStore((s) => s.refresh);

  const [copied, setCopied] = useState(false);

  // Fetch context on first mount
  useEffect(() => {
    const state = useContextStore.getState();
    if (state.flowContext === null && state.globalContext === null) {
      refresh();
    }
  }, [refresh]);

  // -- Copy All handler -----------------------------------------------------

  const handleCopyAll = useCallback(async () => {
    const payload = {
      flow: flowContext ?? {},
      global: globalContext ?? {},
    };
    const ok = await copyToClipboard(JSON.stringify(payload, null, 2));
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [flowContext, globalContext]);

  // -- Empty state ----------------------------------------------------------

  const hasFlow = flowContext !== null && Object.keys(flowContext).length > 0;
  const hasGlobal =
    globalContext !== null && Object.keys(globalContext).length > 0;

  const isEmpty = !isLoading && !hasFlow && !hasGlobal;

  return (
    <div
      className="flex flex-col flex-1 overflow-hidden"
      data-testid="sidebar-tab-content-context"
    >
      {/* Toolbar */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750 shrink-0">
        <button
          type="button"
          onClick={refresh}
          disabled={isLoading}
          className="flex items-center gap-1 px-2 py-1 text-xs text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600 rounded disabled:opacity-50"
          data-testid="context-refresh-btn"
        >
          <RefreshCw
            className={`w-3.5 h-3.5 ${isLoading ? "animate-spin" : ""}`}
          />
          Refresh
        </button>

        <button
          type="button"
          onClick={handleCopyAll}
          className="flex items-center gap-1 px-2 py-1 text-xs text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600 rounded"
          data-testid="context-copy-btn"
        >
          <Copy className="w-3.5 h-3.5" />
          {copied ? "Copied!" : "Copy All"}
        </button>
      </div>

      {/* Loading */}
      {isLoading && (
        <div className="flex items-center justify-center py-8">
          <RefreshCw className="w-5 h-5 text-gray-400 animate-spin" />
          <span className="ml-2 text-xs text-gray-400">Loading...</span>
        </div>
      )}

      {/* Empty state */}
      {isEmpty && (
        <div className="flex items-center justify-center flex-1 p-4">
          <p className="text-sm text-gray-400">No context data</p>
        </div>
      )}

      {/* Context tree */}
      {!isLoading && !isEmpty && (
        <div className="flex-1 overflow-auto py-1">
          <Section label="Flow Context" data={flowContext ?? {}} />
          <Section label="Global Context" data={globalContext ?? {}} />
        </div>
      )}
    </div>
  );
}
