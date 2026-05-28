/**
 * Tests for the Header component
 *
 * We mock stores and the DeployButton to isolate Header rendering.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Header } from "../Header";

// ---------------------------------------------------------------------------
// Mock stores
// ---------------------------------------------------------------------------

vi.mock("../../../store/theme-store", () => ({
  useThemeStore: () => ({
    theme: "dark",
    toggleTheme: vi.fn(),
  }),
}));

vi.mock("../../deploy/Deploy", () => ({
  DeployButton: () => <button data-testid="deploy-button">Deploy</button>,
}));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Header", () => {
  const defaultProps = {
    sidebarOpen: false,
    onToggleSidebar: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the header element", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header")).toBeInTheDocument();
    });

    it("renders the application logo text", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByText("Node-RED")).toBeInTheDocument();
    });

    it("renders the hamburger menu button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header-menu-btn")).toBeInTheDocument();
    });

    it("renders the manage palette button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header-palette-btn")).toBeInTheDocument();
    });

    it("renders the user settings button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header-user-btn")).toBeInTheDocument();
    });

    it("renders the deploy button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("deploy-button")).toBeInTheDocument();
    });

    it("renders the sidebar toggle button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header-sidebar-toggle")).toBeInTheDocument();
    });

    it("renders the theme toggle button", () => {
      render(<Header {...defaultProps} />);
      expect(screen.getByTestId("header-theme-toggle")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Interactions
  // -----------------------------------------------------------------------

  describe("interactions", () => {
    it("calls onToggleSidebar when sidebar toggle is clicked", () => {
      const onToggleSidebar = vi.fn();
      render(<Header {...defaultProps} onToggleSidebar={onToggleSidebar} />);
      fireEvent.click(screen.getByTestId("header-sidebar-toggle"));
      expect(onToggleSidebar).toHaveBeenCalledOnce();
    });

    it("opens main menu dropdown on hamburger click", () => {
      render(<Header {...defaultProps} />);
      fireEvent.click(screen.getByTestId("header-menu-btn"));
      expect(screen.getByText("Flows")).toBeInTheDocument();
      expect(screen.getByText("Import")).toBeInTheDocument();
      expect(screen.getByText("Export")).toBeInTheDocument();
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    it("opens user menu dropdown on user button click", () => {
      render(<Header {...defaultProps} />);
      fireEvent.click(screen.getByTestId("header-user-btn"));
      expect(screen.getByText("Preferences")).toBeInTheDocument();
      expect(screen.getByText("Keyboard shortcuts")).toBeInTheDocument();
      expect(screen.getByText("About")).toBeInTheDocument();
    });
  });
});
