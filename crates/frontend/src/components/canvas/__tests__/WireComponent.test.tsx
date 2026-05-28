/**
 * Tests for WireComponent
 *
 * We mock @xyflow/react's BaseEdge and getBezierPath so the component can be
 * rendered standalone without the ReactFlow provider context.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";

// ---------------------------------------------------------------------------
// Mock @xyflow/react – stub BaseEdge and getBezierPath
// ---------------------------------------------------------------------------

const MOCK_PATH = "M100,50 C200,50 200,50 300,50";

vi.mock("@xyflow/react", () => {
  const React = require("react");
  return {
    BaseEdge: React.forwardRef(function BaseEdge(
      props: {
        id?: string;
        path: string;
        style?: React.CSSProperties;
        className?: string;
      },
      _ref: React.Ref<unknown>,
    ) {
      return (
        <path
          data-testid="base-edge"
          d={props.path}
          data-edgeid={props.id}
          style={props.style}
          className={props.className}
        />
      );
    }),
    getBezierPath: vi.fn(() => [MOCK_PATH, 200, 50, 0, 0]),
    Position: {
      Left: "left",
      Right: "right",
      Top: "top",
      Bottom: "bottom",
    },
  };
});

import { WireComponent } from "../WireComponent";
import { getBezierPath } from "@xyflow/react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type EdgePropsLike = {
  id: string;
  source: string;
  target: string;
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  sourcePosition: string;
  targetPosition: string;
  selected?: boolean;
  style?: React.CSSProperties;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build minimal EdgeProps for testing. */
function makeProps(overrides: Partial<EdgePropsLike> = {}): EdgePropsLike {
  return {
    id: "e1",
    source: "node1",
    target: "node2",
    sourceX: 100,
    sourceY: 50,
    targetX: 300,
    targetY: 50,
    sourcePosition: "right",
    targetPosition: "left",
    selected: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("WireComponent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders an SVG path element", () => {
      render(<WireComponent {...makeProps()} />);
      const path = screen.getByTestId("base-edge");
      expect(path).toBeInTheDocument();
      expect(path.tagName.toLowerCase()).toBe("path");
    });

    it("renders the bezier path from getBezierPath", () => {
      render(<WireComponent {...makeProps()} />);
      const path = screen.getByTestId("base-edge");
      expect(path.getAttribute("d")).toBe(MOCK_PATH);
    });

    it("calls getBezierPath with correct coordinates", () => {
      const props = makeProps({
        sourceX: 10,
        sourceY: 20,
        targetX: 200,
        targetY: 100,
      });
      render(<WireComponent {...props} />);
      expect(getBezierPath).toHaveBeenCalledWith(
        expect.objectContaining({
          sourceX: 10,
          sourceY: 20,
          targetX: 200,
          targetY: 100,
        }),
      );
    });

    it("renders inside an SVG group element", () => {
      const { container } = render(<WireComponent {...makeProps()} />);
      const g = container.querySelector("g");
      expect(g).toBeInTheDocument();
    });

    it("renders an invisible interaction path for hover detection", () => {
      const { container } = render(<WireComponent {...makeProps()} />);
      const invisiblePath = container.querySelector(
        'path[stroke="transparent"]',
      );
      expect(invisiblePath).toBeInTheDocument();
      expect(invisiblePath?.getAttribute("stroke-width")).toBe("12");
    });
  });

  // -----------------------------------------------------------------------
  // Colours
  // -----------------------------------------------------------------------

  describe("wire colour", () => {
    it("uses default grey colour (#999) when not selected", () => {
      render(<WireComponent {...makeProps({ selected: false })} />);
      const path = screen.getByTestId("base-edge");
      expect(path.style.stroke).toBe("rgb(153, 153, 153)");
    });

    it("uses orange colour (#ff7700) when selected", () => {
      render(<WireComponent {...makeProps({ selected: true })} />);
      const path = screen.getByTestId("base-edge");
      expect(path.style.stroke).toBe("rgb(255, 119, 0)");
    });

    it("applies stroke width of 2px", () => {
      render(<WireComponent {...makeProps()} />);
      const path = screen.getByTestId("base-edge");
      expect(path.style.strokeWidth).toBe("2");
    });

    it("merges custom style prop with wire styles", () => {
      render(
        <WireComponent
          {...makeProps({ style: { opacity: 0.5 } })}
        />,
      );
      const path = screen.getByTestId("base-edge");
      expect(path.style.stroke).toBe("rgb(153, 153, 153)");
      expect(path.style.opacity).toBe("0.5");
    });
  });

  // -----------------------------------------------------------------------
  // Memoization
  // -----------------------------------------------------------------------

  describe("memoization", () => {
    it("is memoized (same props = same render)", () => {
      const props = makeProps();
      const { rerender } = render(<WireComponent {...props} />);

      // Initial render calls getBezierPath once
      const callCountAfterFirst = (getBezierPath as ReturnType<typeof vi.fn>).mock.calls.length;

      // Re-render with the same props
      rerender(<WireComponent {...props} />);

      // Should not have called getBezierPath again because memo prevented re-render
      const callCountAfterSecond = (getBezierPath as ReturnType<typeof vi.fn>).mock.calls.length;
      expect(callCountAfterSecond).toBe(callCountAfterFirst);
    });
  });

  // -----------------------------------------------------------------------
  // Edge ID
  // -----------------------------------------------------------------------

  describe("edge id", () => {
    it("passes edge id to BaseEdge", () => {
      render(<WireComponent {...makeProps({ id: "wire-42" })} />);
      const path = screen.getByTestId("base-edge");
      expect(path.getAttribute("data-edgeid")).toBe("wire-42");
    });
  });
});
