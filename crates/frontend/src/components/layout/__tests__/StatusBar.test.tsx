/**
 * Tests for the StatusBar component
 *
 * We mock the Zustand stores to isolate StatusBar rendering.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusBar } from "../StatusBar";

// ---------------------------------------------------------------------------
// Mock stores
// ---------------------------------------------------------------------------

vi.mock("../../../store/status-store", () => ({
  useStatusStore: vi.fn((selector: (s: any) => any) =>
    selector({ connectionStatus: "connected" }),
  ),
}));

vi.mock("../../../store/deploy-store", () => ({
  useDeployStore: vi.fn((selector: (s: any) => any) =>
    selector({ lastDeployTime: 1700000000000 }),
  ),
}));

vi.mock("../../../store/flow-store", () => ({
  useFlowStore: vi.fn((selector: (s: any) => any) =>
    selector({ nodes: Array(12).fill({}) }),
  ),
}));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("StatusBar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the status bar element", () => {
      render(<StatusBar />);
      expect(screen.getByTestId("status-bar")).toBeInTheDocument();
    });

    it("renders the connection status section", () => {
      render(<StatusBar />);
      expect(screen.getByTestId("status-bar-connection")).toBeInTheDocument();
      expect(screen.getByText("Connected")).toBeInTheDocument();
    });

    it("renders the deploy time section", () => {
      render(<StatusBar />);
      const deployEl = screen.getByTestId("status-bar-deploy");
      expect(deployEl).toBeInTheDocument();
      expect(deployEl.textContent).toMatch(/^Deployed:/);
    });

    it("renders the node count section", () => {
      render(<StatusBar />);
      expect(screen.getByTestId("status-bar-nodes")).toBeInTheDocument();
      expect(screen.getByText("12 nodes")).toBeInTheDocument();
    });

    it("renders the zoom level section", () => {
      render(<StatusBar zoomLevel={0.85} />);
      const zoomEl = screen.getByTestId("status-bar-zoom");
      expect(zoomEl).toBeInTheDocument();
      expect(zoomEl.textContent).toBe("85%");
    });

    it("renders the connection status dot with correct color", () => {
      render(<StatusBar />);
      const dot = screen.getByTestId("status-bar-connection-dot");
      expect(dot).toBeInTheDocument();
      // "connected" state -> bg-green-500
      expect(dot.className).toContain("bg-green-500");
    });
  });

  // -----------------------------------------------------------------------
  // Zoom level display
  // -----------------------------------------------------------------------

  describe("zoom level", () => {
    it("shows 100% when no zoomLevel prop is provided", () => {
      render(<StatusBar />);
      expect(screen.getByTestId("status-bar-zoom").textContent).toBe("100%");
    });

    it("shows the correct percentage", () => {
      render(<StatusBar zoomLevel={1.5} />);
      expect(screen.getByTestId("status-bar-zoom").textContent).toBe("150%");
    });
  });

  // -----------------------------------------------------------------------
  // Custom className
  // -----------------------------------------------------------------------

  describe("custom className", () => {
    it("appends custom className to the root element", () => {
      render(<StatusBar className="custom-class" />);
      expect(screen.getByTestId("status-bar").className).toContain(
        "custom-class",
      );
    });
  });
});
