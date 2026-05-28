/**
 * FlowCanvas - Main canvas component for the Node-RED flow editor
 *
 * Behaviours:
 *  - Snap-to-grid: 20px grid matching Node-RED
 *  - Zoom range: 10% to 200% with custom ZoomControls
 *  - Minimap: React Flow MiniMap in bottom-right
 *  - Selection: click, ctrl+click, lasso drag, Delete key to remove
 *  - Double-click: emits 'node:edit-requested' via eventBus
 *  - Right-click: emits 'canvas:context-menu' via eventBus
 *  - Drop handling: create node from palette drop using nodeRegistry
 *  - Custom node types: NodeComponent (nrNode) + GenericNode (generic)
 *  - Custom edge type: WireComponent (wire)
 */

import { useCallback, useRef, useEffect, useState } from "react";
import {
  ReactFlow,
  Background,
  MiniMap,
  addEdge,
  useReactFlow,
  type Connection,
  type Edge,
  type Node,
  type ReactFlowInstance,
  useStoreApi,
} from "@xyflow/react";
import { useFlowStore } from "../../store/flow-store";
import { useEditorStore } from "../../store/editor-store";
import { useBreakpoint, useTouchGestures } from "../../hooks";
import { GenericNode } from "./GenericNode";
import { NodeComponent } from "../canvas/NodeComponent";
import { WireComponent } from "../canvas/WireComponent";
import { ZoomControls } from "../canvas/ZoomControls";
import { eventBus } from "../../red/core/events";
import { nodeRegistry } from "../../red/nodes/registry";

// ---------------------------------------------------------------------------
// Custom node & edge type maps
// ---------------------------------------------------------------------------

const nodeTypes = {
  generic: GenericNode,
  nrNode: NodeComponent,
};

