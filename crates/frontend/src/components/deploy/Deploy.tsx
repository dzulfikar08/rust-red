/**
 * Deploy Button
 *
 * Matches Node-RED's deploy button appearance:
 * - Green (#8aa) background when no unsaved changes
 * - Red (#b55) background with dot indicator when unsaved changes
 * - Spinning icon during deployment
 * - Disabled during deployment
 */
import { useDeployStore } from "../../store/deploy-store";

export function DeployButton() {
  const status = useDeployStore((s) => s.status);
  const hasUnsavedChanges = useDeployStore((s) => s.hasUnsavedChanges);
  const deploy = useDeployStore((s) => s.deploy);

  const isDeploying = status === "deploying";

  // Node-RED color scheme: green when idle, red when unsaved changes
  const bgColor = hasUnsavedChanges ? "#b55" : "#8aa";
  const hoverBgColor = hasUnsavedChanges ? "#c66" : "#9bb";

  return (
    <button
      type="button"
      onClick={() => deploy()}
      disabled={isDeploying}
      className="flex items-center gap-1.5 px-3 py-1 text-sm font-medium text-white rounded transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
      style={{ backgroundColor: bgColor }}
      onMouseEnter={(e) => {
        if (!isDeploying) {
          e.currentTarget.style.backgroundColor = hoverBgColor;
        }
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = bgColor;
      }}
      title={
        isDeploying
          ? "Deploying..."
          : hasUnsavedChanges
            ? "Deploy (unsaved changes)"
            : "Deploy"
      }
    >
      {isDeploying ? (
        <svg
          className="animate-spin"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M21 12a9 9 0 1 1-6.219-8.56" />
        </svg>
      ) : (
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <polygon points="5,3 19,12 5,21" />
        </svg>
      )}
      Deploy
      {hasUnsavedChanges && !isDeploying && (
        <span
          className="inline-block w-1.5 h-1.5 rounded-full bg-white ml-0.5"
          aria-label="Unsaved changes"
        />
      )}
    </button>
  );
}

export default DeployButton;
