import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DeployButton } from "../Deploy";
import { useDeployStore } from "../../../store/deploy-store";

// ---------------------------------------------------------------------------
// Mock the API so deploy() doesn't make real HTTP calls
// ---------------------------------------------------------------------------

vi.mock("../../../api/flows", () => ({
  flowsApi: {
    postFlows: vi.fn().mockResolvedValue({ rev: "rev-1", nodes: [], edges: [] }),
    getFlows: vi.fn(),
  },
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useDeployStore.setState({
    status: "idle",
    lastDeployTime: null,
    hasUnsavedChanges: false,
    errorMessage: null,
  });
}

describe("DeployButton", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the deploy button", () => {
      render(<DeployButton />);
      expect(screen.getByText("Deploy")).toBeInTheDocument();
    });

    it("renders as a button element", () => {
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      expect(btn).toBeInTheDocument();
      expect(btn.tagName).toBe("BUTTON");
    });

    it("has green background when no unsaved changes", () => {
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      expect(btn.style.backgroundColor).toBe("rgb(136, 170, 170)"); // #8aa
    });

    it("has red background when there are unsaved changes", () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      expect(btn.style.backgroundColor).toBe("rgb(187, 85, 85)"); // #b55
    });

    it("shows a dot indicator when there are unsaved changes", () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      render(<DeployButton />);
      const dot = screen.getByLabelText("Unsaved changes");
      expect(dot).toBeInTheDocument();
    });

    it("does not show a dot indicator when no unsaved changes", () => {
      render(<DeployButton />);
      expect(screen.queryByLabelText("Unsaved changes")).not.toBeInTheDocument();
    });

    it("shows a play icon when idle", () => {
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      // The polygon (play icon) should be rendered
      const polygon = btn.querySelector("polygon");
      expect(polygon).toBeInTheDocument();
    });

    it("shows a spinning icon when deploying", () => {
      useDeployStore.setState({ status: "deploying" });
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      const spinSvg = btn.querySelector(".animate-spin");
      expect(spinSvg).toBeInTheDocument();
    });

    it("does not show play icon when deploying", () => {
      useDeployStore.setState({ status: "deploying" });
      render(<DeployButton />);
      const btn = screen.getByRole("button");
      const polygon = btn.querySelector("polygon");
      expect(polygon).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Disabled state
  // -----------------------------------------------------------------------

  describe("disabled state", () => {
    it("is enabled when idle", () => {
      render(<DeployButton />);
      expect(screen.getByRole("button")).toBeEnabled();
    });

    it("is disabled when deploying", () => {
      useDeployStore.setState({ status: "deploying" });
      render(<DeployButton />);
      expect(screen.getByRole("button")).toBeDisabled();
    });

    it("is enabled when in error state", () => {
      useDeployStore.setState({ status: "error" });
      render(<DeployButton />);
      expect(screen.getByRole("button")).toBeEnabled();
    });

    it("is enabled when in success state", () => {
      useDeployStore.setState({ status: "success" });
      render(<DeployButton />);
      expect(screen.getByRole("button")).toBeEnabled();
    });
  });

  // -----------------------------------------------------------------------
  // Click behavior
  // -----------------------------------------------------------------------

  describe("click behavior", () => {
    it("triggers deploy on click", async () => {
      const { flowsApi } = await import("../../../api/flows");
      render(<DeployButton />);

      const btn = screen.getByRole("button");
      fireEvent.click(btn);

      // The deploy function was called (flowsApi.postFlows is invoked inside)
      expect(flowsApi.postFlows).toHaveBeenCalled();
    });

    it("does not trigger deploy when disabled", async () => {
      useDeployStore.setState({ status: "deploying" });
      const { flowsApi } = await import("../../../api/flows");

      render(<DeployButton />);
      const btn = screen.getByRole("button");
      fireEvent.click(btn);

      // postFlows should NOT have been called again (it's already deploying)
      expect(flowsApi.postFlows).not.toHaveBeenCalled();
    });
  });

  // -----------------------------------------------------------------------
  // Title / tooltip
  // -----------------------------------------------------------------------

  describe("title attribute", () => {
    it('shows "Deploy" tooltip when idle with no changes', () => {
      render(<DeployButton />);
      expect(screen.getByRole("button").title).toBe("Deploy");
    });

    it('shows "Deploy (unsaved changes)" tooltip when changes exist', () => {
      useDeployStore.getState().setHasUnsavedChanges(true);
      render(<DeployButton />);
      expect(screen.getByRole("button").title).toBe("Deploy (unsaved changes)");
    });

    it('shows "Deploying..." tooltip when deploying', () => {
      useDeployStore.setState({ status: "deploying" });
      render(<DeployButton />);
      expect(screen.getByRole("button").title).toBe("Deploying...");
    });
  });
});
