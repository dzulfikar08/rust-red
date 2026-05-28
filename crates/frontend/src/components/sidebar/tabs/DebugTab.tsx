import { DebugPanel } from "../../debug/DebugPanel";

export function DebugTab() {
  return (
    <div className="flex-1 flex flex-col overflow-hidden" data-testid="sidebar-tab-content-debug">
      <DebugPanel />
    </div>
  );
}
