<script setup lang="ts">
import { useTheme } from "../composables/useTheme"
import { THEME_PRESETS } from "../lib/theme"
import type { ThemePreset } from "../lib/theme"

const { preset, accent, setPreset, setAccent } = useTheme()

const presets = Object.entries(THEME_PRESETS) as [ThemePreset, { label: string; accent: string }][]
</script>

<template>
  <div class="settings-view">
    <h2 class="view-title">设置</h2>

    <section class="setting-section">
      <h3 class="section-title">主题预设</h3>
      <div class="preset-grid">
        <button
          v-for="[key, val] in presets"
          :key="key"
          :class="['preset-btn', { active: preset === key }]"
          @click="setPreset(key)"
          :style="{ '--preset-color': val.accent }"
        >
          <span class="preset-swatch" />
          <span class="preset-label">{{ val.label }}</span>
        </button>
      </div>
    </section>

    <section class="setting-section">
      <h3 class="section-title">强调色</h3>
      <div class="color-picker-row">
        <input
          type="color"
          :value="accent"
          @input="setAccent(($event.target as HTMLInputElement).value)"
          class="color-input"
        />
        <span class="color-value">{{ accent }}</span>
        <button
          class="reset-btn"
          @click="setAccent(THEME_PRESETS[preset].accent)"
        >
          重置
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-view {
  padding: 24px 32px;
  max-width: 480px;
}

.view-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 24px;
}

.setting-section {
  margin-bottom: 24px;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.preset-grid {
  display: flex;
  gap: 8px;
}

.preset-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  font-family: inherit;
}

.preset-btn:hover {
  border-color: var(--preset-color);
}

.preset-btn.active {
  border-color: var(--preset-color);
  background: color-mix(in srgb, var(--preset-color) 15%, var(--bg-surface));
}

.preset-swatch {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--preset-color);
}

.preset-label {
  font-weight: 500;
}

.color-picker-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.color-input {
  width: 36px;
  height: 36px;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  padding: 2px;
  background: none;
  cursor: pointer;
}

.color-input::-webkit-color-swatch-wrapper {
  padding: 0;
}

.color-input::-webkit-color-swatch {
  border: none;
  border-radius: 4px;
}

.color-value {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
}

.reset-btn {
  padding: 4px 10px;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  font-family: inherit;
}

.reset-btn:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}
</style>
