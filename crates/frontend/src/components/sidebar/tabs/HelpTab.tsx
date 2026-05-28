/**
 * HelpTab -- Node documentation panel.
 *
 * Shows help content for the currently selected node type. When no node is
 * selected, displays general help with keyboard shortcuts and getting started
 * information.
 *
 * Content sections: description, properties, outputs, details, references.
 * HTML content in the details section is rendered after sanitization to prevent
 * XSS. The current help content is static and trusted, but sanitization is
 * applied as a defense-in-depth measure.
 */

import { useMemo } from "react";
import { useEditorStore } from "../../../store/editor-store";
import { nodeRegistry } from "../../../red/nodes/registry";
import { getHelpForType, generalHelpContent } from "./help-content";

// ---------------------------------------------------------------------------
// HTML sanitization
// ---------------------------------------------------------------------------

/** Set of allowed HTML tags for help content. */
const ALLOWED_TAGS = new Set([
  "p",
  "b",
  "i",
  "em",
  "strong",
  "ul",
  "ol",
  "li",
  "code",
  "pre",
  "h3",
  "h4",
  "a",
  "br",
  "span",
  "div",
  "table",
  "thead",
  "tbody",
  "tr",
  "th",
  "td",
]);

/** Set of allowed HTML attributes. */
const ALLOWED_ATTRS = new Set(["href", "target", "rel", "class", "dir"]);

/**
 * Basic HTML sanitizer that strips disallowed tags and attributes.
 * For the static help content map this is sufficient; if help content ever
 * comes from the API, consider using DOMPurify instead.
 */
function sanitizeHtml(html: string): string {
  return html.replace(
    /<\/?([a-zA-Z][a-zA-Z0-9]*)\b[^>]*>/g,
    (match, tagName) => {
      const tag = tagName.toLowerCase();

      // Closing tags pass through if the tag is allowed
      if (match.startsWith("</")) {
        return ALLOWED_TAGS.has(tag) ? match : "";
      }

      // Opening tags: filter attributes
      if (!ALLOWED_TAGS.has(tag)) return "";

      // Extract and filter attributes
      return match.replace(
        /([a-zA-Z][a-zA-Z0-9-]*)\s*=\s*("[^"]*"|'[^']*')/g,
        (attrMatch: string, attrName: string) => {
          return ALLOWED_ATTRS.has(attrName.toLowerCase())
            ? attrMatch
            : "";
        },
      );
    },
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Coloured dot indicating the node category colour. */
function CategoryDot({ color }: { color: string | undefined }) {
  return (
    <span
      className="inline-block w-2.5 h-2.5 rounded-full shrink-0"
      style={{ backgroundColor: color ?? "#c0c0c0" }}
      data-testid="help-category-dot"
    />
  );
}

/** Section header used to separate content areas. */
function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mt-4 mb-2">
      {children}
    </h3>
  );
}

/** A single property row in the properties list. */
function PropertyRow({
  name,
  type,
  description,
}: {
  name: string;
  type: string;
  description: string;
}) {
  return (
    <div className="mb-2 last:mb-0">
      <div className="flex items-baseline gap-1.5">
        <span className="inline-block w-1.5 h-1.5 rounded-full bg-blue-400 dark:bg-blue-500 shrink-0 mt-1.5" />
        <code className="text-xs font-semibold text-gray-800 dark:text-gray-200">
          {name}
        </code>
        <span className="text-[10px] text-gray-400 dark:text-gray-500 font-mono">
          ({type})
        </span>
      </div>
      <p className="text-xs text-gray-500 dark:text-gray-400 ml-4 mt-0.5">
        {description}
      </p>
    </div>
  );
}

/** A single output row. */
function OutputRow({
  name,
  type,
  description,
}: {
  name: string;
  type: string;
  description: string;
}) {
  return (
    <div className="mb-2 last:mb-0">
      <div className="flex items-baseline gap-1.5">
        <span className="inline-block w-1.5 h-1.5 rounded-full bg-green-400 dark:bg-green-500 shrink-0 mt-1.5" />
        <code className="text-xs font-semibold text-gray-800 dark:text-gray-200">
          {name}
        </code>
        <span className="text-[10px] text-gray-400 dark:text-gray-500 font-mono">
          ({type})
        </span>
      </div>
      <p className="text-xs text-gray-500 dark:text-gray-400 ml-4 mt-0.5">
        {description}
      </p>
    </div>
  );
}

