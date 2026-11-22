/** Workspace colors. Mid-tone shades that read well in light and dark mode. */
export const WORKSPACE_COLORS = [
  { name: "Purple", hex: "#7F77DD" },
  { name: "Teal", hex: "#1D9E75" },
  { name: "Coral", hex: "#D85A30" },
  { name: "Blue", hex: "#378ADD" },
  { name: "Green", hex: "#639922" },
  { name: "Amber", hex: "#BA7517" },
  { name: "Pink", hex: "#D4537E" },
  { name: "Gray", hex: "#888780" },
] as const;

/** First palette color not already used, so new workspaces are easy to tell apart. */
export function nextColor(used: string[]): string {
  const taken = new Set(used.map((c) => c.toUpperCase()));
  return (WORKSPACE_COLORS.find((c) => !taken.has(c.hex)) ?? WORKSPACE_COLORS[0]).hex;
}
