/**
 * Tests for FlowCanvas component
 *
 * We mock @xyflow/react heavily to test the component logic without
 * the full ReactFlow rendering pipeline.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";

// ---------------------------------------------------------------------------
// Mock stores — factory must be self-contained (no external variables)
// ---------------------------------------------------------------------------

vi.mock("../../../store/flow-store", () => {
  const state = {
    nodes: [],
    edges: [],
    onNodesChange: vi.fn(),
    onEdgesChange: vi.fn(),
    addNode: vi.fn(),
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

vi.mock("../../../store/editor-store", () => ({
  useEditorStore: Object.assign(
    () => ({
      debugMessages: [],
      selectedNodeId: null,
      selectNode: vi.fn(),
    }),
    {
      getState: () => ({
        selectNode: vi.fn(),
      }),
    },
  ),
}));

// ---------------------------------------------------------------------------
// Mock hooks
// ---------------------------------------------------------------------------

vi.mock("../../../hooks", () => ({
  useBreakpoint: () => ({ isMobile: false }),
  useTouchGestures: () => ({}),
}));

// ---------------------------------------------------------------------------
// Mock eventBus & nodeRegistry
// ---------------------------------------------------------------------------

vi.mock("../../../red/core/events", () => ({
  eventBus: {
    emit: vi.fn(),
    on: vi.fn(() => vi.fn()),
    off: vi.fn(),
  },
}));

vi.mock("../../../red/nodes/registry", () => ({
  nodeRegistry: {
    getType: vi.fn(() => undefined),
  },
}));

// ---------------------------------------------------------------------------
// Mock @xyflow/react
// ---------------------------------------------------------------------------

vi.mock("@xyflow/react", () => {
  const React = require("react");

  function MockReactFlow({
    children,
    nodeTypes: _nodeTypes,
    edgeTypes: _edgeTypes,
    ...rest
  }: {
    children?: React.ReactNode;
    nodeTypes?: Record<string, unknown>;
    edgeTypes?: Record<string, unknown>;
    [key: string]: unknown;
  }) {
    return (
      <div
        data-testid="mock-react-flow"
        data-snap-to-grid={String(rest.snapToGrid)}
        data-snap-grid={JSON.stringify(rest.snapGrid)}
        data-min-zoom={String(rest.minZoom)}
        data-max-zoom={String(rest.maxZoom)}
      >
        {children}
      </div>
    );
  }

  return {
    ReactFlow: MockReactFlow,
    Background: ({ gap, size }: { gap?: number; size?: number }) => (
      <div data-testid="canvas-background" data-gap={gap} data-size={size} />
    ),
    MiniMap: () => <div data-testid="canvas-minimap" />,
    addEdge: vi.fn((connection, edges) => [
      ...edges,
      { ...connection, id: `e-${Date.now()}` },
    ]),
    useReactFlow: () => ({
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      fitView: vi.fn(),
      screenToFlowPosition: (pos: { x: number; y: number }) => ({
        x: pos.x,
        y: pos.y,
      }),
      getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
      setViewport: vi.fn(),
    }),
    useStoreApi: () => ({
      subscribe: () => vi.fn(),
      getState: () => ({ transform: [0, 0, 1] }),
    }),
    Position: {
      Left: "left",
      Right: "right",
      Top: "top",
      Bottom: "bottom",
    },
  };
});

// ---------------------------------------------------------------------------
// Import after mocks
// ---------------------------------------------------------------------------

import { FlowCanvas } from "../../editor/FlowCanvas";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("FlowCanvas", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the flow canvas", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("mock-react-flow")).toBeInTheDocument();
    });

    it("renders the background component", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("canvas-background")).toBeInTheDocument();
    });

    it("renders the minimap", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("canvas-minimap")).toBeInTheDocument();
    });

    it("renders the zoom controls", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-controls")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Grid configuration
  // -----------------------------------------------------------------------

  describe("snap-to-grid configuration", () => {
    it("enables snap-to-grid", () => {
      render(<FlowCanvas />);
      const flow = screen.getByTestId("mock-react-flow");
      expect(flow.getAttribute("data-snap-to-grid")).toBe("true");
    });

    it("uses 20px snap grid (matching Node-RED)", () => {
      render(<FlowCanvas />);
      const flow = screen.getByTestId("mock-react-flow");
      expect(flow.getAttribute("data-snap-grid")).toBe("[20,20]");
    });
  });

  // -----------------------------------------------------------------------
  // Zoom configuration
  // -----------------------------------------------------------------------

  describe("zoom configuration", () => {
    it("sets minimum zoom to 0.1 (10%)", () => {
      render(<FlowCanvas />);
      const flow = screen.getByTestId("mock-react-flow");
      expect(flow.getAttribute("data-min-zoom")).toBe("0.1");
    });

    it("sets maximum zoom to 2.0 (200%)", () => {
      render(<FlowCanvas />);
      const flow = screen.getByTestId("mock-react-flow");
      expect(flow.getAttribute("data-max-zoom")).toBe("2");
    });

    it("renders zoom controls with zoom level display", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-level")).toBeInTheDocument();
    });

    it("shows default zoom level of 100%", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("100%");
    });
  });

  // -----------------------------------------------------------------------
  // Zoom control buttons
  // -----------------------------------------------------------------------

  describe("zoom control interactions", () => {
    it("renders zoom in button", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-in-btn")).toBeInTheDocument();
    });

    it("renders zoom out button", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-out-btn")).toBeInTheDocument();
    });

    it("renders zoom to fit button", () => {
      render(<FlowCanvas />);
      expect(screen.getByTestId("zoom-fit-btn")).toBeInTheDocument();
    });
  });
});
