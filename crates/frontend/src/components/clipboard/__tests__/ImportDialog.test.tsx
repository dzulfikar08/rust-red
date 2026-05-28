import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ImportDialog } from "../ImportDialog";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

describe("ImportDialog", () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onImport: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders when open", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByTestId("import-dialog")).toBeInTheDocument();
    });

    it("does not render when closed", () => {
      render(<ImportDialog {...defaultProps} open={false} />);
      expect(screen.queryByTestId("import-dialog")).not.toBeInTheDocument();
    });

    it("renders the title", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByText("Import Flows")).toBeInTheDocument();
    });

    it("renders the textarea", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByTestId("import-textarea")).toBeInTheDocument();
    });

    it("renders the import button", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByTestId("import-button")).toBeInTheDocument();
    });

    it("renders the cancel button", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByTestId("import-cancel")).toBeInTheDocument();
    });

    it("import button is disabled when textarea is empty", () => {
      render(<ImportDialog {...defaultProps} />);
      expect(screen.getByTestId("import-button")).toBeDisabled();
    });
  });

  // -----------------------------------------------------------------------
  // Import behavior
  // -----------------------------------------------------------------------

  describe("import behavior", () => {
    it("calls onImport with valid JSON", () => {
      render(<ImportDialog {...defaultProps} />);

      const textarea = screen.getByTestId("import-textarea");
      const json = JSON.stringify([{ id: "abc", type: "inject" }]);
      fireEvent.change(textarea, { target: { value: json } });

      const importBtn = screen.getByTestId("import-button");
      fireEvent.click(importBtn);

      expect(defaultProps.onImport).toHaveBeenCalledWith(json);
    });

    it("shows error for invalid JSON", () => {
      render(<ImportDialog {...defaultProps} />);

      const textarea = screen.getByTestId("import-textarea");
      fireEvent.change(textarea, { target: { value: "not valid json" } });

      const importBtn = screen.getByTestId("import-button");
      fireEvent.click(importBtn);

      expect(screen.getByTestId("import-error")).toBeInTheDocument();
      expect(defaultProps.onImport).not.toHaveBeenCalled();
    });

    it("enables import button when textarea has content", () => {
      render(<ImportDialog {...defaultProps} />);

      const textarea = screen.getByTestId("import-textarea");
      fireEvent.change(textarea, { target: { value: "{}" } });

      expect(screen.getByTestId("import-button")).toBeEnabled();
    });

    it("shows error for whitespace-only content via empty check", () => {
      render(<ImportDialog {...defaultProps} />);

      // The button is disabled for empty/whitespace content, so typing
      // just whitespace should keep the button disabled
      const textarea = screen.getByTestId("import-textarea");
      fireEvent.change(textarea, { target: { value: "   " } });

      const importBtn = screen.getByTestId("import-button");
      expect(importBtn).toBeDisabled();
    });
  });

  // -----------------------------------------------------------------------
  // Close / Cancel behavior
  // -----------------------------------------------------------------------

  describe("close behavior", () => {
    it("calls onClose when cancel is clicked", () => {
      render(<ImportDialog {...defaultProps} />);

      fireEvent.click(screen.getByTestId("import-cancel"));
      expect(defaultProps.onClose).toHaveBeenCalled();
    });

    it("calls onClose when X button is clicked", () => {
      render(<ImportDialog {...defaultProps} />);

      const closeBtn = screen.getByLabelText("Close");
      fireEvent.click(closeBtn);
      expect(defaultProps.onClose).toHaveBeenCalled();
    });
  });
});
