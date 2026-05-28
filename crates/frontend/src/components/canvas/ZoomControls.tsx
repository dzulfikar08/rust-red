/**
 * ZoomControls - Custom zoom control panel for the canvas
 *
 * Positioned in the bottom-left corner of the canvas.
 * Shows current zoom level percentage and provides +/-/fit buttons.
 * Matches Node-RED zoom range: 10% to 200%.
 */

interface ZoomControlsProps {
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomToFit: () => void;
  /** Current zoom level, e.g. 0.5 = 50%, 1.0 = 100%, 2.0 = 200% */
  zoomLevel: number;
}

export function ZoomControls({
  onZoomIn,
  onZoomOut,
  onZoomToFit,
  zoomLevel,
}: ZoomControlsProps) {
  const percent = Math.round(zoomLevel * 100);

  return (
    <div
      className="
        absolute bottom-4 left-4 z-10
        flex items-center gap-0.5
        bg-white dark:bg-gray-800
        border border-gray-200 dark:border-gray-700
        rounded shadow-sm
        overflow-hidden
      "
      data-testid="zoom-controls"
    >
      <button
        type="button"
        onClick={onZoomOut}
        className="
          flex items-center justify-center
          w-7 h-7
          text-gray-600 dark:text-gray-300
          hover:bg-gray-100 dark:hover:bg-gray-700
          transition-colors
        "
        aria-label="Zoom out"
        data-testid="zoom-out-btn"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        >
          <line x1="3" y1="7" x2="11" y2="7" />
        </svg>
      </button>

      <span
        className="
          flex items-center justify-center
          min-w-[42px] h-7 px-1
          text-[11px] font-medium tabular-nums
          text-gray-700 dark:text-gray-300
          border-x border-gray-200 dark:border-gray-700
          select-none
        "
        data-testid="zoom-level"
      >
        {percent}%
      </span>

      <button
        type="button"
        onClick={onZoomIn}
        className="
          flex items-center justify-center
          w-7 h-7
          text-gray-600 dark:text-gray-300
          hover:bg-gray-100 dark:hover:bg-gray-700
          transition-colors
        "
        aria-label="Zoom in"
        data-testid="zoom-in-btn"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        >
          <line x1="7" y1="3" x2="7" y2="11" />
          <line x1="3" y1="7" x2="11" y2="7" />
        </svg>
      </button>

      <button
        type="button"
        onClick={onZoomToFit}
        className="
          flex items-center justify-center
          w-7 h-7
          text-gray-600 dark:text-gray-300
          hover:bg-gray-100 dark:hover:bg-gray-700
          transition-colors
        "
        aria-label="Zoom to fit"
        data-testid="zoom-fit-btn"
        title="Zoom to fit"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          {/* Four-corner bracket icon representing "fit" */}
          <path d="M1 5V2.5C1 1.67 1.67 1 2.5 1H5" />
          <path d="M9 1h2.5C12.33 1 13 1.67 13 2.5V5" />
          <path d="M13 9v2.5c0 .83-.67 1.5-1.5 1.5H9" />
          <path d="M5 13H2.5C1.67 13 1 12.33 1 11.5V9" />
        </svg>
      </button>
    </div>
  );
}
