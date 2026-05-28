import { useEffect, useCallback, useState } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { AppShell } from "./components/layout/AppShell";
import { Notifications } from "./components/notifications/Notifications";
import { flowsApi } from "./api/flows";
import { useFlowStore } from "./store/flow-store";
import { useEditorStore } from "./store/editor-store";
import { DashboardPage } from "./dashboard/DashboardPage";
import { useBreakpoint } from "./hooks";

type View = "flows" | "dashboard";

function AppInner() {
  const { setNodes, setEdges, setRevision } = useFlowStore();
  const { addNotification } = useEditorStore();
  const [view, setView] = useState<View>("flows");
  const { isMobile } = useBreakpoint();

  useEffect(() => {
    flowsApi
      .getFlows()
      .then((data) => {
        setNodes(data.nodes);
        setEdges(data.edges);
        setRevision(data.rev);
      })
      .catch((err: unknown) => {
        const message =
          err instanceof Error ? err.message : "Failed to load flows";
        addNotification({ type: "error", message });
      });
  }, [setNodes, setEdges, setRevision, addNotification]);

  const handleDeploy = useCallback(async () => {
    const { nodes, edges, revision } = useFlowStore.getState();
    try {
      await flowsApi.postFlows({ nodes, edges, rev: revision });
      addNotification({ type: "success", message: "Flows deployed" });
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "Deploy failed";
      addNotification({ type: "error", message });
    }
  }, [addNotification]);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      {/* Desktop view tabs (flows / dashboard) */}
      {!isMobile && (
        <div
          className="flex items-center border-b border-gray-200 dark:border-gray-700 bg-[#333] dark:bg-[#2a2a2a]"
          style={{ height: 30 }}
        >
          <div className="flex items-center gap-0 px-1">
            <button
              type="button"
              onClick={() => setView("flows")}
              className={`flex items-center gap-1.5 px-3 py-1 text-[11px] font-medium transition-colors ${
                view === "flows"
                  ? "text-gray-100 border-b-2 border-gray-100"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              Flows
            </button>
            <button
              type="button"
              onClick={() => setView("dashboard")}
              className={`flex items-center gap-1.5 px-3 py-1 text-[11px] font-medium transition-colors ${
                view === "dashboard"
                  ? "text-gray-100 border-b-2 border-gray-100"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              Dashboard
            </button>
          </div>
        </div>
      )}

      {/* View content */}
      <div className="flex-1 overflow-hidden">
        {isMobile ? (
          <AppShell onDeploy={handleDeploy} />
        ) : (
          view === "flows" ? <AppShell onDeploy={handleDeploy} /> : <DashboardPage />
        )}
      </div>

      {/* Notifications overlay (top-right) */}
      <Notifications />
    </div>
  );
}

export default function App() {
  return (
    <ReactFlowProvider>
      <AppInner />
    </ReactFlowProvider>
  );
}
