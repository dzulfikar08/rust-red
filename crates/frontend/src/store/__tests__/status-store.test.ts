import { describe, it, expect, beforeEach } from "vitest";
import { useStatusStore } from "../status-store";

function resetStatusStore() {
  useStatusStore.setState({ connectionStatus: "disconnected" });
}

describe("useStatusStore", () => {
  beforeEach(() => {
    resetStatusStore();
  });

  // -----------------------------------------------------------------------
  // Default state
  // -----------------------------------------------------------------------

  describe("default state", () => {
    it("starts disconnected", () => {
      expect(useStatusStore.getState().connectionStatus).toBe("disconnected");
    });
  });

  // -----------------------------------------------------------------------
  // setConnectionStatus
  // -----------------------------------------------------------------------

  describe("setConnectionStatus()", () => {
    it("sets status to connected", () => {
      useStatusStore.getState().setConnectionStatus("connected");
      expect(useStatusStore.getState().connectionStatus).toBe("connected");
    });

    it("sets status to connecting", () => {
      useStatusStore.getState().setConnectionStatus("connecting");
      expect(useStatusStore.getState().connectionStatus).toBe("connecting");
    });

    it("sets status to disconnected", () => {
      useStatusStore.getState().setConnectionStatus("connected");
      useStatusStore.getState().setConnectionStatus("disconnected");
      expect(useStatusStore.getState().connectionStatus).toBe("disconnected");
    });
  });
});
