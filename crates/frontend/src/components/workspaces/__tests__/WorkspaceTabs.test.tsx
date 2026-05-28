import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkspaceTabs } from "../WorkspaceTabs";
import { useWorkspaceStore } from "../../../store/workspace-store";

function resetStore() {
  useWorkspaceStore.setState({
    flows: [],
    activeFlowId: null,
  });
}

describe("WorkspaceTabs", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders with no flows", () => {
      render(<WorkspaceTabs />);
      // The add button should always be present
      expect(screen.getByLabelText("Add new flow")).toBeInTheDocument();
    });

    it("renders flow tabs", () => {
      useWorkspaceStore.getState().addFlow("Flow 1");
      useWorkspaceStore.getState().addFlow("Flow 2");
      render(<WorkspaceTabs />);
      expect(screen.getByText("Flow 1")).toBeInTheDocument();
      expect(screen.getByText("Flow 2")).toBeInTheDocument();
    });

    it("renders tab with role=tab", () => {
      useWorkspaceStore.getState().addFlow("Flow 1");
      render(<WorkspaceTabs />);
      expect(screen.getByRole("tab", { name: /Flow 1/ })).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Click to switch
  // -----------------------------------------------------------------------

  describe("click to switch active flow", () => {
    it("switches active flow on click", () => {
      useWorkspaceStore.getState().addFlow("Flow 1");
      const id2 = useWorkspaceStore.getState().addFlow("Flow 2");
      render(<WorkspaceTabs />);

      // Flow 2 tab
      const tab = screen.getByRole("tab", { name: /Flow 2/ });
      fireEvent.click(tab);

      expect(useWorkspaceStore.getState().activeFlowId).toBe(id2);
    });
  });

  // -----------------------------------------------------------------------
  // Add button
  // -----------------------------------------------------------------------

  describe("add button", () => {
    it("adds a new flow when + button is clicked", async () => {
      const user = userEvent.setup();
      render(<WorkspaceTabs />);

      const addBtn = screen.getByLabelText("Add new flow");
      await user.click(addBtn);

      expect(useWorkspaceStore.getState().flows).toHaveLength(1);
      expect(useWorkspaceStore.getState().flows[0].label).toBe("Flow 1");
    });

    it("adds multiple flows", async () => {
      const user = userEvent.setup();
      render(<WorkspaceTabs />);

      const addBtn = screen.getByLabelText("Add new flow");
      await user.click(addBtn);
      await user.click(addBtn);

      expect(useWorkspaceStore.getState().flows).toHaveLength(2);
    });
  });

  // -----------------------------------------------------------------------
  // Close button
  // -----------------------------------------------------------------------

  describe("close button", () => {
    it("removes a flow when close button is clicked", () => {
      const id = useWorkspaceStore.getState().addFlow("Close Me");
      render(<WorkspaceTabs />);

      const closeBtn = screen.getByLabelText(`Close Close Me`);
      fireEvent.click(closeBtn);

      expect(useWorkspaceStore.getState().flows).toHaveLength(0);
      expect(useWorkspaceStore.getState().activeFlowId).toBeNull();
    });

    it("switches active flow when active flow is closed", () => {
      const id1 = useWorkspaceStore.getState().addFlow("Flow 1");
      const id2 = useWorkspaceStore.getState().addFlow("Flow 2");
      // Make Flow 2 active
      useWorkspaceStore.getState().setActiveFlow(id2);
      render(<WorkspaceTabs />);

      const closeBtn = screen.getByLabelText("Close Flow 2");
      fireEvent.click(closeBtn);

      expect(useWorkspaceStore.getState().activeFlowId).toBe(id1);
      expect(useWorkspaceStore.getState().flows).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // Double-click to rename
  // -----------------------------------------------------------------------

  describe("double-click to rename", () => {
    it("enters edit mode on double-click", async () => {
      useWorkspaceStore.getState().addFlow("Editable");
      const user = userEvent.setup();
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Editable/ });
      await user.dblClick(tab);

      // Should show an input field
      const input = tab.querySelector("input");
      expect(input).toBeInTheDocument();
      expect(input).toHaveValue("Editable");
    });

    it("renames the flow when Enter is pressed", async () => {
      useWorkspaceStore.getState().addFlow("Original");
      const user = userEvent.setup();
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Original/ });
      await user.dblClick(tab);

      const input = tab.querySelector("input")!;
      await user.clear(input);
      await user.type(input, "Renamed{Enter}");

      expect(useWorkspaceStore.getState().flows[0].label).toBe("Renamed");
    });
  });

  // -----------------------------------------------------------------------
  // Context menu
  // -----------------------------------------------------------------------

  describe("context menu", () => {
    it("opens context menu on right-click", () => {
      useWorkspaceStore.getState().addFlow("Context Flow");
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Context Flow/ });
      fireEvent.contextMenu(tab);

      expect(screen.getByText("Rename")).toBeInTheDocument();
      expect(screen.getByText("Disable")).toBeInTheDocument();
    });

    it("deletes flow via context menu", () => {
      useWorkspaceStore.getState().addFlow("Delete Me");
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Delete Me/ });
      fireEvent.contextMenu(tab);

      const deleteBtn = screen.getByText(/Delete "Delete Me"/);
      fireEvent.click(deleteBtn);

      expect(useWorkspaceStore.getState().flows).toHaveLength(0);
    });

    it("disables flow via context menu", () => {
      useWorkspaceStore.getState().addFlow("Disable Me");
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Disable Me/ });
      fireEvent.contextMenu(tab);

      const disableBtn = screen.getByText("Disable");
      fireEvent.click(disableBtn);

      expect(useWorkspaceStore.getState().flows[0].disabled).toBe(true);
    });

    it("closes context menu on backdrop click", () => {
      useWorkspaceStore.getState().addFlow("Flow");
      render(<WorkspaceTabs />);

      const tab = screen.getByRole("tab", { name: /Flow/ });
      fireEvent.contextMenu(tab);

      expect(screen.getByText("Rename")).toBeInTheDocument();

      // Click the backdrop (the fixed overlay)
      const backdrop = document.querySelector(".fixed.inset-0") as HTMLElement;
      fireEvent.click(backdrop);

      expect(screen.queryByText("Rename")).not.toBeInTheDocument();
    });
  });
});
