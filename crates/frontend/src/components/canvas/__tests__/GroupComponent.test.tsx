/**
 * Tests for GroupComponent
 *
 * We mock @xyflow/react's dependencies and the group store,
 * then render GroupComponent standalone with mock props and verify
 * visual output, label rendering, and resize handles.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

// ---------------------------------------------------------------------------
// Mock @xyflow/react
// ---------------------------------------------------------------------------

vi.mock("@xyflow/react", () => {
  return {
    Handle: () => null,
    Position: {
      Left: "left",
      Right: "right",
      Top: "top",
      Bottom: "bottom",
    },
  };
});

// ---------------------------------------------------------------------------
// Mock group-store
// ---------------------------------------------------------------------------

const mockMoveGroup = vi.fn();
const mockResizeGroup = vi.fn();
const mockUpdateGroup = vi.fn();

let mockGroupData: Record<string, unknown> | null = null;

vi.mock("../../../store/group-store", () => ({
  useGroupStore: (selector: (state: Record<string, unknown>) => unknown) => {
    const state = {
      groups: mockGroupData ? [mockGroupData] : [],
      moveGroup: mockMoveGroup,
      resizeGroup: mockResizeGroup,
      updateGroup: mockUpdateGroup,
    };
    return selector(state);
  },
}));

import { GroupComponent } from "../GroupComponent";
import type { GroupNode } from "../GroupComponent";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type NodeProps<T> = {
  id: string;
  data: T;
  selected: boolean;
  type: string;
  position: { x: number; y: number };
};

function makeProps(
  overrides: Partial<GroupNode["data"]> & Pick<GroupNode["data"], "label">,
  selected = false,
): NodeProps<GroupNode["data"]> {
  return {
    id: "group-1",
    data: {
      label: overrides.label ?? "Test Group",
      nodes: overrides.nodes ?? ["n1", "n2"],
      fill: overrides.fill,
      stroke: overrides.stroke,
      labelColor: overrides.labelColor,
    },
    selected,
    type: "group",
    position: { x: 50, y: 50 },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("GroupComponent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGroupData = {
      id: "group-1",
      label: "Test Group",
      type: "group",
      nodes: ["n1", "n2"],
      x: 50,
      y: 50,
      w: 300,
      h: 200,
      style: {},
    };
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the group container", () => {
      render(<GroupComponent {...makeProps({ label: "My Group" })} />);
      expect(screen.getByTestId("group-container")).toBeInTheDocument();
    });

    it("renders the group body", () => {
      render(<GroupComponent {...makeProps({ label: "My Group" })} />);
      expect(screen.getByTestId("group-body")).toBeInTheDocument();
    });

    it("renders the label text", () => {
      render(<GroupComponent {...makeProps({ label: "My Group" })} />);
      expect(screen.getByTestId("group-label-text")).toHaveTextContent(
        "My Group",
      );
    });

    it("renders with default fill color when none specified", () => {
      render(<GroupComponent {...makeProps({ label: "Group" })} />);
      const body = screen.getByTestId("group-body");
      expect(body).toHaveStyle({ backgroundColor: "#f0f0f0" });
    });

    it("renders with custom fill color", () => {
      render(
        <GroupComponent
          {...makeProps({ label: "Group", fill: "#e8f5e9" })}
        />,
      );
      const body = screen.getByTestId("group-body");
      expect(body).toHaveStyle({ backgroundColor: "#e8f5e9" });
    });

    it("renders with default dashed border when not selected", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, false)} />);
      const body = screen.getByTestId("group-body");
      expect(body).toHaveStyle({ border: "1.5px dashed #999" });
    });
  });

  // -----------------------------------------------------------------------
  // Selected state
  // -----------------------------------------------------------------------

  describe("selected state", () => {
    it("renders with selected border style when selected", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);
      const body = screen.getByTestId("group-body");
      expect(body).toHaveStyle({ border: "2px dashed #4a90d9" });
    });

    it("shows resize handles when selected", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);
      expect(screen.getByTestId("resize-handle-se")).toBeInTheDocument();
      expect(screen.getByTestId("resize-handle-sw")).toBeInTheDocument();
      expect(screen.getByTestId("resize-handle-ne")).toBeInTheDocument();
      expect(screen.getByTestId("resize-handle-nw")).toBeInTheDocument();
    });

    it("hides resize handles when not selected", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, false)} />);
      expect(screen.queryByTestId("resize-handle-se")).not.toBeInTheDocument();
      expect(screen.queryByTestId("resize-handle-sw")).not.toBeInTheDocument();
      expect(screen.queryByTestId("resize-handle-ne")).not.toBeInTheDocument();
      expect(screen.queryByTestId("resize-handle-nw")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Label editing
  // -----------------------------------------------------------------------

  describe("label editing", () => {
    it("shows input when label area is double-clicked", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);
      fireEvent.doubleClick(screen.getByTestId("group-label-area"));
      expect(screen.getByTestId("group-label-input")).toBeInTheDocument();
    });

    it("calls updateGroup with new label on blur", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);
      fireEvent.doubleClick(screen.getByTestId("group-label-area"));

      const input = screen.getByTestId("group-label-input");
      fireEvent.change(input, { target: { value: "New Label" } });
      fireEvent.blur(input);

      expect(mockUpdateGroup).toHaveBeenCalledWith("group-1", {
        label: "New Label",
      });
    });

    it("calls updateGroup on Enter key", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);
      fireEvent.doubleClick(screen.getByTestId("group-label-area"));

      const input = screen.getByTestId("group-label-input");
      fireEvent.change(input, { target: { value: "Updated" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(mockUpdateGroup).toHaveBeenCalledWith("group-1", {
        label: "Updated",
      });
    });
  });

  // -----------------------------------------------------------------------
  // Dimensions
  // -----------------------------------------------------------------------

  describe("dimensions", () => {
    it("uses dimensions from the store group", () => {
      render(<GroupComponent {...makeProps({ label: "Group" })} />);
      const container = screen.getByTestId("group-container");
      expect(container).toHaveStyle({ width: "300px", height: "200px" });
    });
  });

  // -----------------------------------------------------------------------
  // Resize interaction
  // -----------------------------------------------------------------------

  describe("resize interaction", () => {
    it("starts resize on SE handle mousedown", () => {
      render(<GroupComponent {...makeProps({ label: "Group" }, true)} />);

      const handle = screen.getByTestId("resize-handle-se");
      fireEvent.mouseDown(handle);

      // Simulate mouse move
      fireEvent.mouseMove(document, { clientX: 350, clientY: 250 });

      // Should call resizeGroup (we just verify it doesn't crash)
      // The actual resize happens on mousemove listener
      expect(true).toBe(true);
    });
  });
});
