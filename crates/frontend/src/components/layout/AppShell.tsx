/**
 * AppShell -- Main application layout matching Node-RED's editor layout.
 *
 * Desktop (md+):
 *   ┌─────────────────────────────────────────────────────────┐
 *   │  Header / Toolbar                                       │
 *   ├──────────┬──────────────────────────────────┬───────────┤
 *   │          │                                  │           │
 *   │  Palette │       Canvas (Flow Editor)       │  Sidebar  │
 *   │  (180px) │                                  │  (320px)  │
 *   │          │                                  │           │
 *   │          ├──────────────────────────────────┤           │
 *   │          │  WorkspaceTabs                    │           │
 *   └──────────┴──────────────────────────────────┴───────────┘
 *
 * Mobile:
 *   Existing mobile layout with bottom tabs, hamburger, FAB, etc.
 */

import { useState, useCallback, useEffect } from "react";
import type { Node } from "@xyflow/react";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import { Palette } from "../palette/Palette";
import { FlowCanvas } from "../editor/FlowCanvas";
import { WorkspaceTabs } from "../workspaces/WorkspaceTabs";
import { DebugPanel } from "../debug/DebugPanel";
import { MobileDebugPanel } from "../debug/MobileDebugPanel";
import { AIAssistant } from "../ai/AIAssistant";
import { SearchDialog } from "../search/SearchDialog";
import { StatusBar } from "./StatusBar";
import { commsClient } from "../../ws/comms";
import { useStatusStore } from "../../store/status-store";
import { useEditorStore } from "../../store/editor-store";
import { useFlowStore } from "../../store/flow-store";
import { useSidebarStore } from "../../store/sidebar-store";
import { useSearchStore } from "../../store/search-store";
import { useActionStore } from "../../store/action-store";
import { useClipboardStore } from "../../store/clipboard-store";
import { initContextMenuListener } from "../../store/context-menu-store";
import { useBreakpoint } from "../../hooks";
import { ContextMenuManager } from "../context-menu/ContextMenuManager";
import {
  BottomTabBar,
  type MobileTab,
  MobileSidebar,
  FAB,
  NodeConfigSheet,
  ContextMenu,
  ConnectModeOverlay,
  useLongPress,
  type ContextAction,
} from "../mobile";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface AppShellProps {
  onDeploy: () => void;
  showHeader?: boolean;
}

