export type ThemePreset = "midnight" | "slate" | "amber"

export interface Theme {
  preset: ThemePreset
  accent: string
}

export const THEME_PRESETS: Record<ThemePreset, { label: string; accent: string }> = {
  midnight: { label: "Midnight", accent: "#3b82f6" },
  slate: { label: "Slate", accent: "#a78bfa" },
  amber: { label: "Amber", accent: "#f59e0b" },
}