const edgeTypes = {
  wire: WireComponent,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SNAP_GRID_SIZE = 20; // matching Node-RED
const MIN_ZOOM = 0.1; // 10%
const MAX_ZOOM = 2.0; // 200%

const VIEWPORT_KEY = "rustred-viewport";

// ---------------------------------------------------------------------------
// Viewport persistence helpers
// ---------------------------------------------------------------------------

function loadViewport() {
  try {
    const raw = localStorage.getItem(VIEWPORT_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    /* ignore */
  }
  return null;
}

function saveViewport(vp: { x: number; y: number; zoom: number }) {
  try {
    localStorage.setItem(VIEWPORT_KEY, JSON.stringify(vp));
  } catch {
    /* ignore */
  }
}

// ---------------------------------------------------------------------------
// Inner component (must be inside ReactFlowProvider to use useReactFlow)
// ---------------------------------------------------------------------------

function FlowCanvasInner() {
  const { nodes, edges, onNodesChange, onEdgesChange, addNode } =
    useFlowStore();
  const reactFlowInstance = useRef<ReactFlowInstance | null>(null);
  const { isMobile } = useBreakpoint();
  const store = useStoreApi();
  const reactFlowHook = useReactFlow();
  const [zoomLevel, setZoomLevel] = useState(1);

  // -----------------------------------------------------------------------
  // Connection handling
  // -----------------------------------------------------------------------

  const onConnect = useCallback(
    (connection: Connection) => {
      useFlowStore.setState((state) => ({
        edges: addEdge(connection, state.edges) as Edge[],
      }));
    },
    [],
  );

  // -----------------------------------------------------------------------
  // Initialization
  // -----------------------------------------------------------------------

  const onInit = useCallback((instance: ReactFlowInstance) => {
    reactFlowInstance.current = instance;
    const saved = loadViewport();
    if (saved) {
      instance.setViewport(saved);
    }
  }, []);

  // -----------------------------------------------------------------------
  // Zoom tracking
  // -----------------------------------------------------------------------

  const onMoveEnd = useCallback(() => {
    if (reactFlowInstance.current) {
      const vp = reactFlowInstance.current.getViewport();
      saveViewport(vp);
      setZoomLevel(vp.zoom);
    }
  }, []);

  // Sync zoom level on first render and when viewport changes via store
  useEffect(() => {
    const unsubscribe = store.subscribe((state) => {
      if (state.transform) {
        // transform[2] is zoom
        setZoomLevel(state.transform[2]);
      }
    });
    return unsubscribe;
  }, [store]);

  // -----------------------------------------------------------------------
  // Zoom control handlers
  // -----------------------------------------------------------------------

  const handleZoomIn = useCallback(() => {
    reactFlowHook.zoomIn({ duration: 200 });
  }, [reactFlowHook]);

  const handleZoomOut = useCallback(() => {
    reactFlowHook.zoomOut({ duration: 200 });
  }, [reactFlowHook]);

  const handleZoomToFit = useCallback(() => {
    reactFlowHook.fitView({ duration: 300, padding: 0.15 });
  }, [reactFlowHook]);

  // -----------------------------------------------------------------------
  // Drag & Drop from palette
  // -----------------------------------------------------------------------

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const raw = event.dataTransfer.getData("application/reactflow");
      if (!raw) return;

      const paletteNode = JSON.parse(raw) as {
        type: string;
        label: string;
        color: string;
      };

      if (!reactFlowInstance.current) return;

      const position = reactFlowInstance.current.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      // Snap the drop position to the grid
      position.x = Math.round(position.x / SNAP_GRID_SIZE) * SNAP_GRID_SIZE;
      position.y = Math.round(position.y / SNAP_GRID_SIZE) * SNAP_GRID_SIZE;

      // Look up the registry for port counts
      const def = nodeRegistry.getType(paletteNode.type);
      const inputs = def?.inputs ?? 1;
      const outputs = def?.outputs ?? 1;

      // Use nrNode type (NodeComponent) when registry has the type,
      // otherwise fall back to generic
      const nodeType = def ? "nrNode" : "generic";

      const newNode: Node = {
        id: `${paletteNode.type}-${Date.now()}`,
        type: nodeType,
        position,
        data: def
          ? {
              type: paletteNode.type,
              label: paletteNode.label,
              color: paletteNode.color,
              inputs,
              outputs,
              icon: def.icon,
            }
          : {
              label: paletteNode.label,
              nodeType: paletteNode.type,
              color: paletteNode.color,
            },
      };

      addNode(newNode);
      eventBus.emit("nodes:added", {
        id: newNode.id,
        type: paletteNode.type,
      });
    },
    [addNode],
  );

  // -----------------------------------------------------------------------
  // Node double-click → emit edit event
  // -----------------------------------------------------------------------

  const onNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      useEditorStore.getState().selectNode(node.id);
      eventBus.emit("node:edit-requested", { id: node.id });
    },
    [],
  );

  // -----------------------------------------------------------------------
  // Right-click on canvas → context menu event
  // -----------------------------------------------------------------------

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();

      let flowX = 0;
      let flowY = 0;
      if (reactFlowInstance.current) {
        const pos = reactFlowInstance.current.screenToFlowPosition({
          x: event.clientX,
          y: event.clientY,
        });
        flowX = pos.x;
        flowY = pos.y;
      }

      eventBus.emit("canvas:context-menu", {
        x: event.clientX,
        y: event.clientY,
        flowX,
        flowY,
      });
    },
    [],
  );

  // -----------------------------------------------------------------------
  // Delete selected nodes/edges on Delete key
  // -----------------------------------------------------------------------

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Delete" || e.key === "Backspace") {
        // Don't delete if user is typing in an input
        const target = e.target as HTMLElement;
        if (
          target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable
        ) {
          return;
        }

        const { nodes: currentNodes, edges: currentEdges } =
          useFlowStore.getState();

        const selectedNodes = currentNodes.filter((n) => n.selected);
        const selectedEdges = currentEdges.filter((e) => e.selected);

        if (selectedNodes.length > 0 || selectedEdges.length > 0) {
          const selectedNodeIds = new Set(selectedNodes.map((n) => n.id));

          // Remove selected edges first
          if (selectedEdges.length > 0) {
            const remainingEdges = currentEdges.filter(
              (e) => !e.selected && !selectedNodeIds.has(e.source) && !selectedNodeIds.has(e.target),
            );
            useFlowStore.setState({ edges: remainingEdges });
          } else {
            // Remove edges connected to deleted nodes
            const remainingEdges = currentEdges.filter(
              (e) => !selectedNodeIds.has(e.source) && !selectedNodeIds.has(e.target),
            );
            useFlowStore.setState({ edges: remainingEdges });
          }

          // Remove selected nodes
          const remainingNodes = currentNodes.filter((n) => !n.selected);
          useFlowStore.setState({ nodes: remainingNodes });

          // Emit events
          for (const node of selectedNodes) {
            eventBus.emit("nodes:removed", { id: node.id });
          }
        }
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  // -----------------------------------------------------------------------
  // Touch gesture handlers (mobile)
  // -----------------------------------------------------------------------

  const handleDoubleTap = useCallback(
    (position: { x: number; y: number }) => {
      if (!reactFlowInstance.current) return;
      const flowPos = reactFlowInstance.current.screenToFlowPosition(position);
      const clicked = nodes.find((n) => {
        const nodeW = 140;
        const nodeH = 50;
        return (
          flowPos.x >= n.position.x &&
          flowPos.x <= n.position.x + nodeW &&
          flowPos.y >= n.position.y &&
          flowPos.y <= n.position.y + nodeH
        );
      });
      if (clicked) {
        useEditorStore.getState().selectNode(clicked.id);
        eventBus.emit("node:edit-requested", { id: clicked.id });
      }
    },
    [nodes],
  );

  const touchGestures = useTouchGestures({
    rfInstance: reactFlowInstance.current,
    onDoubleTap: handleDoubleTap,
  });

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  return (
    <div className="flex-1 relative" {...(isMobile ? touchGestures : {})}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onInit={onInit}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onMoveEnd={onMoveEnd}
        onNodeDoubleClick={onNodeDoubleClick}
        onPaneContextMenu={onPaneContextMenu}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={{ type: "wire" }}
        fitView={!loadViewport()}
        className="bg-gray-100 dark:bg-gray-900"
        minZoom={MIN_ZOOM}
        maxZoom={MAX_ZOOM}
        snapToGrid={true}
        snapGrid={[SNAP_GRID_SIZE, SNAP_GRID_SIZE]}
        zoomOnScroll={!isMobile}
        zoomOnPinch={!isMobile}
        panOnScroll={!isMobile}
        panOnDrag={isMobile ? [1, 2] : true}
        selectionOnDrag={!isMobile}
        selectNodesOnDrag={!isMobile}
        nodesDraggable={true}
        connectOnClick={true}
        deleteKeyCode={null}
        data-testid="flow-canvas"
      >
        <Background
          gap={SNAP_GRID_SIZE}
          size={1}
          data-testid="canvas-background"
        />
        <MiniMap
          className="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700"
          maskColor="rgba(0, 0, 0, 0.1)"
          pannable={true}
          zoomable={true}
          data-testid="canvas-minimap"
        />
      </ReactFlow>

      {/* Custom zoom controls (bottom-left) */}
      <ZoomControls
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onZoomToFit={handleZoomToFit}
        zoomLevel={zoomLevel}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Exported component (wraps with error boundary safe to use outside provider)
// ---------------------------------------------------------------------------

export function FlowCanvas() {
  return <FlowCanvasInner />;
}
