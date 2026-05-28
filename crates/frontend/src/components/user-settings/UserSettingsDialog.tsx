/**
 * UserSettingsDialog -- Modal dialog with tabbed user preferences.
 *
 * Replaces Node-RED's RED.userSettings (~325 lines).
 *
 * Tabs: Editor, Appearance, Keyboard Shortcuts, About
 * Save persists to settings-store; Cancel reverts changes.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { X, Search } from "lucide-react";
import { useSettingsStore, type Theme } from "../../store/settings-store";
import { useThemeStore } from "../../store/theme-store";
import { DEFAULT_KEYBINDINGS, type KeyBinding } from "../../hooks/useKeyboard";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type SettingsTab = "editor" | "appearance" | "shortcuts" | "about";

export interface UserSettingsDialogProps {
  /** Whether the dialog is open */
  open: boolean;
  /** Called when the dialog should close (cancel or after save) */
  onClose: () => void;
  /** Initial tab to show */
  initialTab?: SettingsTab;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Format a keyboard shortcut for display */
function formatShortcut(binding: KeyBinding): string {
  const parts: string[] = [];
  if (binding.modifiers?.ctrl) parts.push("Ctrl");
  if (binding.modifiers?.shift) parts.push("Shift");
  if (binding.modifiers?.alt) parts.push("Alt");
  if (binding.modifiers?.meta) parts.push("Meta");

  // Human-readable key names
  const keyMap: Record<string, string> = {
    " ": "Space",
    Delete: "Delete",
    Backspace: "Backspace",
    Escape: "Esc",
  };
  parts.push(keyMap[binding.key] ?? binding.key.toUpperCase());
  return parts.join("+");
}

/** Human-readable action label */
function actionLabel(action: string): string {
  return action
    .replace(/^core:/, "")
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "de", label: "Deutsch" },
  { value: "ja", label: "\u65E5\u672C\u8A9E" },
  { value: "zh", label: "\u4E2D\u6587" },
  { value: "ko", label: "\uD55C\uAD6D\uC5B4" },
  { value: "fr", label: "Fran\u00E7ais" },
  { value: "es", label: "Espa\u00F1ol" },
];

const APP_VERSION = "0.1.0";
const NODE_RED_COMPAT = "3.1.x";

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
// Editor Tab
// ---------------------------------------------------------------------------

