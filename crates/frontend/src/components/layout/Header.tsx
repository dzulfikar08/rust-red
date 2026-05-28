/**
 * Header -- Node-RED style toolbar.
 *
 * Layout:
 *   [Hamburger] [Logo "Node-RED"]  ---  [Palette] [User] [Deploy] [Theme]
 *
 * Dark bar (#333 base) with white/light text.
 */

import { useState, useRef, useEffect, useCallback } from "react";
import {
  Menu,
  Package,
  User,
  Sun,
  Moon,
  PanelRightOpen,
  PanelRightClose,
} from "lucide-react";
import { useThemeStore } from "../../store/theme-store";
import { DeployButton } from "../deploy/Deploy";
import { ImportDialog, ExportDialog } from "../clipboard";
import { useClipboardStore } from "../../store/clipboard-store";
import {
  UserSettingsDialog,
  type SettingsTab,
} from "../user-settings/UserSettingsDialog";
import { PaletteEditorDialog } from "../palette/PaletteEditorDialog";
import { DiffDialog } from "../diff/DiffDialog";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface HeaderProps {
  /** Whether the right sidebar is currently open. */
  sidebarOpen: boolean;
  /** Toggle the right sidebar open / closed. */
  onToggleSidebar: () => void;
}

// ---------------------------------------------------------------------------
// Dropdown menu (for hamburger / user / palette menus)
// ---------------------------------------------------------------------------

function DropdownMenu({
  open,
  onClose,
  children,
}: {
  open: boolean;
  onClose: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      ref={ref}
      className="absolute top-full left-0 mt-1 min-w-[180px] rounded border border-gray-600 bg-gray-800 py-1 shadow-lg z-50"
    >
      {children}
    </div>
  );
}

function MenuItem({
  label,
  onClick,
}: {
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="flex w-full items-center px-3 py-1.5 text-left text-xs text-gray-200 hover:bg-gray-700"
      onClick={onClick}
    >
      {label}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function Header({ sidebarOpen, onToggleSidebar }: HeaderProps) {
  const { theme, toggleTheme } = useThemeStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("editor");
  const [paletteEditorOpen, setPaletteEditorOpen] = useState(false);
  const [diffOpen, setDiffOpen] = useState(false);

  const closeMenu = useCallback(() => setMenuOpen(false), []);
  const closeUserMenu = useCallback(() => setUserMenuOpen(false), []);

  const openSettings = useCallback(
    (tab: SettingsTab = "editor") => {
      setSettingsTab(tab);
      setSettingsOpen(true);
    },
    [],
  );

  const handleImport = useCallback((json: string) => {
    useClipboardStore.getState().importFromJson(json);
    setImportOpen(false);
  }, []);

  const handleExportOpen = useCallback(() => {
    setExportOpen(true);
  }, []);

  const exportJson = useClipboardStore.getState().exportToJson();

  return (
    <header
      className="flex items-center justify-between px-2 bg-[#333] dark:bg-[#2a2a2a] border-b border-[#444] relative z-30 select-none"
      style={{ height: 40 }}
      data-testid="header"
    >
      {/* Left section */}
      <div className="flex items-center gap-1">
        {/* Hamburger / main menu */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setMenuOpen((v) => !v)}
            className="flex items-center justify-center w-8 h-8 rounded hover:bg-white/10 transition-colors"
            title="Main menu"
            data-testid="header-menu-btn"
          >
            <Menu size={18} className="text-gray-200" />
          </button>
          <DropdownMenu open={menuOpen} onClose={closeMenu}>
            <MenuItem label="Flows" onClick={closeMenu} />
            <MenuItem label="Import" onClick={() => { closeMenu(); setImportOpen(true); }} />
            <MenuItem label="Export" onClick={() => { closeMenu(); handleExportOpen(); }} />
            <MenuItem label="Compare with deployed" onClick={() => { closeMenu(); setDiffOpen(true); }} />
            <hr className="my-1 border-gray-600" />
            <MenuItem label="Settings" onClick={() => { closeMenu(); openSettings("editor"); }} />
          </DropdownMenu>
        </div>

        {/* Logo */}
        <div className="flex items-center gap-1.5 ml-1">
          <svg
            width="22"
            height="22"
            viewBox="0 0 32 32"
            className="flex-shrink-0"
          >
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
          <span className="font-bold text-sm tracking-tight text-gray-100 select-none">
            Node-RED
          </span>
        </div>
      </div>

      {/* Center section (empty or breadcrumb) */}
      <div className="flex-1" />

      {/* Right section */}
      <div className="flex items-center gap-1">
        {/* Manage palette */}
        <button
          type="button"
          onClick={() => setPaletteEditorOpen(true)}
          className="flex items-center justify-center w-8 h-8 rounded hover:bg-white/10 transition-colors"
          title="Manage palette"
          data-testid="header-palette-btn"
        >
          <Package size={16} className="text-gray-300" />
        </button>

        {/* User settings */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setUserMenuOpen((v) => !v)}
            className="flex items-center justify-center w-8 h-8 rounded hover:bg-white/10 transition-colors"
            title="User"
            data-testid="header-user-btn"
          >
            <User size={16} className="text-gray-300" />
          </button>
          <DropdownMenu open={userMenuOpen} onClose={closeUserMenu}>
            <MenuItem label="Preferences" onClick={() => { closeUserMenu(); openSettings("editor"); }} />
            <MenuItem label="Keyboard shortcuts" onClick={() => { closeUserMenu(); openSettings("shortcuts"); }} />
            <hr className="my-1 border-gray-600" />
            <MenuItem label="About" onClick={() => { closeUserMenu(); openSettings("about"); }} />
          </DropdownMenu>
        </div>

        {/* Deploy button */}
        <DeployButton />

        {/* Sidebar toggle */}
        <button
          type="button"
          onClick={onToggleSidebar}
          className="flex items-center justify-center w-8 h-8 rounded hover:bg-white/10 transition-colors"
          title={sidebarOpen ? "Close sidebar" : "Open sidebar"}
          data-testid="header-sidebar-toggle"
        >
          {sidebarOpen ? (
            <PanelRightClose size={16} className="text-gray-300" />
          ) : (
            <PanelRightOpen size={16} className="text-gray-300" />
          )}
        </button>

        {/* Theme toggle */}
        <button
          type="button"
          onClick={toggleTheme}
          className="flex items-center justify-center w-8 h-8 rounded hover:bg-white/10 transition-colors"
          title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
          data-testid="header-theme-toggle"
        >
          {theme === "dark" ? (
            <Sun size={16} className="text-gray-300" />
          ) : (
            <Moon size={16} className="text-gray-300" />
          )}
        </button>
      </div>

      {/* Import/Export dialogs */}
      <ImportDialog
        open={importOpen}
        onClose={() => setImportOpen(false)}
        onImport={handleImport}
      />
      <ExportDialog
        open={exportOpen}
        onClose={() => setExportOpen(false)}
        json={exportJson}
      />

      {/* User Settings dialog */}
      <UserSettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialTab={settingsTab}
      />

      {/* Palette Editor dialog */}
      <PaletteEditorDialog
        open={paletteEditorOpen}
        onClose={() => setPaletteEditorOpen(false)}
      />

      {/* Diff dialog */}
      <DiffDialog
        open={diffOpen}
        onClose={() => setDiffOpen(false)}
      />
    </header>
  );
}
