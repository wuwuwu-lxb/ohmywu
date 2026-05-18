export type ThemePreset = "midnight" | "slate" | "amber"
export type BackgroundPreset =
  | "noctis"
  | "tidal"
  | "ember"
  | "aurora"
  | "graphite"

export interface Theme {
  preset: ThemePreset
  accent: string
}

export const THEME_PRESETS: Record<ThemePreset, { label: string; accent: string }> = {
  midnight: { label: "Midnight", accent: "#3b82f6" },
  slate: { label: "Slate", accent: "#8b9bff" },
  amber: { label: "Amber", accent: "#ff9e57" },
}

export interface BackgroundPresetDef {
  label: string
  css: string
  seed: string
}

export const BACKGROUND_PRESETS: Record<BackgroundPreset, BackgroundPresetDef> = {
  noctis: {
    label: "Noctis",
    css: "#111827",
    seed: "#60a5fa",
  },
  tidal: {
    label: "Tidal",
    css: "#10232a",
    seed: "#4dd0e1",
  },
  ember: {
    label: "Ember",
    css: "#26171a",
    seed: "#ff8a65",
  },
  aurora: {
    label: "Aurora",
    css: "#18211d",
    seed: "#81c784",
  },
  graphite: {
    label: "Graphite",
    css: "#20242c",
    seed: "#9aa5b1",
  },
}
