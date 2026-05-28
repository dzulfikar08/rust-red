/**
 * PaletteEditorDialog -- Modal dialog for managing installed node modules.
 *
 * Two tabs:
 *   - Installed: shows currently installed modules (core + community)
 *   - Install: search npm for new modules to install
 *
 * Replaces Node-RED's RED.paletteEditor (~1930 lines).
 */

import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { X, Search, Trash2, Download, Loader2 } from "lucide-react";
import {
  usePaletteEditorStore,
  type InstalledModule,
} from "../../store/palette-editor-store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type PaletteEditorTab = "installed" | "install";

export interface PaletteEditorDialogProps {
  open: boolean;
  onClose: () => void;
  initialTab?: PaletteEditorTab;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function TabButton({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`px-3 py-1.5 text-xs font-medium whitespace-nowrap transition-colors ${
        active
          ? "text-blue-400 border-b-2 border-blue-400"
          : "text-gray-400 hover:text-gray-200 border-b-2 border-transparent"
      }`}
    >
      {label}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Installed Tab
// ---------------------------------------------------------------------------

function InstalledTab() {
  const installed = usePaletteEditorStore((s) => s.installed);
  const removeModule = usePaletteEditorStore((s) => s.removeModule);

  if (installed.length === 0) {
    return (
      <div className="py-6 text-center text-xs text-gray-500">
        No modules installed
      </div>
    );
  }

  return (
    <div className="space-y-1" data-testid="palette-installed-list">
      {installed.map((mod) => (
        <ModuleRow
          key={mod.name}
          module={mod}
          onRemove={mod.local ? undefined : () => removeModule(mod.name)}
        />
      ))}
    </div>
  );
}

function ModuleRow({
  module,
  onRemove,
}: {
  module: InstalledModule;
  onRemove?: () => void;
}) {
  return (
    <div
      className="flex items-start gap-3 rounded border border-[#444] bg-[#333] px-3 py-2"
      data-testid="palette-module-row"
    >
      {/* Module info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span
            className="text-sm font-medium text-gray-100 truncate"
            data-testid="palette-module-name"
          >
            {module.name}
          </span>
          <span className="text-[10px] text-gray-500" data-testid="palette-module-version">
            v{module.version}
          </span>
          {module.local && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded bg-blue-900/40 text-blue-300"
              data-testid="palette-module-badge"
            >
              built-in
            </span>
          )}
        </div>
        <p className="text-xs text-gray-400 mt-0.5 truncate" data-testid="palette-module-desc">
          {module.description}
        </p>
        <p className="text-[10px] text-gray-500 mt-0.5" data-testid="palette-module-nodes">
          {module.nodes.length} node{module.nodes.length !== 1 ? "s" : ""}
        </p>
      </div>

      {/* Remove button (only for non-core modules) */}
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="flex items-center justify-center w-7 h-7 rounded hover:bg-red-900/40 text-gray-400 hover:text-red-400 transition-colors flex-shrink-0"
          title={`Remove ${module.name}`}
          data-testid="palette-module-remove"
        >
          <Trash2 size={14} />
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Install Tab
// ---------------------------------------------------------------------------

function InstallTab() {
  const [query, setQuery] = useState("");
  const available = usePaletteEditorStore((s) => s.available);
  const isLoading = usePaletteEditorStore((s) => s.isLoading);
  const installStatus = usePaletteEditorStore((s) => s.installStatus);
  const searchNpm = usePaletteEditorStore((s) => s.searchNpm);
  const installModule = usePaletteEditorStore((s) => s.installModule);
  const installedNames = usePaletteEditorStore(
    (s) => s.installed.map((m) => m.name).join(","),
  );
  const installedSet = useMemo(
    () => new Set(installedNames.split(",")),
    [installedNames],
  );

  const [installingName, setInstallingName] = useState<string | null>(null);

  const handleSearch = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (query.trim()) {
        searchNpm(query.trim());
      }
    },
    [query, searchNpm],
  );

  const handleInstall = useCallback(
    async (name: string) => {
      setInstallingName(name);
      await installModule(name);
      setInstallingName(null);
    },
    [installModule],
  );

  return (
    <div className="space-y-3">
      {/* Search form */}
      <form onSubmit={handleSearch} className="flex gap-2">
        <div className="relative flex-1">
          <Search
            size={14}
            className="absolute left-2 top-1/2 -translate-y-1/2 text-gray-500"
          />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search npm packages..."
            className="w-full border border-[#555] rounded bg-[#444] pl-7 pr-2 py-1.5 text-sm text-gray-100 outline-none focus:border-[#777]"
            data-testid="palette-npm-search"
          />
        </div>
        <button
          type="submit"
          disabled={isLoading || !query.trim()}
          className="px-3 py-1.5 text-xs rounded bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          data-testid="palette-npm-search-btn"
        >
          {isLoading ? "Searching..." : "Search"}
        </button>
      </form>

      {/* Search results */}
      <div data-testid="palette-search-results">
        {isLoading && (
          <div className="flex items-center justify-center gap-2 py-6 text-xs text-gray-400">
            <Loader2 size={14} className="animate-spin" />
            Searching...
          </div>
        )}

        {!isLoading && available.length === 0 && query.trim() && (
          <div className="py-6 text-center text-xs text-gray-500">
            No packages found. Try a different search term.
          </div>
        )}

        {!isLoading && available.length === 0 && !query.trim() && (
          <div className="py-6 text-center text-xs text-gray-500">
            Search for Node-RED packages on npm to install new nodes.
          </div>
        )}

        {!isLoading &&
          available.map((mod) => {
            const isInstalled = installedSet.has(mod.name);
            const isInstalling =
              installingName === mod.name ||
              (installStatus === "installing" && installingName === mod.name);

            return (
              <div
                key={mod.name}
                className="flex items-start gap-3 rounded border border-[#444] bg-[#333] px-3 py-2 mb-1"
                data-testid="palette-search-result"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-gray-100 truncate">
                      {mod.name}
                    </span>
                    <span className="text-[10px] text-gray-500">
                      v{mod.version}
                    </span>
                  </div>
                  <p className="text-xs text-gray-400 mt-0.5 truncate">
                    {mod.description}
                  </p>
                </div>

                <button
                  type="button"
                  onClick={() => handleInstall(mod.name)}
                  disabled={isInstalled || isInstalling}
                  className={`flex items-center gap-1 px-2 py-1 text-[11px] rounded transition-colors flex-shrink-0 ${
                    isInstalled
                      ? "bg-green-900/30 text-green-400 cursor-default"
                      : isInstalling
                        ? "bg-[#444] text-gray-400 cursor-wait"
                        : "bg-blue-600 text-white hover:bg-blue-700"
                  }`}
                  data-testid="palette-search-install"
                >
                  {isInstalling ? (
                    <>
                      <Loader2 size={12} className="animate-spin" />
                      Installing
                    </>
                  ) : isInstalled ? (
                    "Installed"
                  ) : (
                    <>
                      <Download size={12} />
                      Install
                    </>
                  )}
                </button>
              </div>
            );
          })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Dialog
// ---------------------------------------------------------------------------

export function PaletteEditorDialog({
  open,
  onClose,
  initialTab = "installed",
}: PaletteEditorDialogProps) {
  const [activeTab, setActiveTab] = useState<PaletteEditorTab>(initialTab);
  const refreshInstalled = usePaletteEditorStore((s) => s.refreshInstalled);

  // Refresh installed list when dialog opens
  const prevOpenRef = useRef(false);
  useEffect(() => {
    if (open && !prevOpenRef.current) {
      refreshInstalled();
      setActiveTab(initialTab);
    }
    prevOpenRef.current = open;
  }, [open, initialTab, refreshInstalled]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    function handler(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  const tabs: { id: PaletteEditorTab; label: string }[] = [
    { id: "installed", label: "Installed" },
    { id: "install", label: "Install" },
  ];

  return (
    <div
      className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50"
      data-testid="palette-editor-overlay"
    >
      <div
        className="w-[560px] max-h-[80vh] flex flex-col bg-[#2a2a2a] border border-[#555] rounded-lg shadow-2xl overflow-hidden"
        data-testid="palette-editor-dialog"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-[#555]">
          <span className="text-sm font-semibold text-gray-100">
            Manage Palette
          </span>
          <button
            type="button"
            className="text-gray-400 hover:text-gray-200 text-lg leading-none"
            onClick={onClose}
            aria-label="Close"
            data-testid="palette-editor-btn-close"
          >
            <X size={16} />
          </button>
        </div>

        {/* Tab bar */}
        <div className="flex items-center gap-1 px-4 border-b border-[#555]">
          {tabs.map((tab) => (
            <TabButton
              key={tab.id}
              label={tab.label}
              active={activeTab === tab.id}
              onClick={() => setActiveTab(tab.id)}
            />
          ))}
        </div>

        {/* Tab content */}
        <div className="flex-1 overflow-y-auto px-4 py-3">
          {activeTab === "installed" && <InstalledTab />}
          {activeTab === "install" && <InstallTab />}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-2 border-t border-[#555]">
          <button
            type="button"
            className="px-3 py-1 text-sm rounded border border-[#555] text-gray-300 hover:bg-[#333] transition-colors"
            onClick={onClose}
            data-testid="palette-editor-btn-done"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

export default PaletteEditorDialog;
