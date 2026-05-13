<script setup lang="ts">
import { ref, onMounted } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { useTheme } from "../composables/useTheme"
import { THEME_PRESETS } from "../lib/theme"
import type { ThemePreset } from "../lib/theme"

const { preset, accent, setPreset, setAccent } = useTheme()
const presets = Object.entries(THEME_PRESETS) as [ThemePreset, { label: string; accent: string }][]

// LLM config
interface LlmConfig {
  provider_type: string
  endpoint: string
  model: string
  api_key?: string
}

interface AppConfig {
  policy_mode: string
  theme: string
  accent: string
  llm_provider: LlmConfig | null
}

const llmEnabled = ref(false)
const llmProvider = ref("ollama")
const llmEndpoint = ref("http://localhost:11434")
const llmModel = ref("qwen2.5")
const llmApiKey = ref("")
const configSaving = ref(false)
const configMsg = ref("")

onMounted(async () => {
  try {
    const cfg = await invoke<AppConfig>("get_config")
    if (cfg.llm_provider) {
      llmEnabled.value = true
      llmProvider.value = cfg.llm_provider.provider_type
      llmEndpoint.value = cfg.llm_provider.endpoint
      llmModel.value = cfg.llm_provider.model
      llmApiKey.value = cfg.llm_provider.api_key || ""
    }
  } catch (e) {
    console.error("Load config:", e)
  }
})

async function saveLlmConfig() {
  configSaving.value = true
  configMsg.value = ""
  try {
    const current = await invoke<AppConfig>("get_config")
    const updated: AppConfig = {
      ...current,
      llm_provider: llmEnabled.value
        ? {
            provider_type: llmProvider.value,
            endpoint: llmEndpoint.value,
            model: llmModel.value,
            api_key: llmApiKey.value || undefined,
          }
        : null,
    }
    await invoke("save_config", { config: updated })
    configMsg.value = "已保存"
    setTimeout(() => (configMsg.value = ""), 2000)
  } catch (e) {
    configMsg.value = `保存失败：${e}`
  } finally {
    configSaving.value = false
  }
}

async function testConnection() {
  configMsg.value = "测试中..."
  try {
    const result = await invoke<string>("test_llm_connection")
    configMsg.value = result
  } catch (e) {
    configMsg.value = String(e)
  }
}
</script>

<template>
  <div class="settings-view">
    <h2 class="view-title">设置</h2>

    <!-- theme -->
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
        <button class="reset-btn" @click="setAccent(THEME_PRESETS[preset].accent)">重置</button>
      </div>
    </section>

    <!-- LLM -->
    <section class="setting-section">
      <h3 class="section-title">LLM 配置</h3>
      <label class="toggle-row">
        <input type="checkbox" v-model="llmEnabled" />
        <span>启用 LLM</span>
      </label>

      <div v-if="llmEnabled" class="llm-form">
        <div class="form-field">
          <label>Provider</label>
          <select v-model="llmProvider" class="form-input">
            <option value="ollama">Ollama (本地)</option>
            <option value="openai_compatible">OpenAI Compatible</option>
          </select>
        </div>

        <div class="form-field">
          <label>Endpoint</label>
          <input
            v-model="llmEndpoint"
            class="form-input"
            type="text"
            :placeholder="llmProvider === 'ollama' ? 'http://localhost:11434' : 'https://api.openai.com'"
          />
        </div>

        <div class="form-field">
          <label>Model</label>
          <input v-model="llmModel" class="form-input" type="text" placeholder="qwen2.5" />
        </div>

        <div v-if="llmProvider === 'openai_compatible'" class="form-field">
          <label>API Key</label>
          <input v-model="llmApiKey" class="form-input" type="password" placeholder="sk-..." />
        </div>

        <div class="form-actions">
          <button class="save-btn" :disabled="configSaving" @click="saveLlmConfig">
            {{ configSaving ? "保存中..." : "保存" }}
          </button>
          <button class="test-btn" :disabled="configSaving" @click="testConnection">
            测试连接
          </button>
          <span v-if="configMsg" class="config-msg">{{ configMsg }}</span>
        </div>
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

/* LLM form */
.toggle-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
}

.toggle-row input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  cursor: pointer;
}

.llm-form {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-field label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.form-input {
  padding: 8px 10px;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}

.form-input:focus {
  border-color: var(--accent);
}

.form-input::placeholder {
  color: var(--text-tertiary);
}

select.form-input {
  cursor: pointer;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 4px;
}

.save-btn {
  padding: 6px 16px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: var(--text-on-accent);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.save-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.test-btn {
  padding: 6px 12px;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  font-family: inherit;
}

.test-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.test-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.config-msg {
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
