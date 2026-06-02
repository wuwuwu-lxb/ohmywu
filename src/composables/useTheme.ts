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
const backgroundSolidColor = ref("#111827")
const backgroundPreset = ref<BackgroundPreset>("noctis")
const backgroundMode = ref<"solid" | "image">("solid")
const backgroundImageUrl = ref("")
const backgroundAutoTheme = ref(true)
const backgroundThemeColor = ref("")
const bgScale = ref(1.0)
const bgBlur = ref(0)
const bgMaskOpacity = ref(30)
const surfaceOpacity = ref(72)
const shellBlur = ref(0)
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
  const surface = mixHex(background, "#ffffff", 0.045)
  const surfaceRaised = mixHex(background, "#ffffff", 0.075)
  const surfaceHover = mixHex(background, "#ffffff", 0.105)
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
    background_solid_color?: string;
    background_preset: string; background_mode: string; surface_opacity: number;
    background_scale: number; background_blur: number; background_mask_opacity: number;
    background_auto_theme?: boolean;
    background_theme_color?: string | null;
  }) => {
    preset.value = (cfg.theme as ThemePreset) || "midnight"
    accent.value = cfg.accent || THEME_PRESETS[preset.value].accent
    backgroundPreset.value = (cfg.background_preset as BackgroundPreset) || "noctis"
    backgroundSolidColor.value = cfg.background_solid_color || BACKGROUND_PRESETS[backgroundPreset.value]?.css || "#111827"
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
  const setBackgroundSolidColor = (color: string) => {
    backgroundSolidColor.value = color
    applyTheme()
  }
  const setBackgroundPreset = (p: BackgroundPreset) => {
    backgroundPreset.value = p
    if (backgroundMode.value === "solid") {
      backgroundSolidColor.value = BACKGROUND_PRESETS[p].css
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
    d.style.setProperty("--accent-soft", `rgba(${ar}, ${ag}, ${ab}, 0.12)`)
    d.style.setProperty("--accent-glow", `rgba(${ar}, ${ag}, ${ab}, 0.08)`)
    d.style.setProperty("--shadow-glow", `0 0 0 1px rgba(${ar}, ${ag}, ${ab}, 0.12)`)

    const activePreset = BACKGROUND_PRESETS[backgroundPreset.value] ?? BACKGROUND_PRESETS.noctis

    const glass = 1 - surfaceOpacity.value / 100
    const mediaMode = backgroundMode.value !== "solid"

    if (!mediaMode) {
      const palette = deriveSolidPalette(
        backgroundSolidColor.value || activePreset.css,
        accent.value || activePreset.seed,
      )
      shellBlur.value = 0
      const [sr, sg, sb] = hexToRgb(palette.surface)
      const [br, bg, bb] = hexToRgb(palette.border)
      d.style.setProperty("--surface-rgb", `${sr}, ${sg}, ${sb}`)
      d.style.setProperty("--border-rgb", `${br}, ${bg}, ${bb}`)
      d.style.setProperty("--bg-gradient", palette.background)
      d.style.setProperty("--surface-alpha", glass.toFixed(3))
      d.style.setProperty("--surface-bg", palette.background)
      d.style.setProperty("--shell-bg", palette.background)
      d.style.setProperty("--shell-bg-soft", withAlpha(palette.surface, 0.92))
      d.style.setProperty("--surface-1", withAlpha(palette.surface, 0.74))
      d.style.setProperty("--surface-2", withAlpha(palette.surfaceRaised, 0.78))
      d.style.setProperty("--surface-3", withAlpha(palette.surfaceHover, 0.82))
      d.style.setProperty("--panel-bg", withAlpha(palette.surface, 0.74))
      d.style.setProperty("--control-bg", withAlpha(palette.surfaceRaised, 0.72))
      d.style.setProperty("--control-bg-focus", withAlpha(palette.surfaceHover, 0.78))
      d.style.setProperty("--border-color", withAlpha(palette.border, 0.32))
      d.style.setProperty("--border-hover", withAlpha(palette.shell, 0.42))
      d.style.setProperty("--shell-border", withAlpha(palette.shell, 0.34))
      d.style.setProperty("--hover-bg", withAlpha(palette.surfaceHover, 0.56))
      d.style.setProperty("--text-secondary", palette.textSecondary)
      d.style.setProperty("--text-tertiary", palette.textTertiary)
    } else {
      shellBlur.value = 0
      d.style.setProperty("--bg-gradient", backgroundSolidColor.value || activePreset.css)
      d.style.setProperty("--surface-alpha", glass.toFixed(3))
      d.style.setProperty("--surface-rgb", "18, 22, 30")
      d.style.setProperty("--border-rgb", "255, 255, 255")
      d.style.setProperty("--surface-bg", `rgba(14, 18, 24, ${Math.max(0.24, glass * 0.86)})`)
      d.style.setProperty("--shell-bg", `rgba(255, 255, 255, 0.01)`)
      d.style.setProperty("--shell-bg-soft", `rgba(255, 255, 255, ${Math.max(0.025, glass * 0.08)})`)
      d.style.setProperty("--surface-1", `rgba(255, 255, 255, ${Math.max(0.035, glass * 0.12)})`)
      d.style.setProperty("--surface-2", `rgba(255, 255, 255, ${Math.max(0.052, glass * 0.16)})`)
      d.style.setProperty("--surface-3", `rgba(255, 255, 255, ${Math.max(0.07, glass * 0.2)})`)
      d.style.setProperty("--panel-bg", `rgba(255, 255, 255, ${Math.max(0.035, glass * 0.12)})`)
      d.style.setProperty("--control-bg", `rgba(255, 255, 255, ${Math.max(0.048, glass * 0.15)})`)
      d.style.setProperty("--control-bg-focus", `rgba(255, 255, 255, ${Math.max(0.065, glass * 0.18)})`)
      d.style.setProperty("--border-color", `rgba(255, 255, 255, ${Math.max(0.045, glass * 0.11)})`)
      d.style.setProperty("--border-hover", `rgba(255, 255, 255, ${Math.max(0.075, glass * 0.17)})`)
      d.style.setProperty("--shell-border", `rgba(255, 255, 255, ${Math.max(0.055, glass * 0.14)})`)
      d.style.setProperty("--hover-bg", `rgba(255, 255, 255, 0.032)`)
      d.style.setProperty("--text-secondary", "#a7abbb")
      d.style.setProperty("--text-tertiary", "#6e7282")
    }

    d.style.setProperty("--active-bg", `rgba(var(--accent-rgb), 0.09)`)
    d.style.setProperty("--focus-ring", `0 0 0 1px rgba(${ar}, ${ag}, ${ab}, 0.28)`)
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
    background_solid_color: backgroundSolidColor.value,
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
    preset, accent, backgroundSolidColor, backgroundPreset, backgroundMode, backgroundAutoTheme, backgroundThemeColor, surfaceOpacity,
    bgScale, bgBlur, bgMaskOpacity,
    backgroundImageUrl,
    currentConfig,
    initFromConfig, setPreset, setAccent, setBackgroundSolidColor, setBackgroundPreset, setSurfaceOpacity,
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
