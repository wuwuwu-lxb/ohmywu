import { ref, computed } from "vue"
import type { ThemePreset } from "../lib/theme"
import { THEME_PRESETS } from "../lib/theme"

const preset = ref<ThemePreset>("midnight")
const accent = ref(THEME_PRESETS.midnight.accent)
const backgroundMode = ref<"solid" | "image" | "video">("solid")
const backgroundImageUrl = ref("")
const backgroundVideoUrl = ref("")
const bgScale = ref(1.0)
const bgBlur = ref(0)
const bgMaskOpacity = ref(30)
const surfaceOpacity = ref(72)

const hexToRgb = (hex: string): [number, number, number] => {
  const v = parseInt(hex.replace("#", ""), 16)
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255]
}

export function useTheme() {
  const initFromConfig = (cfg: {
    theme: string; accent: string;
    background_mode: string; surface_opacity: number;
    background_scale: number; background_blur: number; background_mask_opacity: number;
  }) => {
    preset.value = (cfg.theme as ThemePreset) || "midnight"
    accent.value = cfg.accent || THEME_PRESETS[preset.value].accent
    backgroundMode.value = (cfg.background_mode as "solid" | "image" | "video") || "solid"
    surfaceOpacity.value = cfg.surface_opacity ?? 72
    bgScale.value = cfg.background_scale ?? 1.0
    bgBlur.value = cfg.background_blur ?? 0
    bgMaskOpacity.value = cfg.background_mask_opacity ?? 30
    applyTheme()
  }

  const setPreset = (p: ThemePreset) => {
    preset.value = p
    accent.value = THEME_PRESETS[p].accent
    applyTheme()
  }
  const setAccent = (color: string) => { accent.value = color; applyTheme() }
  const setSurfaceOpacity = (v: number) => { surfaceOpacity.value = Math.max(35, Math.min(88, v)); applyTheme() }
  const setBackgroundMode = (m: "solid" | "image" | "video") => { backgroundMode.value = m; applyTheme() }
  const setBgScale = (v: number) => { bgScale.value = v; applyTheme() }
  const setBgBlur = (v: number) => { bgBlur.value = v; applyTheme() }
  const setBgMaskOpacity = (v: number) => { bgMaskOpacity.value = v; applyTheme() }
  const setBackgroundImage = (url: string) => { backgroundImageUrl.value = url; applyTheme() }
  const setBackgroundVideo = (url: string) => { backgroundVideoUrl.value = url; applyTheme() }

  const applyTheme = () => {
    const d = document.documentElement
    d.style.setProperty("--accent", accent.value)
    d.setAttribute("data-theme", preset.value)

    // Accent RGB for rgba()
    const [ar, ag, ab] = hexToRgb(accent.value)
    d.style.setProperty("--accent-rgb", `${ar}, ${ag}, ${ab}`)

    // Solid background: accent-derived gradient (SPlayer solid mode)
    const hue = Math.round((Math.atan2(ag - 128, ar - 128) * 180) / Math.PI + 180)
    d.style.setProperty("--bg-gradient", `
      radial-gradient(ellipse 60% 40% at 30% 30%, rgba(var(--accent-rgb), 0.12) 0%, transparent 60%),
      radial-gradient(ellipse 50% 45% at 70% 70%, rgba(var(--accent-rgb), 0.08) 0%, transparent 55%),
      linear-gradient(160deg, hsl(${hue}, 15%, 7%) 0%, hsl(${hue + 10}, 12%, 9%) 50%, hsl(${hue - 10}, 14%, 7%) 100%)
    `)

    // Surface translucency
    const s = 1 - surfaceOpacity.value / 100
    d.style.setProperty("--surface-bg", `rgba(18, 18, 22, ${s * 0.8})`)
    d.style.setProperty("--surface-1", `rgba(18, 18, 22, ${Math.max(0.06, s * 0.35)})`)
    d.style.setProperty("--surface-2", `rgba(18, 18, 22, ${Math.max(0.10, s * 0.5)})`)
    d.style.setProperty("--surface-3", `rgba(18, 18, 22, ${Math.max(0.14, s * 0.65)})`)
    d.style.setProperty("--border-color", `rgba(var(--accent-rgb), ${Math.max(0.06, s * 0.25)})`)
    d.style.setProperty("--border-hover", `rgba(var(--accent-rgb), ${Math.max(0.10, s * 0.4)})`)
    d.style.setProperty("--hover-bg", `rgba(var(--accent-rgb), 0.08)`)
    d.style.setProperty("--active-bg", `rgba(var(--accent-rgb), 0.14)`)

    // Background config for App.vue
    d.style.setProperty("--bg-mode", backgroundMode.value)
    d.style.setProperty("--bg-scale", String(bgScale.value))
    d.style.setProperty("--bg-blur", `${bgBlur.value}px`)
    d.style.setProperty("--bg-mask", String(bgMaskOpacity.value / 100))
    d.style.setProperty("--bg-image-url", backgroundImageUrl.value ? `url(${backgroundImageUrl.value})` : "none")
    d.style.setProperty("--bg-video-url", backgroundVideoUrl.value)
  }

  const currentConfig = computed(() => ({
    theme: preset.value,
    accent: accent.value,
    background_mode: backgroundMode.value,
    surface_opacity: surfaceOpacity.value,
    background_scale: bgScale.value,
    background_blur: bgBlur.value,
    background_mask_opacity: bgMaskOpacity.value,
  }))

  return {
    preset, accent, backgroundMode, surfaceOpacity,
    bgScale, bgBlur, bgMaskOpacity,
    backgroundImageUrl, backgroundVideoUrl,
    currentConfig,
    initFromConfig, setPreset, setAccent, setSurfaceOpacity,
    setBackgroundMode, setBgScale, setBgBlur, setBgMaskOpacity,
    setBackgroundImage, setBackgroundVideo,
  }
}

export function useSidebar() {
  const collapsed = ref(false)
  const toggle = () => { collapsed.value = !collapsed.value }
  return { collapsed, toggle }
}
