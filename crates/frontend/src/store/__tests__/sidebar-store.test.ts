import { describe, it, expect, beforeEach } from "vitest";
import { useSidebarStore } from "../sidebar-store";

function resetStore() {
  useSidebarStore.setState({
    activeTab: "info",
    isOpen: false,
    width: 320,
  });
}

describe("useSidebarStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts with 'info' as active tab", () => {
      expect(useSidebarStore.getState().activeTab).toBe("info");
    });

    it("starts closed", () => {
      expect(useSidebarStore.getState().isOpen).toBe(false);
    });

    it("starts with width 320", () => {
      expect(useSidebarStore.getState().width).toBe(320);
    });
  });

  // -----------------------------------------------------------------------
  // setActiveTab
  // -----------------------------------------------------------------------

  describe("setActiveTab()", () => {
    it("changes the active tab", () => {
      useSidebarStore.getState().setActiveTab("debug");
      expect(useSidebarStore.getState().activeTab).toBe("debug");
    });

    it("can switch between tabs", () => {
      useSidebarStore.getState().setActiveTab("debug");
      expect(useSidebarStore.getState().activeTab).toBe("debug");
      useSidebarStore.getState().setActiveTab("config");
      expect(useSidebarStore.getState().activeTab).toBe("config");
    });

    it("supports all tab types", () => {
      const tabs = ["info", "debug", "config", "context", "help", "outliner"] as const;
      for (const tab of tabs) {
        useSidebarStore.getState().setActiveTab(tab);
        expect(useSidebarStore.getState().activeTab).toBe(tab);
      }
    });
  });

  // -----------------------------------------------------------------------
  // toggle
  // -----------------------------------------------------------------------

  describe("toggle()", () => {
    it("opens a closed sidebar", () => {
      expect(useSidebarStore.getState().isOpen).toBe(false);
      useSidebarStore.getState().toggle();
      expect(useSidebarStore.getState().isOpen).toBe(true);
    });

    it("closes an open sidebar", () => {
      useSidebarStore.getState().toggle(); // open
      useSidebarStore.getState().toggle(); // close
      expect(useSidebarStore.getState().isOpen).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // open
  // -----------------------------------------------------------------------

  describe("open()", () => {
    it("opens the sidebar without changing tab", () => {
      useSidebarStore.getState().setActiveTab("debug");
      useSidebarStore.getState().open();
      expect(useSidebarStore.getState().isOpen).toBe(true);
      expect(useSidebarStore.getState().activeTab).toBe("debug");
    });

    it("opens the sidebar with a specific tab", () => {
      useSidebarStore.getState().open("config");
      expect(useSidebarStore.getState().isOpen).toBe(true);
      expect(useSidebarStore.getState().activeTab).toBe("config");
    });
  });

  // -----------------------------------------------------------------------
  // close
  // -----------------------------------------------------------------------

  describe("close()", () => {
    it("closes the sidebar", () => {
      useSidebarStore.getState().open();
      expect(useSidebarStore.getState().isOpen).toBe(true);
      useSidebarStore.getState().close();
      expect(useSidebarStore.getState().isOpen).toBe(false);
    });

    it("does not change the active tab when closing", () => {
      useSidebarStore.getState().open("debug");
      useSidebarStore.getState().close();
      expect(useSidebarStore.getState().activeTab).toBe("debug");
    });
  });

  // -----------------------------------------------------------------------
  // setWidth
  // -----------------------------------------------------------------------

  describe("setWidth()", () => {
    it("sets the sidebar width", () => {
      useSidebarStore.getState().setWidth(400);
      expect(useSidebarStore.getState().width).toBe(400);
    });

    it("can set width to minimum value", () => {
      useSidebarStore.getState().setWidth(200);
      expect(useSidebarStore.getState().width).toBe(200);
    });

    it("can set width to maximum value", () => {
      useSidebarStore.getState().setWidth(600);
      expect(useSidebarStore.getState().width).toBe(600);
    });
  });
});
