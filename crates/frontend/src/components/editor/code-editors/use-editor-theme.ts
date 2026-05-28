/**
 * Hook that syncs Monaco Editor's theme with the application theme store.
 *
 * Returns `"vs-dark"` when the app is in dark mode, `"light"` otherwise.
 * This can be passed directly as the `theme` prop to the Monaco Editor.
 */

import { useMemo } from "react";
import { useThemeStore } from "@/store/theme-store";

export type EditorTheme = "vs-dark" | "light";

export function useEditorTheme(): EditorTheme {
  const appTheme = useThemeStore((s) => s.theme);

  return useMemo(() => (appTheme === "dark" ? "vs-dark" : "light"), [appTheme]);
}
