/**
 * SidebarTabBar -- Vertical strip of icon buttons along the sidebar's right edge.
 *
 * Each tab is an icon button (~36x36px). The active tab is highlighted with an
 * accent background. Clicking a tab switches the active tab and opens the
 * sidebar if it is currently closed.
 */

import {
  Info,
  Bug,
  Settings,
  Box,
  HelpCircle,
  ListTree,
} from "lucide-react";
import type { SidebarTab } from "../../store/sidebar-store";
import { useSidebarStore } from "../../store/sidebar-store";

// ---------------------------------------------------------------------------
// Tab definition
// ---------------------------------------------------------------------------

interface TabDef {
  id: SidebarTab;
  icon: React.ComponentType<{ size?: number }>;
  label: string;
}

const TABS: TabDef[] = [
  { id: "info", icon: Info, label: "Info" },
  { id: "debug", icon: Bug, label: "Debug" },
  { id: "config", icon: Settings, label: "Config" },
  { id: "context", icon: Box, label: "Context" },
  { id: "help", icon: HelpCircle, label: "Help" },
  { id: "outliner", icon: ListTree, label: "Outliner" },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarTabBar() {
  const activeTab = useSidebarStore((s) => s.activeTab);
  const isOpen = useSidebarStore((s) => s.isOpen);
  const open = useSidebarStore((s) => s.open);
  const setActiveTab = useSidebarStore((s) => s.setActiveTab);

  const handleClick = (tab: SidebarTab) => {
    if (isOpen && activeTab === tab) {
      // Clicking the active tab on an open sidebar closes it
      useSidebarStore.getState().close();
    } else {
      setActiveTab(tab);
      if (!isOpen) {
        open(tab);
      }
    }
  };

  return (
    <div
      className="flex flex-col items-center py-1 bg-gray-100 dark:bg-gray-800 border-l border-gray-300 dark:border-gray-700"
      data-testid="sidebar-tab-bar"
    >
      {TABS.map(({ id, icon: Icon, label }) => {
        const isActive = activeTab === id;
        return (
          <button
            key={id}
            type="button"
            title={label}
            aria-label={label}
            data-testid={`sidebar-tab-btn-${id}`}
            onClick={() => handleClick(id)}
            className={`
              flex items-center justify-center w-9 h-9 rounded transition-colors my-0.5
              ${
                isActive
                  ? "bg-gray-300 dark:bg-gray-600 text-gray-800 dark:text-gray-100"
                  : "text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 hover:text-gray-700 dark:hover:text-gray-200"
              }
            `}
          >
            <Icon size={18} />
          </button>
        );
      })}
    </div>
  );
}
