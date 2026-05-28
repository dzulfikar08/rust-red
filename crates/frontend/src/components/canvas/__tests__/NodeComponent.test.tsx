/**
 * Tests for NodeComponent
 *
 * We mock @xyflow/react's Handle component to avoid needing the ReactFlow
 * provider context, then render NodeComponent standalone with mock props
 * and verify visual output.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// ---------------------------------------------------------------------------
// Mock @xyflow/react – stub Handle so it doesn't need the zustand provider
// ---------------------------------------------------------------------------

vi.mock("@xyflow/react", () => {
  const React = require("react");
  return {
    Handle: React.forwardRef(function Handle(
      props: {
        type: string;
        position: string;
        id: string;
        style?: React.CSSProperties;
        className?: string;
      },
      _ref: React.Ref<HTMLDivElement>,
    ) {
      return (
        <div
          data-handleid={props.id}
          data-handletype={props.type}
          data-handleposition={props.position}
          className={props.className}
          style={props.style}
        />
      );
    }),
    Position: {
      Left: "left",
      Right: "right",
      Top: "top",
      Bottom: "bottom",
    },
    // Re-export types (erased at runtime, but keeps imports valid)
  };
});

import { NodeComponent } from "../NodeComponent";
import type { NRNode } from "../NodeComponent";

// ---------------------------------------------------------------------------
// Type-only import (resolved at compile time, not affected by vi.mock)
// ---------------------------------------------------------------------------

type NodeProps<T> = {
  id: string;
  data: T;
  selected: boolean;
  type: string;
  position: { x: number; y: number };
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build minimal props for testing. */
function makeProps(
  overrides: Partial<NRNode["data"]> &
    Pick<NRNode["data"], "type" | "label" | "color" | "inputs" | "outputs">,
  selected = false,
): NodeProps<NRNode["data"]> {
  return {
    id: "test-node-1",
    data: {
      type: overrides.type,
      label: overrides.label,
      color: overrides.color,
      icon: overrides.icon,
      inputs: overrides.inputs,
      outputs: overrides.outputs,
      status: overrides.status,
      disabled: overrides.disabled,
    },
    selected,
    type: "nrNode",
    position: { x: 0, y: 0 },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("NodeComponent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering & label
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders with correct label text", () => {
      const props = makeProps({
        type: "inject",
        label: "Inject Data",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      expect(screen.getByText("Inject Data")).toBeInTheDocument();
    });

    it("renders the label span element even when label is empty", () => {
      const props = makeProps({
        type: "debug",
        label: "",
        color: "#a6bbcf",
        inputs: 1,
        outputs: 0,
      });
      render(<NodeComponent {...props} />);
      const labelSpan = document.querySelector(".truncate");
      expect(labelSpan).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Handles / ports
  // -----------------------------------------------------------------------

  describe("handles", () => {
    it("shows no input handle when inputs is 0", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const handles = document.querySelectorAll("[data-handleid]");
      const inputHandles = Array.from(handles).filter(
        (h) => h.getAttribute("data-handleid")?.startsWith("input-"),
      );
      expect(inputHandles).toHaveLength(0);
    });

    it("shows one input handle when inputs is 1", () => {
      const props = makeProps({
        type: "function",
        label: "function",
        color: "#e2d96e",
        inputs: 1,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const handles = document.querySelectorAll("[data-handleid]");
      const inputHandles = Array.from(handles).filter(
        (h) => h.getAttribute("data-handleid")?.startsWith("input-"),
      );
      expect(inputHandles).toHaveLength(1);
    });

    it("shows no output handle when outputs is 0", () => {
      const props = makeProps({
        type: "debug",
        label: "debug",
        color: "#a6bbcf",
        inputs: 1,
        outputs: 0,
      });
      render(<NodeComponent {...props} />);
      const handles = document.querySelectorAll("[data-handleid]");
      const outputHandles = Array.from(handles).filter(
        (h) => h.getAttribute("data-handleid")?.startsWith("output-"),
      );
      expect(outputHandles).toHaveLength(0);
    });

    it("shows multiple output handles based on output count", () => {
      const props = makeProps({
        type: "switch",
        label: "switch",
        color: "#e2d96e",
        inputs: 1,
        outputs: 4,
      });
      render(<NodeComponent {...props} />);
      const handles = document.querySelectorAll("[data-handleid]");
      const outputHandles = Array.from(handles).filter(
        (h) => h.getAttribute("data-handleid")?.startsWith("output-"),
      );
      expect(outputHandles).toHaveLength(4);
    });

    it("renders input handles on the left", () => {
      const props = makeProps({
        type: "function",
        label: "function",
        color: "#e2d96e",
        inputs: 1,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const handle = document.querySelector('[data-handleposition="left"]');
      expect(handle).toBeInTheDocument();
      expect(handle?.getAttribute("data-handletype")).toBe("target");
    });

    it("renders output handles on the right", () => {
      const props = makeProps({
        type: "function",
        label: "function",
        color: "#e2d96e",
        inputs: 1,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const handle = document.querySelector('[data-handleposition="right"]');
      expect(handle).toBeInTheDocument();
      expect(handle?.getAttribute("data-handletype")).toBe("source");
    });
  });

  // -----------------------------------------------------------------------
  // Category colour
  // -----------------------------------------------------------------------

  describe("category color", () => {
    it("applies the category color as background", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).toBeInTheDocument();
      expect(nodeBody).toHaveStyle({ backgroundColor: "#a6bbcf" });
    });

    it("uses a different color for function category", () => {
      const props = makeProps({
        type: "function",
        label: "function",
        color: "#e2d96e",
        inputs: 1,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).toHaveStyle({ backgroundColor: "#e2d96e" });
    });

    it("uses green color for parser category", () => {
      const props = makeProps({
        type: "json",
        label: "json",
        color: "#c0edc0",
        inputs: 1,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).toHaveStyle({ backgroundColor: "#c0edc0" });
    });
  });

  // -----------------------------------------------------------------------
  // Status indicator
  // -----------------------------------------------------------------------

  describe("status indicator", () => {
    it("shows status indicator when status is set", () => {
      const props = makeProps({
        type: "mqtt in",
        label: "mqtt in",
        color: "#e2d96e",
        inputs: 0,
        outputs: 1,
        status: "ok",
      });
      render(<NodeComponent {...props} />);
      expect(screen.getByTestId("node-status-indicator")).toBeInTheDocument();
    });

    it("does not show status indicator when status is undefined", () => {
      const props = makeProps({
        type: "mqtt in",
        label: "mqtt in",
        color: "#e2d96e",
        inputs: 0,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      expect(
        screen.queryByTestId("node-status-indicator"),
      ).not.toBeInTheDocument();
    });

    it("uses green color for 'ok' status", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        status: "ok",
      });
      render(<NodeComponent {...props} />);
      const dot = screen.getByTestId("node-status-indicator");
      expect(dot).toHaveStyle({ backgroundColor: "#5ea652" });
    });

    it("uses red color for 'error' status", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        status: "error",
      });
      render(<NodeComponent {...props} />);
      const dot = screen.getByTestId("node-status-indicator");
      expect(dot).toHaveStyle({ backgroundColor: "#e05555" });
    });

    it("uses yellow color for 'warning' status", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        status: "warning",
      });
      render(<NodeComponent {...props} />);
      const dot = screen.getByTestId("node-status-indicator");
      expect(dot).toHaveStyle({ backgroundColor: "#d7d240" });
    });

    it("uses grey color for 'disconnected' status", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        status: "disconnected",
      });
      render(<NodeComponent {...props} />);
      const dot = screen.getByTestId("node-status-indicator");
      expect(dot).toHaveStyle({ backgroundColor: "#aaa" });
    });
  });

  // -----------------------------------------------------------------------
  // Selected state
  // -----------------------------------------------------------------------

  describe("selected state", () => {
    it("applies selected border style when selected is true", () => {
      const props = makeProps(
        {
          type: "inject",
          label: "inject",
          color: "#a6bbcf",
          inputs: 0,
          outputs: 1,
        },
        true,
      );
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).toHaveClass("border-[#4a90d9]");
    });

    it("does not apply selected border when selected is false", () => {
      const props = makeProps(
        {
          type: "inject",
          label: "inject",
          color: "#a6bbcf",
          inputs: 0,
          outputs: 1,
        },
        false,
      );
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).not.toHaveClass("border-[#4a90d9]");
    });
  });

  // -----------------------------------------------------------------------
  // Disabled state
  // -----------------------------------------------------------------------

  describe("disabled state", () => {
    it("applies reduced opacity when disabled", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        disabled: true,
      });
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).toHaveClass("opacity-50");
    });

    it("does not apply reduced opacity when not disabled", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        disabled: false,
      });
      render(<NodeComponent {...props} />);
      const nodeBody = document.querySelector(".rounded-\\[6px\\]");
      expect(nodeBody).not.toHaveClass("opacity-50");
    });
  });

  // -----------------------------------------------------------------------
  // Icon
  // -----------------------------------------------------------------------

  describe("icon", () => {
    it("renders an icon element when icon is provided", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
        icon: "fa-solid fa-arrow-right",
      });
      render(<NodeComponent {...props} />);
      const icon = document.querySelector("i.fa-solid.fa-arrow-right");
      expect(icon).toBeInTheDocument();
    });

    it("does not render an icon element when icon is omitted", () => {
      const props = makeProps({
        type: "inject",
        label: "inject",
        color: "#a6bbcf",
        inputs: 0,
        outputs: 1,
      });
      render(<NodeComponent {...props} />);
      const icon = document.querySelector("i");
      expect(icon).not.toBeInTheDocument();
    });
  });
});
