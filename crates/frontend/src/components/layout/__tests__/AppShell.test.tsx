/**
 * Tests for the AppShell component
 *
 * We mock all child components and stores to isolate the layout shell logic.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { AppShell } from "../AppShell";

// ---------------------------------------------------------------------------
// Mock child components
// ---------------------------------------------------------------------------

vi.mock("../Header", () => ({
  Header: ({ sidebarOpen, onToggleSidebar }: { sidebarOpen: boolean; onToggleSidebar: () => void }) => (
    <div data-testid="header" data-sidebar-open={String(sidebarOpen)} />
  ),
}));

vi.mock("../Sidebar", () => ({
  Sidebar: ({ open }: { open: boolean }) => (
    <div data-testid="sidebar" data-open={String(open)} />
  ),
}));

vi.mock("../../palette/Palette", () => ({
  Palette: () => <div data-testid="palette" />,
}));

vi.mock("../../editor/FlowCanvas", () => ({
  FlowCanvas: () => <div data-testid="flow-canvas" />,
}));

vi.mock("../../workspaces/WorkspaceTabs", () => ({
  WorkspaceTabs: () => <div data-testid="workspace-tabs" />,
}));

vi.mock("../../debug/DebugPanel", () => ({
  DebugPanel: () => <div data-testid="debug-panel" />,
}));

vi.mock("../../debug/MobileDebugPanel", () => ({
  MobileDebugPanel: () => <div data-testid="mobile-debug-panel" />,
}));

vi.mock("../../ai/AIAssistant", () => ({
  AIAssistant: () => <div data-testid="ai-assistant" />,
}));

vi.mock("../mobile", () => ({
  BottomTabBar: () => <div data-testid="bottom-tab-bar" />,
  MobileSidebar: () => <div data-testid="mobile-sidebar" />,
  FAB: () => <div data-testid="fab" />,
  NodeConfigSheet: () => <div data-testid="node-config-sheet" />,
  ContextMenu: () => <div data-testid="context-menu" />,
  ConnectModeOverlay: () => <div data-testid="connect-mode-overlay" />,
  useLongPress: () => ({}),
}));

// ---------------------------------------------------------------------------
// Mock stores
// ---------------------------------------------------------------------------

vi.mock("../../../store/editor-store", () => ({
  useEditorStore: Object.assign(
    (selector?: (s: Record<string, unknown>) => unknown) =>
      selector
        ? selector({ debugMessages: [], selectedNodeId: null, addNotification: vi.fn() })
        : { debugMessages: [], selectedNodeId: null },
    {
      getState: () => ({
        addNotification: vi.fn(),
        selectNode: vi.fn(),
      }),
    },
  ),
}));

vi.mock("../../../store/flow-store", () => {
  const state = {
    addNode: vi.fn(),
    updateNodeData: vi.fn(),
    nodes: [],
    edges: [],
  };
  const useFlowStore = (selector?: (s: typeof state) => unknown) =>
    selector ? selector(state) : state;
  return {
    useFlowStore: Object.assign(useFlowStore, {
      setState: vi.fn(),
      getState: () => state,
    }),
  };
});

// ---------------------------------------------------------------------------
// Mock hooks
// ---------------------------------------------------------------------------

vi.mock("../../../hooks", () => ({
  useBreakpoint: () => ({ isMobile: false }),
}));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("AppShell", () => {
  const defaultProps = {
    onDeploy: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Desktop layout
  // -----------------------------------------------------------------------

  describe("desktop layout", () => {
    it("renders the header", () => {
      render(<AppShell {...defaultProps} />);
      expect(screen.getByTestId("header")).toBeInTheDocument();
    });

    it("renders the palette", () => {
      render(<AppShell {...defaultProps} />);
      expect(screen.getByTestId("palette")).toBeInTheDocument();
    });

    it("renders the flow canvas", () => {
      render(<AppShell {...defaultProps} />);
      expect(screen.getByTestId("flow-canvas")).toBeInTheDocument();
    });

    it("renders the workspace tabs", () => {
      render(<AppShell {...defaultProps} />);
      expect(screen.getByTestId("workspace-tabs")).toBeInTheDocument();
    });

    it("renders the sidebar (hidden by default)", () => {
      render(<AppShell {...defaultProps} />);
      const sidebar = screen.getByTestId("sidebar");
      expect(sidebar).toBeInTheDocument();
      expect(sidebar.dataset.open).toBe("false");
    });

    it("does not render mobile components on desktop", () => {
      render(<AppShell {...defaultProps} />);
      expect(screen.queryByTestId("bottom-tab-bar")).not.toBeInTheDocument();
      expect(screen.queryByTestId("fab")).not.toBeInTheDocument();
      expect(screen.queryByTestId("mobile-sidebar")).not.toBeInTheDocument();
    });

    it("passes sidebarOpen=false to header by default", () => {
      render(<AppShell {...defaultProps} />);
      const header = screen.getByTestId("header");
      expect(header.dataset.sidebarOpen).toBe("false");
    });
  });
});
