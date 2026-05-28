import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LibraryDialog } from "../LibraryDialog";
import { useLibraryStore, type LibraryEntry } from "../../../store/library-store";
import { eventBus } from "../../../red/core/events";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useLibraryStore.setState({ entries: [], isLoading: false });
  eventBus.clear();
}

/**
 * Seed the library store with test entries.
 */
function seedEntries() {
  useLibraryStore.getState().saveToLibrary({
    name: "Inject Flow",
    path: "/flows",
    type: "flow",
    content: JSON.stringify([
      { id: "n1", type: "inject", wires: [["n2"]] },
      { id: "n2", type: "debug", wires: [] },
    ]),
  });
  useLibraryStore.getState().saveToLibrary({
    name: "Helper Function",
    path: "/utils",
    type: "function",
    content: "return msg;",
  });
  useLibraryStore.getState().saveToLibrary({
    name: "Email Template",
    path: "/templates",
    type: "template",
    content: JSON.stringify({ subject: "Hello", body: "World" }),
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("LibraryDialog", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("does not render when closed", () => {
      render(
        <LibraryDialog isOpen={false} onClose={() => {}} />,
      );
      expect(screen.queryByTestId("library-dialog")).not.toBeInTheDocument();
    });

    it("renders when open", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-dialog")).toBeInTheDocument();
    });

    it("renders the filter input", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-filter-input")).toBeInTheDocument();
    });

    it("renders the entry list panel", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-entry-list")).toBeInTheDocument();
    });

    it("renders the preview panel", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-preview")).toBeInTheDocument();
    });

    it("renders the save button", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-save-button")).toBeInTheDocument();
    });

    it("renders empty state when no entries", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );
      expect(screen.getByTestId("library-empty")).toBeInTheDocument();
      expect(screen.getByText(/Library is empty/)).toBeInTheDocument();
    });

    it("renders entries when store has data", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      // Should have 3 entries in the list
      expect(screen.getByText("Inject Flow")).toBeInTheDocument();
      expect(screen.getByText("Helper Function")).toBeInTheDocument();
      expect(screen.getByText("Email Template")).toBeInTheDocument();
    });

    it("renders type badges for entries", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      expect(screen.getByTestId("library-type-badge-flow")).toBeInTheDocument();
      expect(screen.getByTestId("library-type-badge-function")).toBeInTheDocument();
      expect(screen.getByTestId("library-type-badge-template")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Selection & Preview
  // -----------------------------------------------------------------------

  describe("selection and preview", () => {
    it("shows preview when an entry is clicked", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      // Click on an entry
      const entries = useLibraryStore.getState().entries;
      const firstEntry = entries.find((e) => e.name === "Inject Flow")!;
      fireEvent.click(screen.getByTestId(`library-entry-${firstEntry.id}`));

      // Preview panel should show the entry name (appears in both list and preview)
      const preview = screen.getByTestId("library-preview");
      expect(preview.textContent).toContain("Inject Flow");
    });

    it("shows formatted JSON in preview for valid JSON content", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const entries = useLibraryStore.getState().entries;
      const flowEntry = entries.find((e) => e.name === "Inject Flow")!;
      fireEvent.click(screen.getByTestId(`library-entry-${flowEntry.id}`));

      // Preview should show JSON content
      const preview = screen.getByTestId("library-preview");
      expect(preview.textContent).toContain("inject");
    });

    it("shows action buttons when an entry is selected", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const entries = useLibraryStore.getState().entries;
      fireEvent.click(screen.getByTestId(`library-entry-${entries[0].id}`));

      expect(screen.getByTestId("library-load-button")).toBeInTheDocument();
      expect(screen.getByTestId("library-rename-button")).toBeInTheDocument();
      expect(screen.getByTestId("library-delete-button")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Save
  // -----------------------------------------------------------------------

  describe("save", () => {
    it("opens save form when Save to Library is clicked", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      fireEvent.click(screen.getByTestId("library-save-button"));

      expect(screen.getByTestId("library-save-form")).toBeInTheDocument();
    });

    it("saves a new entry when form is submitted", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      fireEvent.click(screen.getByTestId("library-save-button"));

      const nameInput = screen.getByTestId("library-save-name");
      fireEvent.change(nameInput, { target: { value: "My New Flow" } });

      const submitButton = screen.getByTestId("library-save-submit");
      fireEvent.click(submitButton);

      // Should switch back to browse mode and have the new entry
      expect(useLibraryStore.getState().entries).toHaveLength(1);
      expect(useLibraryStore.getState().entries[0].name).toBe("My New Flow");
    });

    it("does not save when name is empty", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      fireEvent.click(screen.getByTestId("library-save-button"));

      const submitButton = screen.getByTestId("library-save-submit");
      expect(submitButton).toBeDisabled();
    });

    it("returns to browse mode on cancel", () => {
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      fireEvent.click(screen.getByTestId("library-save-button"));
      fireEvent.click(screen.getByTestId("library-save-cancel"));

      expect(screen.queryByTestId("library-save-form")).not.toBeInTheDocument();
      expect(screen.getByTestId("library-entry-list")).toBeInTheDocument();
    });

    it("uses provided initial content and type", () => {
      render(
        <LibraryDialog
          isOpen={true}
          onClose={() => {}}
          initialContent='{"test": true}'
          initialType="function"
        />,
      );

      fireEvent.click(screen.getByTestId("library-save-button"));

      const typeSelect = screen.getByTestId("library-save-type") as HTMLSelectElement;
      expect(typeSelect.value).toBe("function");
    });
  });

  // -----------------------------------------------------------------------
  // Load
  // -----------------------------------------------------------------------

  describe("load", () => {
    it("calls onLoad with the selected entry and closes dialog", () => {
      seedEntries();
      const onClose = vi.fn();
      const onLoad = vi.fn();

      render(
        <LibraryDialog isOpen={true} onClose={onClose} onLoad={onLoad} />,
      );

      const entries = useLibraryStore.getState().entries;
      const firstEntry = entries[0];
      fireEvent.click(screen.getByTestId(`library-entry-${firstEntry.id}`));
      fireEvent.click(screen.getByTestId("library-load-button"));

      expect(onLoad).toHaveBeenCalledOnce();
      const loadedEntry = onLoad.mock.calls[0][0] as LibraryEntry;
      expect(loadedEntry.id).toBe(firstEntry.id);
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("does not render load button when no entry is selected", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      expect(screen.queryByTestId("library-load-button")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Delete
  // -----------------------------------------------------------------------

  describe("delete", () => {
    it("deletes the selected entry", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      expect(useLibraryStore.getState().entries).toHaveLength(3);

      const entries = useLibraryStore.getState().entries;
      fireEvent.click(screen.getByTestId(`library-entry-${entries[0].id}`));
      fireEvent.click(screen.getByTestId("library-delete-button"));

      expect(useLibraryStore.getState().entries).toHaveLength(2);
    });
  });

  // -----------------------------------------------------------------------
  // Rename
  // -----------------------------------------------------------------------

  describe("rename", () => {
    it("shows rename input when Rename is clicked", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const entries = useLibraryStore.getState().entries;
      fireEvent.click(screen.getByTestId(`library-entry-${entries[0].id}`));
      fireEvent.click(screen.getByTestId("library-rename-button"));

      expect(screen.getByTestId("library-rename-input")).toBeInTheDocument();
    });

    it("renames entry on confirm", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const entries = useLibraryStore.getState().entries;
      const firstId = entries[0].id;
      fireEvent.click(screen.getByTestId(`library-entry-${firstId}`));
      fireEvent.click(screen.getByTestId("library-rename-button"));

      const renameInput = screen.getByTestId("library-rename-input");
      fireEvent.change(renameInput, { target: { value: "Renamed Flow" } });
      fireEvent.keyDown(renameInput, { key: "Enter" });

      const updated = useLibraryStore.getState().loadFromLibrary(firstId);
      expect(updated!.name).toBe("Renamed Flow");
    });

    it("cancels rename on Escape", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const entries = useLibraryStore.getState().entries;
      const originalName = entries[0].name;
      const firstId = entries[0].id;
      fireEvent.click(screen.getByTestId(`library-entry-${firstId}`));
      fireEvent.click(screen.getByTestId("library-rename-button"));

      const renameInput = screen.getByTestId("library-rename-input");
      fireEvent.change(renameInput, { target: { value: "Should Not Apply" } });
      fireEvent.keyDown(renameInput, { key: "Escape" });

      expect(screen.queryByTestId("library-rename-input")).not.toBeInTheDocument();
      const entry = useLibraryStore.getState().loadFromLibrary(firstId);
      expect(entry!.name).toBe(originalName);
    });
  });

  // -----------------------------------------------------------------------
  // Filter
  // -----------------------------------------------------------------------

  describe("filter", () => {
    it("filters entries by name", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      // Verify all entries are in the entry list initially
      const entryList = screen.getByTestId("library-entry-list");
      expect(entryList.textContent).toContain("Inject Flow");
      expect(entryList.textContent).toContain("Helper Function");
      expect(entryList.textContent).toContain("Email Template");

      const filterInput = screen.getByTestId("library-filter-input");
      fireEvent.change(filterInput, { target: { value: "email" } });

      // After filtering, only Email Template should be in the entry list
      expect(entryList.textContent).not.toContain("Inject Flow");
      expect(entryList.textContent).not.toContain("Helper Function");
      expect(entryList.textContent).toContain("Email Template");
    });

    it("shows no matching entries message when filter matches nothing", () => {
      seedEntries();
      render(
        <LibraryDialog isOpen={true} onClose={() => {}} />,
      );

      const filterInput = screen.getByTestId("library-filter-input");
      fireEvent.change(filterInput, { target: { value: "xyznonexistent" } });

      expect(screen.getByTestId("library-empty")).toBeInTheDocument();
      expect(screen.getByText(/No matching entries/)).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Close
  // -----------------------------------------------------------------------

  describe("close", () => {
    it("calls onClose when close button is clicked", () => {
      const onClose = vi.fn();
      render(
        <LibraryDialog isOpen={true} onClose={onClose} />,
      );

      fireEvent.click(screen.getByTestId("library-close-button"));
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("calls onClose when backdrop is clicked", () => {
      const onClose = vi.fn();
      render(
        <LibraryDialog isOpen={true} onClose={onClose} />,
      );

      fireEvent.click(screen.getByTestId("library-dialog-backdrop"));
      expect(onClose).toHaveBeenCalledOnce();
    });

    it("does not call onClose when dialog content is clicked", () => {
      const onClose = vi.fn();
      render(
        <LibraryDialog isOpen={true} onClose={onClose} />,
      );

      // Click on the dialog itself, not the backdrop
      fireEvent.click(screen.getByTestId("library-dialog"));
      expect(onClose).not.toHaveBeenCalled();
    });
  });
});
