import { ref, computed } from "vue"
import type { BackgroundPreset, ThemePreset } from "../lib/theme"
import { BACKGROUND_PRESETS, THEME_PRESETS } from "../lib/theme"
import {
  extractDominantAccentFromImage,
  hexToRgb,
  mixHex,
  rgbToHsl,
  shiftLightness,
} from "../lib/color"

const preset = ref<ThemePreset>("midnight")
const accent = ref(THEME_PRESETS.midnight.accent)
const backgroundPreset = ref<BackgroundPreset>("noctis")
const backgroundMode = ref<"solid" | "image">("solid")
const backgroundImageUrl = ref("")
const backgroundAutoTheme = ref(true)
const backgroundThemeColor = ref("")
const bgScale = ref(1.0)
const bgBlur = ref(0)
const bgMaskOpacity = ref(30)
const surfaceOpacity = ref(72)
const shellBlur = ref(20)
let backgroundThemeJob = 0

const withAlpha = (hex: string, alpha: number) => {
  const [r, g, b] = hexToRgb(hex)
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

const revokeObjectUrl = (url: string) => {
  if (!url || !url.startsWith("blob:")) return
  try {
    URL.revokeObjectURL(url)
  } catch {
    // ignore
  }
}

const deriveSolidPalette = (backgroundHex: string, seedHex: string) => {
  const [br, bgValue, bb] = hexToRgb(backgroundHex)
  const { l } = rgbToHsl(br, bgValue, bb)
  const background = l > 0.22 ? shiftLightness(backgroundHex, 0.18) : backgroundHex
  const surface = mixHex(background, "#ffffff", 0.06)
  const surfaceRaised = mixHex(background, "#ffffff", 0.1)
  const surfaceHover = mixHex(background, "#ffffff", 0.14)
  const border = mixHex(background, "#ffffff", 0.16)
  const shell = mixHex(background, "#ffffff", 0.2)
  const accent = shiftLightness(seedHex, 0.66, 0.08)
  const textSecondary = mixHex("#e9edf5", background, 0.42)
  const textTertiary = mixHex("#95a1b5", background, 0.36)
  return {
    background,
    surface,
    surfaceRaised,
    surfaceHover,
    border,
    shell,
    accent,
    textSecondary,
    textTertiary,
  }
}

export function useTheme() {
  const initFromConfig = (cfg: {
    theme: string; accent: string;
    background_preset: string; background_mode: string; surface_opacity: number;
    background_scale: number; background_blur: number; background_mask_opacity: number;
    background_auto_theme?: boolean;
    background_theme_color?: string | null;
  }) => {
    preset.value = (cfg.theme as ThemePreset) || "midnight"
    accent.value = cfg.accent || THEME_PRESETS[preset.value].accent
    backgroundPreset.value = (cfg.background_preset as BackgroundPreset) || "noctis"
    backgroundMode.value = cfg.background_mode === "image" ? "image" : "solid"
    backgroundAutoTheme.value = cfg.background_auto_theme ?? true
    backgroundThemeColor.value = cfg.background_theme_color || ""
    surfaceOpacity.value = cfg.surface_opacity ?? 72
    bgScale.value = cfg.background_scale ?? 1.0
    bgBlur.value = cfg.background_blur ?? 0
    bgMaskOpacity.value = cfg.background_mask_opacity ?? 30

    if (backgroundMode.value === "image" && backgroundAutoTheme.value && backgroundThemeColor.value) {
      accent.value = backgroundThemeColor.value
    }
    applyTheme()
  }

  const setPreset = (p: ThemePreset) => {
    preset.value = p
    accent.value = THEME_PRESETS[p].accent
    backgroundAutoTheme.value = false
    applyTheme()
  }
  const setBackgroundPreset = (p: BackgroundPreset) => {
    backgroundPreset.value = p
    if (backgroundMode.value === "solid") {
      accent.value = BACKGROUND_PRESETS[p].seed
    }
    applyTheme()
  }
  const setAccent = (color: string, options?: { keepAuto?: boolean }) => {
    accent.value = color
    if (!options?.keepAuto) backgroundAutoTheme.value = false
    applyTheme()
  }
  const setSurfaceOpacity = (v: number) => { surfaceOpacity.value = Math.max(35, Math.min(88, v)); applyTheme() }
  const setBackgroundMode = (m: "solid" | "image") => {
    backgroundMode.value = m
    if (m === "solid") {
      accent.value = BACKGROUND_PRESETS[backgroundPreset.value].seed
    }
    applyTheme()
  }
  const setBgScale = (v: number) => { bgScale.value = v; applyTheme() }
  const setBgBlur = (v: number) => { bgBlur.value = v; applyTheme() }
  const setBgMaskOpacity = (v: number) => { bgMaskOpacity.value = v; applyTheme() }
  const setBackgroundAutoTheme = (enabled: boolean) => { backgroundAutoTheme.value = enabled }
  const setBackgroundThemeColor = (color: string) => { backgroundThemeColor.value = color }

  const syncBackgroundTheme = async (url: string) => {
    const job = ++backgroundThemeJob
    const nextAccent = await extractDominantAccentFromImage(url).catch(() => null)
    if (!nextAccent || job !== backgroundThemeJob) return null
    accent.value = nextAccent
    backgroundThemeColor.value = nextAccent
    applyTheme()
    return nextAccent
  }

  const setBackgroundImage = async (url: string, options?: { syncTheme?: boolean }) => {
    backgroundThemeJob += 1
    const previous = backgroundImageUrl.value
    backgroundImageUrl.value = url
    if (previous && previous !== url) revokeObjectUrl(previous)
    applyTheme()

    const shouldSyncTheme = options?.syncTheme ?? backgroundAutoTheme.value
    if (url && shouldSyncTheme) {
      return syncBackgroundTheme(url)
    }

    return null
  }
  const applyTheme = () => {
    const d = document.documentElement
    d.style.setProperty("--accent", accent.value)
    d.setAttribute("data-theme", preset.value)

    // Accent RGB for rgba()
    const [ar, ag, ab] = hexToRgb(accent.value)
    d.style.setProperty("--accent-rgb", `${ar}, ${ag}, ${ab}`)
    d.style.setProperty("--accent-soft", `rgba(${ar}, ${ag}, ${ab}, 0.10)`)
    d.style.setProperty("--accent-glow", `rgba(${ar}, ${ag}, ${ab}, 0.18)`)
    d.style.setProperty("--shadow-glow", `0 0 20px rgba(${ar}, ${ag}, ${ab}, 0.16)`)

    const activePreset = BACKGROUND_PRESETS[backgroundPreset.value] ?? BACKGROUND_PRESETS.noctis

    const glass = 1 - surfaceOpacity.value / 100
    const mediaMode = backgroundMode.value !== "solid"

    if (!mediaMode) {
      const palette = deriveSolidPalette(activePreset.css, accent.value || activePreset.seed)
      shellBlur.value = 0
      d.style.setProperty("--bg-gradient", palette.background)
      d.style.setProperty("--surface-alpha", glass.toFixed(3))
      d.style.setProperty("--surface-bg", palette.background)
      d.style.setProperty("--shell-bg", palette.background)
      d.style.setProperty("--shell-bg-soft", withAlpha(palette.surface, 0.98))
      d.style.setProperty("--surface-1", withAlpha(palette.surface, 0.9))
      d.style.setProperty("--surface-2", withAlpha(palette.surfaceRaised, 0.92))
      d.style.setProperty("--surface-3", withAlpha(palette.surfaceHover, 0.94))
      d.style.setProperty("--border-color", withAlpha(palette.border, 0.42))
      d.style.setProperty("--border-hover", withAlpha(palette.shell, 0.52))
      d.style.setProperty("--shell-border", withAlpha(palette.shell, 0.46))
      d.style.setProperty("--hover-bg", withAlpha(palette.surfaceHover, 0.7))
      d.style.setProperty("--text-secondary", palette.textSecondary)
      d.style.setProperty("--text-tertiary", palette.textTertiary)
    } else {
      shellBlur.value = 20
      d.style.setProperty("--bg-gradient", activePreset.css)
      d.style.setProperty("--surface-alpha", glass.toFixed(3))
      d.style.setProperty("--surface-bg", `rgba(9, 11, 15, ${Math.max(0.58, glass * 1.78)})`)
      d.style.setProperty("--shell-bg", `rgba(7, 9, 13, 0.56)`)
      d.style.setProperty("--shell-bg-soft", `rgba(8, 10, 14, 0.2)`)
      d.style.setProperty("--surface-1", `rgba(255, 255, 255, ${Math.max(0.018, glass * 0.08)})`)
      d.style.setProperty("--surface-2", `rgba(255, 255, 255, ${Math.max(0.028, glass * 0.11)})`)
      d.style.setProperty("--surface-3", `rgba(255, 255, 255, ${Math.max(0.04, glass * 0.15)})`)
      d.style.setProperty("--border-color", `rgba(255, 255, 255, ${Math.max(0.05, glass * 0.15)})`)
      d.style.setProperty("--border-hover", `rgba(255, 255, 255, ${Math.max(0.08, glass * 0.22)})`)
      d.style.setProperty("--shell-border", `rgba(255, 255, 255, ${Math.max(0.06, glass * 0.18)})`)
      d.style.setProperty("--hover-bg", `rgba(255, 255, 255, 0.035)`)
      d.style.setProperty("--text-secondary", "#a7abbb")
      d.style.setProperty("--text-tertiary", "#6e7282")
    }

    d.style.setProperty("--active-bg", `rgba(var(--accent-rgb), 0.12)`)
    d.style.setProperty("--shell-blur", `${shellBlur.value}px`)

    // Background config for App.vue
    d.style.setProperty("--bg-mode", backgroundMode.value)
    d.style.setProperty("--bg-scale", String(bgScale.value))
    d.style.setProperty("--bg-blur", `${bgBlur.value}px`)
    d.style.setProperty("--bg-mask", String(bgMaskOpacity.value / 100))
    d.style.setProperty("--bg-image-url", backgroundImageUrl.value ? `url(${backgroundImageUrl.value})` : "none")
  }

  const currentConfig = computed(() => ({
    theme: preset.value,
    accent: accent.value,
    background_preset: backgroundPreset.value,
    background_mode: backgroundMode.value,
    background_auto_theme: backgroundAutoTheme.value,
    background_theme_color: backgroundThemeColor.value,
    surface_opacity: surfaceOpacity.value,
    background_scale: bgScale.value,
    background_blur: bgBlur.value,
    background_mask_opacity: bgMaskOpacity.value,
  }))

  return {
    preset, accent, backgroundPreset, backgroundMode, backgroundAutoTheme, backgroundThemeColor, surfaceOpacity,
    bgScale, bgBlur, bgMaskOpacity,
    backgroundImageUrl,
    currentConfig,
    initFromConfig, setPreset, setAccent, setBackgroundPreset, setSurfaceOpacity,
    setBackgroundMode, setBgScale, setBgBlur, setBgMaskOpacity,
    setBackgroundAutoTheme, setBackgroundThemeColor, syncBackgroundTheme,
    setBackgroundImage,
  }
}

export function useSidebar() {
  const collapsed = ref(false)
  const toggle = () => { collapsed.value = !collapsed.value }
  return { collapsed, toggle }
}
