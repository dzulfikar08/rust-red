/**
 * Tests for ZoomControls component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ZoomControls } from "../ZoomControls";

describe("ZoomControls", () => {
  const defaultProps = {
    onZoomIn: vi.fn(),
    onZoomOut: vi.fn(),
    onZoomToFit: vi.fn(),
    zoomLevel: 1,
  };

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the zoom controls container", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-controls")).toBeInTheDocument();
    });

    it("renders zoom out button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-out-btn")).toBeInTheDocument();
    });

    it("renders zoom in button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-in-btn")).toBeInTheDocument();
    });

    it("renders zoom to fit button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-fit-btn")).toBeInTheDocument();
    });

    it("renders zoom level display", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-level")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Zoom level display
  // -----------------------------------------------------------------------

  describe("zoom level display", () => {
    it("shows 100% when zoomLevel is 1", () => {
      render(<ZoomControls {...defaultProps} zoomLevel={1} />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("100%");
    });

    it("shows 50% when zoomLevel is 0.5", () => {
      render(<ZoomControls {...defaultProps} zoomLevel={0.5} />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("50%");
    });

    it("shows 200% when zoomLevel is 2.0", () => {
      render(<ZoomControls {...defaultProps} zoomLevel={2.0} />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("200%");
    });

    it("shows 10% when zoomLevel is 0.1 (minimum)", () => {
      render(<ZoomControls {...defaultProps} zoomLevel={0.1} />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("10%");
    });

    it("rounds fractional percentages", () => {
      render(<ZoomControls {...defaultProps} zoomLevel={0.734} />);
      expect(screen.getByTestId("zoom-level").textContent).toBe("73%");
    });
  });

  // -----------------------------------------------------------------------
  // Button clicks
  // -----------------------------------------------------------------------

  describe("button interactions", () => {
    it("calls onZoomOut when zoom out button is clicked", () => {
      const onZoomOut = vi.fn();
      render(<ZoomControls {...defaultProps} onZoomOut={onZoomOut} />);
      fireEvent.click(screen.getByTestId("zoom-out-btn"));
      expect(onZoomOut).toHaveBeenCalledTimes(1);
    });

    it("calls onZoomIn when zoom in button is clicked", () => {
      const onZoomIn = vi.fn();
      render(<ZoomControls {...defaultProps} onZoomIn={onZoomIn} />);
      fireEvent.click(screen.getByTestId("zoom-in-btn"));
      expect(onZoomIn).toHaveBeenCalledTimes(1);
    });

    it("calls onZoomToFit when fit button is clicked", () => {
      const onZoomToFit = vi.fn();
      render(<ZoomControls {...defaultProps} onZoomToFit={onZoomToFit} />);
      fireEvent.click(screen.getByTestId("zoom-fit-btn"));
      expect(onZoomToFit).toHaveBeenCalledTimes(1);
    });
  });

  // -----------------------------------------------------------------------
  // Accessibility
  // -----------------------------------------------------------------------

  describe("accessibility", () => {
    it("has aria-label on zoom out button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-out-btn")).toHaveAttribute(
        "aria-label",
        "Zoom out",
      );
    });

    it("has aria-label on zoom in button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-in-btn")).toHaveAttribute(
        "aria-label",
        "Zoom in",
      );
    });

    it("has aria-label on zoom to fit button", () => {
      render(<ZoomControls {...defaultProps} />);
      expect(screen.getByTestId("zoom-fit-btn")).toHaveAttribute(
        "aria-label",
        "Zoom to fit",
      );
    });
  });
});
