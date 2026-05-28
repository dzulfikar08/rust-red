import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEnvVarStore } from "../../../store/env-var-store";
import { EnvVarEditor } from "../EnvVarEditor";

// ---------------------------------------------------------------------------
// Reset helper
// ---------------------------------------------------------------------------

function resetStore() {
  useEnvVarStore.setState({ vars: [] });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("EnvVarEditor", () => {
  beforeEach(() => {
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the editor container", () => {
      render(<EnvVarEditor />);
      expect(screen.getByTestId("env-var-editor")).toBeInTheDocument();
    });

    it("renders the header label", () => {
      render(<EnvVarEditor />);
      expect(
        screen.getByText("Environment Variables"),
      ).toBeInTheDocument();
    });

    it("renders column headers", () => {
      render(<EnvVarEditor />);
      expect(screen.getByText("Name")).toBeInTheDocument();
      expect(screen.getByText("Type")).toBeInTheDocument();
      expect(screen.getByText("Value")).toBeInTheDocument();
    });

    it("shows empty message when no variables", () => {
      render(<EnvVarEditor />);
      expect(
        screen.getByText("No environment variables defined"),
      ).toBeInTheDocument();
    });

    it("renders rows for existing variables", () => {
      useEnvVarStore.getState().setVars([
        { name: "API_KEY", value: "secret", type: "str" },
        { name: "PORT", value: "8080", type: "num" },
      ]);
      render(<EnvVarEditor />);
      expect(screen.getByTestId("env-var-row-API_KEY")).toBeInTheDocument();
      expect(screen.getByTestId("env-var-row-PORT")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Add variable
  // -----------------------------------------------------------------------

  describe("adding a variable", () => {
    it("adds a variable via the Add button", async () => {
      const user = userEvent.setup();
      render(<EnvVarEditor />);

      await user.type(screen.getByTestId("env-var-add-name"), "MY_VAR");
      await user.type(screen.getByTestId("env-var-add-value"), "hello");
      await user.click(screen.getByTestId("env-var-add-btn"));

      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "MY_VAR", value: "hello", type: "str" },
      ]);
      // Row should appear
      expect(screen.getByTestId("env-var-row-MY_VAR")).toBeInTheDocument();
    });

    it("adds a variable with explicit type", async () => {
      const user = userEvent.setup();
      render(<EnvVarEditor />);

      await user.type(screen.getByTestId("env-var-add-name"), "COUNT");
      await user.selectOptions(screen.getByTestId("env-var-add-type"), "num");
      await user.type(screen.getByTestId("env-var-add-value"), "42");
      await user.click(screen.getByTestId("env-var-add-btn"));

      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "COUNT", value: "42", type: "num" },
      ]);
    });

    it("adds a variable on Enter key in name field", async () => {
      const user = userEvent.setup();
      render(<EnvVarEditor />);

      await user.type(screen.getByTestId("env-var-add-name"), "FLAG{Enter}");

      expect(useEnvVarStore.getState().vars).toHaveLength(1);
      expect(useEnvVarStore.getState().vars[0].name).toBe("FLAG");
    });

    it("Add button is disabled when name is empty", () => {
      render(<EnvVarEditor />);
      expect(screen.getByTestId("env-var-add-btn")).toBeDisabled();
    });

    it("shows duplicate name error", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().addVar("EXISTING", "val");
      render(<EnvVarEditor />);

      await user.type(screen.getByTestId("env-var-add-name"), "EXISTING");
      await user.click(screen.getByTestId("env-var-add-btn"));

      expect(screen.getByTestId("env-var-dup-error")).toHaveTextContent(
        '"EXISTING" already exists',
      );
      expect(useEnvVarStore.getState().vars).toHaveLength(1);
    });

    it("clears inputs after successful add", async () => {
      const user = userEvent.setup();
      render(<EnvVarEditor />);

      await user.type(screen.getByTestId("env-var-add-name"), "A");
      await user.type(screen.getByTestId("env-var-add-value"), "v");
      await user.click(screen.getByTestId("env-var-add-btn"));

      expect(screen.getByTestId("env-var-add-name")).toHaveValue("");
      expect(screen.getByTestId("env-var-add-value")).toHaveValue("");
    });
  });

  // -----------------------------------------------------------------------
  // Edit variable
  // -----------------------------------------------------------------------

  describe("editing a variable", () => {
    it("updates value via inline input", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().setVars([
        { name: "HOST", value: "old", type: "str" },
      ]);
      render(<EnvVarEditor />);

      const valueInput = screen.getByTestId("env-var-value-HOST");
      await user.clear(valueInput);
      await user.type(valueInput, "new");

      expect(useEnvVarStore.getState().vars[0].value).toBe("new");
    });

    it("updates type via dropdown", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().setVars([
        { name: "PORT", value: "8080", type: "str" },
      ]);
      render(<EnvVarEditor />);

      await user.selectOptions(screen.getByTestId("env-var-type-PORT"), "num");
      expect(useEnvVarStore.getState().vars[0].type).toBe("num");
    });

    it("renames a variable via double-click", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().setVars([
        { name: "OLD", value: "val", type: "str" },
      ]);
      render(<EnvVarEditor />);

      // Double-click name to enter edit mode
      await user.dblClick(screen.getByTestId("env-var-name-OLD"));

      const nameInput = screen.getByTestId("env-var-name-input-OLD");
      expect(nameInput).toBeInTheDocument();

      await user.clear(nameInput);
      await user.type(nameInput, "NEW");
      // Press Enter to commit
      await user.keyboard("{Enter}");

      // OLD should be gone, NEW should exist
      expect(useEnvVarStore.getState().vars).toHaveLength(1);
      expect(useEnvVarStore.getState().vars[0].name).toBe("NEW");
    });
  });

  // -----------------------------------------------------------------------
  // Delete variable
  // -----------------------------------------------------------------------

  describe("deleting a variable", () => {
    it("removes a variable via delete button", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().setVars([
        { name: "A", value: "1", type: "str" },
        { name: "B", value: "2", type: "num" },
      ]);
      render(<EnvVarEditor />);

      await user.click(screen.getByTestId("env-var-delete-A"));

      expect(useEnvVarStore.getState().vars).toEqual([
        { name: "B", value: "2", type: "num" },
      ]);
    });

    it("shows empty message after deleting last variable", async () => {
      const user = userEvent.setup();
      useEnvVarStore.getState().setVars([
        { name: "SOLO", value: "1", type: "str" },
      ]);
      render(<EnvVarEditor />);

      await user.click(screen.getByTestId("env-var-delete-SOLO"));

      expect(
        screen.getByText("No environment variables defined"),
      ).toBeInTheDocument();
    });
  });
});