function EditorTab({
  gridSize,
  snapToGrid,
  showTips,
  zoomLevel,
  onGridSizeChange,
  onSnapToGridChange,
  onShowTipsChange,
}: {
  gridSize: number;
  snapToGrid: boolean;
  showTips: boolean;
  zoomLevel: number;
  onGridSizeChange: (v: number) => void;
  onSnapToGridChange: (v: boolean) => void;
  onShowTipsChange: (v: boolean) => void;
}) {
  return (
    <div className="space-y-4">
      {/* Grid size */}
      <div>
        <label
          htmlFor="settings-grid-size"
          className="block text-xs font-medium text-gray-300 mb-1"
        >
          Grid size
        </label>
        <input
          id="settings-grid-size"
          type="number"
          min={5}
          max={100}
          value={gridSize}
          onChange={(e) => onGridSizeChange(Number(e.target.value))}
          className="w-24 border border-[#555] rounded bg-[#444] px-2 py-1 text-sm text-gray-100 outline-none focus:border-[#777]"
          data-testid="settings-grid-size"
        />
      </div>

      {/* Snap to grid */}
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={snapToGrid}
          onChange={(e) => onSnapToGridChange(e.target.checked)}
          className="accent-blue-500"
          data-testid="settings-snap-to-grid"
        />
        <span className="text-xs text-gray-300">Snap to grid</span>
      </label>

      {/* Show tips */}
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={showTips}
          onChange={(e) => onShowTipsChange(e.target.checked)}
          className="accent-blue-500"
          data-testid="settings-show-tips"
        />
        <span className="text-xs text-gray-300">Show tips</span>
      </label>

      {/* Zoom level (read-only display) */}
      <div>
        <span className="block text-xs font-medium text-gray-300 mb-1">
          Current zoom level
        </span>
        <span
          className="text-sm text-gray-200"
          data-testid="settings-zoom-level"
        >
          {Math.round(zoomLevel * 100)}%
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Appearance Tab
// ---------------------------------------------------------------------------

function AppearanceTab({
  theme,
  locale,
  onThemeChange,
  onLocaleChange,
}: {
  theme: Theme;
  locale: string;
  onThemeChange: (t: Theme) => void;
  onLocaleChange: (l: string) => void;
}) {
  return (
    <div className="space-y-4">
      {/* Theme radio buttons */}
      <div>
        <span className="block text-xs font-medium text-gray-300 mb-2">
          Theme
        </span>
        <div className="space-y-1.5">
          {(["dark", "light", "system"] as const).map((t) => (
            <label
              key={t}
              className="flex items-center gap-2 cursor-pointer"
            >
              <input
                type="radio"
                name="theme"
                value={t}
                checked={theme === t}
                onChange={() => onThemeChange(t)}
                className="accent-blue-500"
                data-testid={`settings-theme-${t}`}
              />
              <span className="text-xs text-gray-300 capitalize">{t}</span>
            </label>
          ))}
        </div>
      </div>

      {/* Language dropdown */}
      <div>
        <label
          htmlFor="settings-locale"
          className="block text-xs font-medium text-gray-300 mb-1"
        >
          Language
        </label>
        <select
          id="settings-locale"
          value={locale}
          onChange={(e) => onLocaleChange(e.target.value)}
          className="w-48 border border-[#555] rounded bg-[#444] px-2 py-1 text-sm text-gray-100 outline-none focus:border-[#777]"
          data-testid="settings-locale"
        >
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Keyboard Shortcuts Tab
// ---------------------------------------------------------------------------

function ShortcutsTab() {
  const [filter, setFilter] = useState("");

  const filtered = DEFAULT_KEYBINDINGS.filter((b) => {
    if (!filter) return true;
    const label = actionLabel(b.action).toLowerCase();
    const shortcut = formatShortcut(b).toLowerCase();
    const q = filter.toLowerCase();
    return label.includes(q) || shortcut.includes(q);
  });

  return (
    <div className="space-y-3">
      {/* Search filter */}
      <div className="relative">
        <Search
          size={14}
          className="absolute left-2 top-1/2 -translate-y-1/2 text-gray-500"
        />
        <input
          type="text"
          placeholder="Search shortcuts..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="w-full border border-[#555] rounded bg-[#444] pl-7 pr-2 py-1 text-sm text-gray-100 outline-none focus:border-[#777]"
          data-testid="settings-shortcut-search"
        />
      </div>

      {/* Shortcuts table */}
      <table className="w-full text-xs">
        <thead>
          <tr className="border-b border-[#555]">
            <th className="text-left py-1.5 pr-2 text-gray-400 font-medium">
              Action
            </th>
            <th className="text-left py-1.5 text-gray-400 font-medium">
              Shortcut
            </th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((b, i) => (
            <tr
              key={`${b.action}-${i}`}
              className="border-b border-[#444] last:border-b-0"
            >
              <td className="py-1.5 pr-2 text-gray-200">
                {actionLabel(b.action)}
              </td>
              <td className="py-1.5">
                <kbd className="px-1.5 py-0.5 rounded bg-[#444] border border-[#555] text-gray-300 font-mono text-[11px]">
                  {formatShortcut(b)}
                </kbd>
              </td>
            </tr>
          ))}
          {filtered.length === 0 && (
            <tr>
              <td
                colSpan={2}
                className="py-3 text-center text-gray-500 text-xs"
              >
                No shortcuts found
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

// ---------------------------------------------------------------------------
// About Tab
// ---------------------------------------------------------------------------

function AboutTab() {
  return (
    <div className="space-y-3 text-sm text-gray-200">
      {/* Logo + name */}
      <div className="flex items-center gap-3">
        <svg width="32" height="32" viewBox="0 0 32 32">
          <rect width="32" height="32" rx="6" fill="#8f0000" />
          <text
            x="16"
            y="23"
            fontFamily="Arial, Helvetica, sans-serif"
            fontSize="22"
            fontWeight="bold"
            fill="white"
            textAnchor="middle"
          >
            R
          </text>
        </svg>
        <div>
          <div className="font-bold text-gray-100">Rust-RED</div>
          <div className="text-xs text-gray-400">
            A Rust-powered Node-RED compatible runtime
          </div>
        </div>
      </div>

      <hr className="border-[#555]" />

      {/* Version info */}
      <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
        <span className="text-gray-400">Version</span>
        <span data-testid="settings-about-version">{APP_VERSION}</span>
        <span className="text-gray-400">Node-RED compat</span>
        <span data-testid="settings-about-nr-compat">{NODE_RED_COMPAT}</span>
      </div>

      <hr className="border-[#555]" />

      {/* Links */}
      <div className="space-y-1 text-xs">
        <a
          href="https://nodered.org/docs"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-400 hover:text-blue-300 underline"
        >
          Documentation
        </a>
        <br />
        <a
          href="https://github.com/dzats/rust-red"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-400 hover:text-blue-300 underline"
        >
          Repository
        </a>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Dialog
// ---------------------------------------------------------------------------

export function UserSettingsDialog({
  open,
  onClose,
  initialTab = "editor",
}: UserSettingsDialogProps) {
  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);

  // Read current state from stores
  const settingsTheme = useSettingsStore((s) => s.theme);
  const locale = useSettingsStore((s) => s.locale);
  const editorPrefs = useSettingsStore((s) => s.editorPreferences);
  const sidebarWidth = useSettingsStore((s) => s.sidebarWidth);

  // Local draft state (reverted on Cancel, persisted on Save)
  const [draftGridSize, setDraftGridSize] = useState(editorPrefs.gridSize);
  const [draftSnapToGrid, setDraftSnapToGrid] = useState(
    editorPrefs.snapToGrid,
  );
  const [draftShowTips, setDraftShowTips] = useState(editorPrefs.showTips);
  const [draftTheme, setDraftTheme] = useState<Theme>(settingsTheme);
  const [draftLocale, setDraftLocale] = useState(locale);

  // Reset draft when dialog opens
  const prevOpenRef = useRef(open);
  useEffect(() => {
    if (open && !prevOpenRef.current) {
      const s = useSettingsStore.getState();
      setDraftGridSize(s.editorPreferences.gridSize);
      setDraftSnapToGrid(s.editorPreferences.snapToGrid);
      setDraftShowTips(s.editorPreferences.showTips);
      setDraftTheme(s.theme);
      setDraftLocale(s.locale);
      setActiveTab(initialTab);
    }
    prevOpenRef.current = open;
  }, [open, initialTab]);

  // Zoom approximation (sidebar width ratio is a rough indicator)
  const zoomLevel = 1;

  // Actions
  const updateEditorPreference = useSettingsStore(
    (s) => s.updateEditorPreference,
  );
  const settingsSetTheme = useSettingsStore((s) => s.setTheme);
  const settingsSetLocale = useSettingsStore((s) => s.setLocale);
  const themeStoreSetTheme = useThemeStore((s) => s.setTheme);

  // Save: write draft to stores
  const handleSave = useCallback(() => {
    updateEditorPreference("gridSize", draftGridSize);
    updateEditorPreference("snapToGrid", draftSnapToGrid);
    updateEditorPreference("showTips", draftShowTips);
    settingsSetTheme(draftTheme);
    settingsSetLocale(draftLocale);

    // Sync the runtime theme store
    if (draftTheme === "system") {
      const isDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      themeStoreSetTheme(isDark ? "dark" : "light");
    } else {
      themeStoreSetTheme(draftTheme);
    }

    onClose();
  }, [
    draftGridSize,
    draftSnapToGrid,
    draftShowTips,
    draftTheme,
    draftLocale,
    updateEditorPreference,
    settingsSetTheme,
    settingsSetLocale,
    themeStoreSetTheme,
    onClose,
  ]);

  // Cancel: just close (draft is discarded)
  const handleCancel = useCallback(() => {
    onClose();
  }, [onClose]);

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

  // Suppress unused var lint (sidebarWidth may be used for future zoom display)
  void sidebarWidth;

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: "editor", label: "Editor" },
    { id: "appearance", label: "Appearance" },
    { id: "shortcuts", label: "Keyboard Shortcuts" },
    { id: "about", label: "About" },
  ];

  return (
    <div
      className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50"
      data-testid="user-settings-overlay"
    >
      <div
        className="w-[540px] max-h-[80vh] flex flex-col bg-[#2a2a2a] border border-[#555] rounded-lg shadow-2xl overflow-hidden"
        data-testid="user-settings-dialog"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-[#555]">
          <span className="text-sm font-semibold text-gray-100">
            User Settings
          </span>
          <button
            type="button"
            className="text-gray-400 hover:text-gray-200 text-lg leading-none"
            onClick={handleCancel}
            aria-label="Close"
            data-testid="user-settings-btn-close"
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
          {activeTab === "editor" && (
            <EditorTab
              gridSize={draftGridSize}
              snapToGrid={draftSnapToGrid}
              showTips={draftShowTips}
              zoomLevel={zoomLevel}
              onGridSizeChange={setDraftGridSize}
              onSnapToGridChange={setDraftSnapToGrid}
              onShowTipsChange={setDraftShowTips}
            />
          )}
          {activeTab === "appearance" && (
            <AppearanceTab
              theme={draftTheme}
              locale={draftLocale}
              onThemeChange={setDraftTheme}
              onLocaleChange={setDraftLocale}
            />
          )}
          {activeTab === "shortcuts" && <ShortcutsTab />}
          {activeTab === "about" && <AboutTab />}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-2 border-t border-[#555]">
          <button
            type="button"
            className="px-3 py-1 text-sm rounded border border-[#555] text-gray-300 hover:bg-[#333] transition-colors"
            onClick={handleCancel}
            data-testid="user-settings-btn-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            className="px-3 py-1 text-sm rounded bg-blue-600 text-white hover:bg-blue-700 transition-colors"
            onClick={handleSave}
            data-testid="user-settings-btn-save"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

export default UserSettingsDialog;
