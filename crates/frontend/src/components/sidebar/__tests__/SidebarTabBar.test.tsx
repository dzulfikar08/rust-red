import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SidebarTabBar } from "../SidebarTabBar";
import { useSidebarStore } from "../../../store/sidebar-store";

function resetStore() {
  useSidebarStore.setState({
    activeTab: "info",
    isOpen: false,
    width: 320,
  });
}

describe("SidebarTabBar", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the tab bar container", () => {
      render(<SidebarTabBar />);
      expect(screen.getByTestId("sidebar-tab-bar")).toBeInTheDocument();
    });

    it("renders all six tab buttons", () => {
      render(<SidebarTabBar />);
      const tabs = ["info", "debug", "config", "context", "help", "outliner"];
      for (const tab of tabs) {
        expect(screen.getByTestId(`sidebar-tab-btn-${tab}`)).toBeInTheDocument();
      }
    });

    it("renders tab labels as aria-label", () => {
      render(<SidebarTabBar />);
      expect(screen.getByLabelText("Info")).toBeInTheDocument();
      expect(screen.getByLabelText("Debug")).toBeInTheDocument();
      expect(screen.getByLabelText("Config")).toBeInTheDocument();
      expect(screen.getByLabelText("Context")).toBeInTheDocument();
      expect(screen.getByLabelText("Help")).toBeInTheDocument();
      expect(screen.getByLabelText("Outliner")).toBeInTheDocument();
    });

    it("renders tab labels as title attributes", () => {
      render(<SidebarTabBar />);
      const tabs = ["Info", "Debug", "Config", "Context", "Help", "Outliner"];
      for (const label of tabs) {
        const btn = screen.getByTitle(label);
        expect(btn).toBeInTheDocument();
      }
    });
  });

  // -----------------------------------------------------------------------
  // Click handlers
  // -----------------------------------------------------------------------

  describe("click handlers", () => {
    it("opens sidebar when clicking a tab while closed", async () => {
      const user = userEvent.setup();
      render(<SidebarTabBar />);

      expect(useSidebarStore.getState().isOpen).toBe(false);

      await user.click(screen.getByTestId("sidebar-tab-btn-debug"));

      expect(useSidebarStore.getState().isOpen).toBe(true);
      expect(useSidebarStore.getState().activeTab).toBe("debug");
    });

    it("switches tab when clicking a different tab while open", async () => {
      const user = userEvent.setup();
      useSidebarStore.getState().open("info");
      render(<SidebarTabBar />);

      expect(useSidebarStore.getState().activeTab).toBe("info");

      await user.click(screen.getByTestId("sidebar-tab-btn-config"));

      expect(useSidebarStore.getState().activeTab).toBe("config");
    });

    it("closes sidebar when clicking active tab while open", async () => {
      const user = userEvent.setup();
      useSidebarStore.getState().open("info");
      render(<SidebarTabBar />);

      expect(useSidebarStore.getState().isOpen).toBe(true);

      await user.click(screen.getByTestId("sidebar-tab-btn-info"));

      expect(useSidebarStore.getState().isOpen).toBe(false);
    });

    it("opens sidebar with specific tab when clicking any tab while closed", async () => {
      const user = userEvent.setup();
      render(<SidebarTabBar />);

      const tabs = ["debug", "config", "context", "help", "outliner"] as const;
      for (const tab of tabs) {
        useSidebarStore.getState().close();
        await user.click(screen.getByTestId(`sidebar-tab-btn-${tab}`));
        expect(useSidebarStore.getState().isOpen).toBe(true);
        expect(useSidebarStore.getState().activeTab).toBe(tab);
      }
    });
  });

  // -----------------------------------------------------------------------
  // Active tab highlighting
  // -----------------------------------------------------------------------

  describe("active tab highlighting", () => {
    it("highlights the active tab button", () => {
      useSidebarStore.getState().setActiveTab("debug");
      render(<SidebarTabBar />);

      const debugBtn = screen.getByTestId("sidebar-tab-btn-debug");
      expect(debugBtn.className).toContain("bg-gray-300");
    });

    it("does not highlight inactive tab buttons", () => {
      useSidebarStore.getState().setActiveTab("debug");
      render(<SidebarTabBar />);

      const infoBtn = screen.getByTestId("sidebar-tab-btn-info");
      expect(infoBtn.className).not.toContain("bg-gray-300");
    });
  });
});
