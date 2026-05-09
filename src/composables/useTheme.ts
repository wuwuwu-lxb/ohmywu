import { ref } from "vue"
import type { ThemePreset } from "../lib/theme"
import { THEME_PRESETS } from "../lib/theme"

const preset = ref<ThemePreset>("midnight")
const accent = ref(THEME_PRESETS.midnight.accent)
const collapsed = ref(false)

export function useTheme() {
  const setPreset = (p: ThemePreset) => {
    preset.value = p
    accent.value = THEME_PRESETS[p].accent
    applyTheme()
  }

  const setAccent = (color: string) => {
    accent.value = color
    applyTheme()
  }

  const applyTheme = () => {
    document.documentElement.style.setProperty("--accent", accent.value)
    document.documentElement.setAttribute("data-theme", preset.value)
  }

  return { preset, accent, setPreset, setAccent }
}

export function useSidebar() {
  const toggle = () => { collapsed.value = !collapsed.value }
  return { collapsed, toggle }
}