interface PaletteNode {
  type: string;
  label: string;
  color: string;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AppShell({ onDeploy, showHeader = false }: AppShellProps) {
  const debugMessages = useEditorStore((s) => s.debugMessages);
  const debugVisible = debugMessages.length > 0;
  const { isMobile } = useBreakpoint();

  // Sidebar state from store (shared with Header and SidebarContainer)
  const sidebarOpen = useSidebarStore((s) => s.isOpen);
  const sidebarToggle = useSidebarStore((s) => s.toggle);

  // Mobile state
  const [mobileTab, setMobileTab] = useState<MobileTab>("flows");
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [configNode, setConfigNode] = useState<Node | null>(null);
  const [configOpen, setConfigOpen] = useState(false);
  const [ctxMenu, setCtxMenu] = useState<{
    x: number;
    y: number;
    actions: ContextAction[];
  } | null>(null);

  const addNode = useFlowStore((s) => s.addNode);

  const handleSelectNode = useCallback(
    (node: PaletteNode) => {
      const newNode: Node = {
        id: `${node.type}-${Date.now()}`,
        type: "generic",
        position: { x: 100 + Math.random() * 200, y: 100 + Math.random() * 200 },
        data: { label: node.label, nodeType: node.type, color: node.color },
      };
      addNode(newNode);
    },
    [addNode],
  );

  const updateNodeData = useFlowStore((s) => s.updateNodeData);

  const handleSaveConfig = useCallback(
    (id: string, data: Record<string, unknown>) => {
      updateNodeData(id, data);
      useEditorStore.getState().addNotification({
        type: "success",
        message: `Node ${id} updated`,
      });
    },
    [updateNodeData],
  );

  const canvasLongPress = useLongPress((e) => {
    setCtxMenu({
      x: (e as React.PointerEvent).clientX,
      y: (e as React.PointerEvent).clientY,
      actions: [
        { id: "add-node", label: "Add node...", handler: () => setMobileSidebarOpen(true) },
        { id: "paste", label: "Paste", handler: () => {} },
        { id: "select-all", label: "Select all", handler: () => {} },
      ],
    });
  });

  const handleToggleSidebar = useCallback(() => {
    sidebarToggle();
  }, [sidebarToggle]);

  // -----------------------------------------------------------------------
  // WebSocket connection status tracking
  // -----------------------------------------------------------------------

  const setConnectionStatus = useStatusStore((s) => s.setConnectionStatus);

  useEffect(() => {
    // Wire up the comms client lifecycle to the status store so the
    // StatusBar always shows the current connection state.
    commsClient.onConnectionChange(setConnectionStatus);
    // Default to disconnected until the WebSocket transitions.
    setConnectionStatus("disconnected");
  }, [setConnectionStatus]);

  // -----------------------------------------------------------------------
  // Initialize context menu listener (canvas right-click events)
  // -----------------------------------------------------------------------

  useEffect(() => {
    initContextMenuListener();
  }, []);

  // -----------------------------------------------------------------------
  // Ctrl+F keyboard shortcut to open search
  // -----------------------------------------------------------------------

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        useSearchStore.getState().open();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  // -----------------------------------------------------------------------
  // Register clipboard actions (core:copy, core:paste, core:cut)
  // -----------------------------------------------------------------------

  useEffect(() => {
    const clipboard = useClipboardStore.getState();
    const actionStore = useActionStore.getState();

    actionStore.registerAction({
      id: "core:copy",
      name: "Copy",
      scope: "core",
      key: "Ctrl-C",
      handler: () => {
        const selectedNodeId = useEditorStore.getState().selectedNodeId;
        if (selectedNodeId) {
          clipboard.copy([selectedNodeId]);
        }
      },
    });

    actionStore.registerAction({
      id: "core:paste",
      name: "Paste",
      scope: "core",
      key: "Ctrl-V",
      handler: () => {
        if (clipboard.hasClipboard) {
          clipboard.paste();
        }
      },
    });

    actionStore.registerAction({
      id: "core:cut",
      name: "Cut",
      scope: "core",
      key: "Ctrl-X",
      handler: () => {
        const selectedNodeId = useEditorStore.getState().selectedNodeId;
        if (selectedNodeId) {
          clipboard.cut([selectedNodeId]);
        }
      },
    });

    // Cleanup is optional since the app shell lives for the app lifetime,
    // but we return a cleanup function for correctness.
    return () => {
      useActionStore.getState().unregisterAction("core:copy");
      useActionStore.getState().unregisterAction("core:paste");
      useActionStore.getState().unregisterAction("core:cut");
    };
  }, []);

  // -----------------------------------------------------------------------
  // Desktop layout
  // -----------------------------------------------------------------------

  if (!isMobile) {
    return (
      <div className="flex flex-col h-full w-full overflow-hidden bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
        {/* Header / Toolbar */}
        <Header
          sidebarOpen={sidebarOpen}
          onToggleSidebar={handleToggleSidebar}
        />

        {/* Main content: Palette | Canvas+Tabs | Sidebar */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left: Palette (fixed ~180px) */}
          <Palette />

          {/* Center: Canvas + WorkspaceTabs at bottom */}
          <main className="flex-1 flex flex-col overflow-hidden min-w-0">
            <FlowCanvas />
            <WorkspaceTabs />
            {debugVisible && <DebugPanel />}
          </main>

          {/* Right: Sidebar (collapsible, hidden by default) */}
          <Sidebar open={sidebarOpen} />
        </div>

        {/* Status bar */}
        <StatusBar />

        {/* Global search dialog (Ctrl+F) */}
        <SearchDialog />

        {/* Context menu (right-click) */}
        <ContextMenuManager />
      </div>
    );
  }

  // -----------------------------------------------------------------------
  // Mobile layout (preserved from original)
  // -----------------------------------------------------------------------

  return (
    <div className="flex flex-col h-full w-full overflow-hidden bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      {/* Header with deploy and hamburger */}
      <div className="flex items-center justify-between border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-1.5">
        <button
          type="button"
          onClick={() => setMobileSidebarOpen(true)}
          className="w-10 h-10 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
          aria-label="Open node palette"
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor" className="text-gray-600 dark:text-gray-300">
            <rect x="2" y="4" width="16" height="2" rx="1" />
            <rect x="2" y="9" width="16" height="2" rx="1" />
            <rect x="2" y="14" width="16" height="2" rx="1" />
          </svg>
        </button>
        <span className="text-sm font-semibold text-gray-800 dark:text-gray-100">RustRED</span>
        <button
          type="button"
          onClick={onDeploy}
          className="px-4 py-2 rounded-lg text-sm font-medium text-white active:opacity-90"
          style={{ backgroundColor: "var(--color-primary)" }}
        >
          Deploy
        </button>
      </div>

      {/* Connection mode overlay */}
      <ConnectModeOverlay />

      {/* Main content */}
      <div className="flex-1 overflow-hidden" {...canvasLongPress}>
        {mobileTab === "flows" && <FlowCanvas />}
        {mobileTab === "debug" && <MobileDebugPanel />}
        {mobileTab === "ai" && <AIAssistant />}
      </div>

      {/* Status bar */}
      <StatusBar />

      {/* Bottom tab bar */}
      <BottomTabBar
        active={mobileTab}
        onTabChange={setMobileTab}
        debugCount={debugMessages.length}
      />

      {/* FAB to add nodes */}
      {mobileTab === "flows" && <FAB onClick={() => setMobileSidebarOpen(true)} />}

      {/* Mobile sidebar drawer */}
      <MobileSidebar
        open={mobileSidebarOpen}
        onClose={() => setMobileSidebarOpen(false)}
        onSelectNode={handleSelectNode}
      />

      {/* Node config sheet */}
      <NodeConfigSheet
        node={configNode}
        open={configOpen}
        onClose={() => setConfigOpen(false)}
        onSave={handleSaveConfig}
      />

      {/* Context menu */}
      {ctxMenu && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          actions={ctxMenu.actions}
          onClose={() => setCtxMenu(null)}
        />
      )}

      {/* Global search dialog (Ctrl+F) */}
      <SearchDialog />
    </div>
  );
}
