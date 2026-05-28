import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ExportDialog } from "../ExportDialog";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const sampleJson = JSON.stringify(
  [{ id: "abc", type: "inject", x: 100, y: 200, wires: [] }],
  null,
  2,
);

describe("ExportDialog", () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    json: sampleJson,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders when open", () => {
      render(<ExportDialog {...defaultProps} />);
      expect(screen.getByTestId("export-dialog")).toBeInTheDocument();
    });

    it("does not render when closed", () => {
      render(<ExportDialog {...defaultProps} open={false} />);
      expect(screen.queryByTestId("export-dialog")).not.toBeInTheDocument();
    });

    it("renders the title", () => {
      render(<ExportDialog {...defaultProps} />);
      expect(screen.getByText("Export Flows")).toBeInTheDocument();
    });

    it("renders the textarea with JSON content", () => {
      render(<ExportDialog {...defaultProps} />);
      const textarea = screen.getByTestId("export-textarea");
      expect(textarea).toBeInTheDocument();
      expect(textarea).toHaveValue(sampleJson);
    });

    it("textarea is read-only", () => {
      render(<ExportDialog {...defaultProps} />);
      const textarea = screen.getByTestId("export-textarea");
      expect(textarea).toHaveAttribute("readonly");
    });

    it("renders the copy button", () => {
      render(<ExportDialog {...defaultProps} />);
      expect(screen.getByTestId("export-copy")).toBeInTheDocument();
    });

    it("renders the download button", () => {
      render(<ExportDialog {...defaultProps} />);
      expect(screen.getByTestId("export-download")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Copy behavior
  // -----------------------------------------------------------------------

  describe("copy behavior", () => {
    it("shows 'Copy to clipboard' initially", () => {
      render(<ExportDialog {...defaultProps} />);
      expect(screen.getByText("Copy to clipboard")).toBeInTheDocument();
    });

    it("calls clipboard.writeText with the JSON content", async () => {
      const writeText = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, {
        clipboard: { writeText },
      });

      render(<ExportDialog {...defaultProps} />);

      const copyBtn = screen.getByTestId("export-copy");
      fireEvent.click(copyBtn);

      await vi.waitFor(() => {
        expect(writeText).toHaveBeenCalledWith(sampleJson);
      });
    });
  });

  // -----------------------------------------------------------------------
  // Download behavior
  // -----------------------------------------------------------------------

  describe("download behavior", () => {
    it("creates a download link when clicked", () => {
      // Mock Blob and URL APIs
      const createObjectURL = vi.fn(() => "blob:mock-url");
      const revokeObjectURL = vi.fn();
      global.URL.createObjectURL = createObjectURL;
      global.URL.revokeObjectURL = revokeObjectURL;

      render(<ExportDialog {...defaultProps} />);

      const downloadBtn = screen.getByTestId("export-download");
      fireEvent.click(downloadBtn);

      // Verify Blob URL was created
      expect(createObjectURL).toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // Close behavior
  // -----------------------------------------------------------------------

  describe("close behavior", () => {
    it("calls onClose when X button is clicked", () => {
      render(<ExportDialog {...defaultProps} />);

      const closeBtn = screen.getByLabelText("Close");
      fireEvent.click(closeBtn);
      expect(defaultProps.onClose).toHaveBeenCalled();
    });
  });
});
