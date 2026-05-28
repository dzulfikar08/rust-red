/**
 * SidebarContainer -- Sidebar content container with tab switching.
 *
 * Layout:
 *   ┌──────────────────────────┬───┐
 *   │                          │ i |
 *   │  Content Panel           |   |  SidebarTabBar (vertical icon strip)
 *   │  (active tab content)    │   |
 *   │                          │   |
 *   └──────────────────────────┴───┘
 *
 * When the sidebar is closed, only the tab bar strip (~36px) is visible.
 * Clicking a tab opens the sidebar and shows the content panel.
 */

import { useSidebarStore } from "../../store/sidebar-store";
import { SidebarTabBar } from "./SidebarTabBar";
import { InfoTab } from "./tabs/InfoTab";
import { DebugTab } from "./tabs/DebugTab";
import { ConfigTab } from "./tabs/ConfigTab";
import { ContextTab } from "./tabs/ContextTab";
import { HelpTab } from "./tabs/HelpTab";
import { OutlinerTab } from "./tabs/OutlinerTab";
import type { SidebarTab } from "../../store/sidebar-store";

// ---------------------------------------------------------------------------
// Tab content mapping
// ---------------------------------------------------------------------------

const TAB_COMPONENTS: Record<SidebarTab, React.ComponentType> = {
  info: InfoTab,
  debug: DebugTab,
  config: ConfigTab,
  context: ContextTab,
  help: HelpTab,
  outliner: OutlinerTab,
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarContainer() {
  const isOpen = useSidebarStore((s) => s.isOpen);
  const activeTab = useSidebarStore((s) => s.activeTab);

  const ContentComponent = TAB_COMPONENTS[activeTab];

  return (
    <div
      className="flex h-full"
      data-testid="sidebar-container"
    >
      {/* Content panel -- visible when sidebar is open */}
      {isOpen && (
        <div
          className="flex-1 flex flex-col overflow-hidden bg-white dark:bg-gray-800"
          data-testid="sidebar-content-panel"
        >
          {/* Tab header */}
          <div className="flex items-center justify-between px-3 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
            <span className="text-xs font-semibold text-gray-600 dark:text-gray-300 uppercase tracking-wide">
              {activeTab}
            </span>
          </div>

          {/* Tab content */}
          <div className="flex-1 flex flex-col overflow-auto">
            <ContentComponent />
          </div>
        </div>
      )}

      {/* Tab bar strip (always visible) */}
      <SidebarTabBar />
    </div>
  );
}
