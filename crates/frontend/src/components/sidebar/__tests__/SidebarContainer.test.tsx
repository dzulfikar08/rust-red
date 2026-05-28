import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SidebarContainer } from "../SidebarContainer";
import { useSidebarStore } from "../../../store/sidebar-store";

function resetStore() {
  useSidebarStore.setState({
    activeTab: "info",
    isOpen: false,
    width: 320,
  });
}

describe("SidebarContainer", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the sidebar container", () => {
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-container")).toBeInTheDocument();
    });

    it("renders the tab bar", () => {
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-bar")).toBeInTheDocument();
    });

    it("does not show content panel when closed", () => {
      render(<SidebarContainer />);
      expect(screen.queryByTestId("sidebar-content-panel")).not.toBeInTheDocument();
    });

    it("shows content panel when open", () => {
      useSidebarStore.getState().open();
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-content-panel")).toBeInTheDocument();
    });

    it("renders all six tab buttons", () => {
      render(<SidebarContainer />);
      const tabs = ["info", "debug", "config", "context", "help", "outliner"];
      for (const tab of tabs) {
        expect(screen.getByTestId(`sidebar-tab-btn-${tab}`)).toBeInTheDocument();
      }
    });
  });

  // -----------------------------------------------------------------------
  // Tab switching
  // -----------------------------------------------------------------------

  describe("tab switching", () => {
    it("shows info content when info tab is active and open", () => {
      useSidebarStore.getState().open("info");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-info")).toBeInTheDocument();
    });

    it("shows debug content when debug tab is active", () => {
      useSidebarStore.getState().open("debug");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-debug")).toBeInTheDocument();
    });

    it("shows config content when config tab is active", () => {
      useSidebarStore.getState().open("config");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-config")).toBeInTheDocument();
    });

    it("shows context content when context tab is active", () => {
      useSidebarStore.getState().open("context");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-context")).toBeInTheDocument();
    });

    it("shows help content when help tab is active", () => {
      useSidebarStore.getState().open("help");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-help")).toBeInTheDocument();
    });

    it("shows outliner content when outliner tab is active", () => {
      useSidebarStore.getState().open("outliner");
      render(<SidebarContainer />);
      expect(screen.getByTestId("sidebar-tab-content-outliner")).toBeInTheDocument();
    });

    it("opens sidebar and switches content when clicking a different tab", async () => {
      const user = userEvent.setup();
      render(<SidebarContainer />);

      // Sidebar is closed initially
      expect(screen.queryByTestId("sidebar-content-panel")).not.toBeInTheDocument();

      // Click debug tab
      await user.click(screen.getByTestId("sidebar-tab-btn-debug"));

      // Sidebar should be open with debug content
      expect(useSidebarStore.getState().isOpen).toBe(true);
      expect(useSidebarStore.getState().activeTab).toBe("debug");
    });
  });

  // -----------------------------------------------------------------------
  // Tab toggle behavior
  // -----------------------------------------------------------------------

  describe("tab toggle behavior", () => {
    it("closes sidebar when clicking active tab while open", async () => {
      const user = userEvent.setup();
      useSidebarStore.getState().open("info");
      render(<SidebarContainer />);

      expect(useSidebarStore.getState().isOpen).toBe(true);

      // Click the active tab (info) to close
      await user.click(screen.getByTestId("sidebar-tab-btn-info"));
      expect(useSidebarStore.getState().isOpen).toBe(false);
    });
  });
});
