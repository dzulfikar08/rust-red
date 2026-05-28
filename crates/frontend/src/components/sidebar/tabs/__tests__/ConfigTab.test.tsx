import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ConfigTab } from "../ConfigTab";
import { useFlowStore } from "../../../../store/flow-store";
import { useEditorStore } from "../../../../store/editor-store";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStores() {
  useFlowStore.setState({ nodes: [], edges: [], revision: "" });
  useEditorStore.setState({ selectedNodeId: null });
}

/** Create a ReactFlow-style config node in the flow store. */
function addConfigNode(
  id: string,
  type: string,
  label: string,
  users: string[] = [],
) {
  const nodes = useFlowStore.getState().nodes;
  useFlowStore.setState({
    nodes: [
      ...nodes,
      {
        id,
        type: "nrNode",
        position: { x: 0, y: 0 },
        data: {
          _isConfig: true,
          _users: users,
          type,
          label,
        },
      },
    ],
  });
}

/** Create a regular (non-config) node in the flow store. */
function addRegularNode(id: string, type: string, label: string) {
  const nodes = useFlowStore.getState().nodes;
  useFlowStore.setState({
    nodes: [
      ...nodes,
      {
        id,
        type: "nrNode",
        position: { x: 100, y: 100 },
        data: {
          type,
          label,
        },
      },
    ],
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ConfigTab", () => {
  beforeEach(() => {
    resetStores();
  });

  // -----------------------------------------------------------------------
  // Rendering
  // -----------------------------------------------------------------------

  describe("rendering", () => {
    it("renders the config tab container", () => {
      render(<ConfigTab />);
      expect(screen.getByTestId("sidebar-tab-content-config")).toBeInTheDocument();
    });

    it("renders the search input", () => {
      render(<ConfigTab />);
      expect(screen.getByTestId("config-search-input")).toBeInTheDocument();
    });

    it("shows empty state when no config nodes exist", () => {
      render(<ConfigTab />);
      expect(screen.getByTestId("config-empty-state")).toBeInTheDocument();
      expect(screen.getByTestId("config-empty-state")).toHaveTextContent(
        "No configuration nodes",
      );
    });

    it("does not show empty state when config nodes exist", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      render(<ConfigTab />);
      expect(screen.queryByTestId("config-empty-state")).not.toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Grouped config nodes
  // -----------------------------------------------------------------------

  describe("grouped config nodes", () => {
    it("groups config nodes by type", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      addConfigNode("cfg2", "http-request", "My HTTP", ["n2"]);
      addConfigNode("cfg3", "mqtt-broker", "Remote MQTT", ["n3"]);

      render(<ConfigTab />);

      const groups = screen.getAllByTestId("config-group");
      // 2 used groups + 0 unused groups
      expect(groups).toHaveLength(2);
    });

    it("renders group headers with type name and count", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      addConfigNode("cfg2", "mqtt-broker", "Remote MQTT", ["n3"]);

      render(<ConfigTab />);

      const headers = screen.getAllByTestId("config-group-header");
      // Find the mqtt-broker header
      const mqttHeader = headers.find((h) =>
        h.textContent?.includes("mqtt-broker"),
      );
      expect(mqttHeader).toBeTruthy();
      expect(mqttHeader!.textContent).toContain("(2)");
    });

    it("ignores non-config nodes", () => {
      addRegularNode("n1", "inject", "Inject");
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      const groups = screen.getAllByTestId("config-group");
      expect(groups).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // User counts
  // -----------------------------------------------------------------------

  describe("user counts", () => {
    it("shows user count for each config node", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1", "n2"]);

      render(<ConfigTab />);

      // Expand the group by clicking the header
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      const userCounts = screen.getAllByTestId("config-node-user-count");
      expect(userCounts[0]).toHaveTextContent("2 users");
    });

    it("shows singular 'user' for a single user", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      const userCounts = screen.getAllByTestId("config-node-user-count");
      expect(userCounts[0]).toHaveTextContent("1 user");
    });
  });

  // -----------------------------------------------------------------------
  // Unused section
  // -----------------------------------------------------------------------

  describe("unused section", () => {
    it("creates an unused group for config nodes with 0 users", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      addConfigNode("cfg2", "http-request", "Old HTTP", []);

      render(<ConfigTab />);

      const groups = screen.getAllByTestId("config-group");
      // One used group (mqtt-broker) + one unused group
      expect(groups).toHaveLength(2);

      const unusedGroup = groups.find(
        (g) => g.dataset.groupType === "(unused)",
      );
      expect(unusedGroup).toBeTruthy();
    });

    it("shows 'unused' text for nodes in the unused group", () => {
      addConfigNode("cfg2", "http-request", "Old HTTP", []);

      render(<ConfigTab />);

      // Auto-expanded since it's the only group, or click to expand
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      const userCounts = screen.getAllByTestId("config-node-user-count");
      expect(userCounts[0]).toHaveTextContent("unused");
    });

    it("does not create unused group when all configs have users", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      const groups = screen.getAllByTestId("config-group");
      const unusedGroup = groups.find(
        (g) => g.dataset.groupType === "(unused)",
      );
      expect(unusedGroup).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Search filter
  // -----------------------------------------------------------------------

  describe("search filter", () => {
    it("filters config nodes by type", async () => {
      const user = userEvent.setup();
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      addConfigNode("cfg2", "http-request", "My HTTP", ["n2"]);

      render(<ConfigTab />);

      const input = screen.getByTestId("config-search-input");
      await user.type(input, "mqtt");

      const groups = screen.getAllByTestId("config-group");
      expect(groups).toHaveLength(1);
      expect(groups[0].dataset.groupType).toBe("mqtt-broker");
    });

    it("filters config nodes by label", async () => {
      const user = userEvent.setup();
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);
      addConfigNode("cfg2", "mqtt-broker", "Remote MQTT", ["n2"]);

      render(<ConfigTab />);

      const input = screen.getByTestId("config-search-input");
      await user.type(input, "Remote");

      // Group still exists but with only one matching node
      const items = screen.getAllByTestId("config-node-item");
      expect(items).toHaveLength(1);
      expect(screen.getByTestId("config-node-label")).toHaveTextContent(
        "Remote MQTT",
      );
    });

    it("shows empty state when search has no matches", async () => {
      const user = userEvent.setup();
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      const input = screen.getByTestId("config-search-input");
      await user.type(input, "nonexistent");

      expect(screen.getByTestId("config-empty-state")).toBeInTheDocument();
    });

    it("auto-expands groups during search", async () => {
      const user = userEvent.setup();
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      // Type to trigger auto-expand
      const input = screen.getByTestId("config-search-input");
      await user.type(input, "mqtt");

      // The group list should be visible (auto-expanded)
      expect(screen.getByTestId("config-group-list")).toBeInTheDocument();
    });
  });

  // -----------------------------------------------------------------------
  // Delete button for unused configs
  // -----------------------------------------------------------------------

  describe("delete button for unused configs", () => {
    it("shows delete button for unused config nodes", () => {
      addConfigNode("cfg1", "http-request", "Old HTTP", []);

      render(<ConfigTab />);

      // Expand the group
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      expect(screen.getByTestId("config-node-delete")).toBeInTheDocument();
    });

    it("does not show delete button for used config nodes", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      // Expand the group
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      expect(screen.queryByTestId("config-node-delete")).not.toBeInTheDocument();
    });

    it("removes the node when delete button is clicked", () => {
      addConfigNode("cfg1", "http-request", "Old HTTP", []);

      render(<ConfigTab />);

      // Expand the group
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      const deleteBtn = screen.getByTestId("config-node-delete");
      fireEvent.click(deleteBtn);

      // Node should be removed from the store
      expect(
        useFlowStore.getState().nodes.find((n) => n.id === "cfg1"),
      ).toBeUndefined();
    });
  });

  // -----------------------------------------------------------------------
  // Click to select
  // -----------------------------------------------------------------------

  describe("click to select", () => {
    it("selects a config node when clicked", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      // Expand the group
      const header = screen.getByTestId("config-group-header");
      fireEvent.click(header);

      const nodeItem = screen.getByTestId("config-node-item");
      fireEvent.click(nodeItem);

      expect(useEditorStore.getState().selectedNodeId).toBe("cfg1");
    });
  });

  // -----------------------------------------------------------------------
  // Collapse / expand
  // -----------------------------------------------------------------------

  describe("collapse / expand", () => {
    it("collapses a group when header is clicked twice", () => {
      addConfigNode("cfg1", "mqtt-broker", "Local MQTT", ["n1"]);

      render(<ConfigTab />);

      const header = screen.getByTestId("config-group-header");

      // Expand
      fireEvent.click(header);
      expect(screen.getByTestId("config-group-list")).toBeInTheDocument();

      // Collapse
      fireEvent.click(header);
      expect(screen.queryByTestId("config-group-list")).not.toBeInTheDocument();
    });
  });
});