/** Render sanitized HTML content. */
function HtmlContent({ html }: { html: string }) {
  const sanitized = useMemo(() => sanitizeHtml(html), [html]);
  return (
    <div
      className="prose prose-xs prose-gray dark:prose-invert max-w-none text-xs text-gray-600 dark:text-gray-300 [&_p]:mb-2 [&_p]:text-xs [&_ul]:mb-2 [&_ul]:pl-4 [&_ul]:text-xs [&_li]:mb-1 [&_li]:text-xs [&_code]:text-[11px] [&_code]:bg-gray-100 [&_code]:dark:bg-gray-700 [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_b]:font-semibold [&_h3]:text-xs [&_h3]:font-semibold [&_h3]:text-gray-500 [&_h3]:dark:text-gray-400 [&_h3]:uppercase [&_h3]:tracking-wider [&_h3]:mt-4 [&_h3]:mb-2"
      // eslint-disable-next-line react/no-danger -- HTML is sanitized via sanitizeHtml() above
      dangerouslySetInnerHTML={{ __html: sanitized }}
      data-testid="help-html-content"
    />
  );
}

/** References list with links. */
function ReferencesList({
  references,
}: {
  references: { label: string; url?: string }[];
}) {
  if (references.length === 0) return null;
  return (
    <div data-testid="help-references">
      <SectionHeader>References</SectionHeader>
      <ul className="space-y-1">
        {references.map((ref, i) => (
          <li key={i} className="flex items-center gap-1.5 text-xs">
            <span className="text-gray-300 dark:text-gray-600">
              {"\u2022"}
            </span>
            {ref.url ? (
              <a
                href={ref.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 underline"
              >
                {ref.label}
              </a>
            ) : (
              <span className="text-gray-600 dark:text-gray-400">
                {ref.label}
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function HelpTab() {
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);

  // Resolve help content and node metadata.
  const { help, title, nodeColor, isGeneral } = useMemo(() => {
    if (!selectedNodeId) {
      return {
        help: generalHelpContent,
        title: "Node-RED Help",
        nodeColor: undefined,
        isGeneral: true,
      };
    }

    // Attempt to resolve the type definition from the registry.
    // In this implementation, selectedNodeId may be a node type string
    // used directly for help lookup.
    const def = nodeRegistry.getType(selectedNodeId);
    const help = getHelpForType(selectedNodeId);

    return {
      help,
      title:
        typeof def?.paletteLabel === "string"
          ? def.paletteLabel
          : typeof def?.label === "string"
            ? def.label
            : selectedNodeId,
      nodeColor: def?.color,
      isGeneral: false,
    };
  }, [selectedNodeId]);

  return (
    <div
      className="flex-1 flex flex-col overflow-hidden"
      data-testid="sidebar-tab-content-help"
    >
      {/* Header with node type name and category colour dot */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750 shrink-0">
        <CategoryDot color={nodeColor} />
        <h2
          className="text-sm font-bold text-gray-800 dark:text-gray-100 truncate"
          data-testid="help-title"
        >
          {title}
        </h2>
      </div>

      {/* Scrollable content area */}
      <div className="flex-1 overflow-y-auto px-3 pb-4">
        {/* Description */}
        <p
          className="text-xs text-gray-600 dark:text-gray-300 mt-3 leading-relaxed"
          data-testid="help-description"
        >
          {help.description}
        </p>

        {/* Properties */}
        {help.properties && help.properties.length > 0 && (
          <div data-testid="help-properties">
            <SectionHeader>Properties</SectionHeader>
            {help.properties.map((prop, i) => (
              <PropertyRow key={i} {...prop} />
            ))}
          </div>
        )}

        {/* Outputs */}
        {help.outputs && help.outputs.length > 0 && (
          <div data-testid="help-outputs">
            <SectionHeader>Outputs</SectionHeader>
            {help.outputs.map((out, i) => (
              <OutputRow key={i} {...out} />
            ))}
          </div>
        )}

        {/* Details (HTML content) */}
        {help.details && (
          <div data-testid="help-details">
            <SectionHeader>Details</SectionHeader>
            <HtmlContent html={help.details} />
          </div>
        )}

        {/* No selection message */}
        {isGeneral && (
          <div data-testid="help-no-selection">
            <SectionHeader>No Node Selected</SectionHeader>
            <p className="text-xs text-gray-400 dark:text-gray-500 italic">
              Select a node in the workspace to view its documentation.
            </p>
          </div>
        )}

        {/* References */}
        {help.references && help.references.length > 0 && (
          <ReferencesList references={help.references} />
        )}
      </div>
    </div>
  );
}
