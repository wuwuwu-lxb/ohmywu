<script setup lang="ts">
import { ref, computed, onMounted } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { useTheme } from "../composables/useTheme"
import { THEME_PRESETS } from "../lib/theme"
import type { ThemePreset } from "../lib/theme"

const {
  preset, accent, surfaceOpacity, backgroundMode,
  bgScale, bgBlur, bgMaskOpacity,
  setPreset, setAccent, setSurfaceOpacity,
  setBackgroundMode, setBgScale, setBgBlur, setBgMaskOpacity,
  setBackgroundImage, setBackgroundVideo,
} = useTheme()

const presets = Object.entries(THEME_PRESETS) as [ThemePreset, { label: string; accent: string }][]

interface ProviderInfo {
  id: string; name: string; apiFormat: string; icon?: string; iconColor?: string
  defaultModel: string; supportsTools: boolean; websiteUrl?: string
}
interface LlmConfig {
  provider_type: string; api_format: string; endpoint: string
  model: string; api_key?: string; max_tokens?: number
}
interface AppConfig {
  policy_mode: string; theme: string; accent: string
  background_mode: string; surface_opacity: number
  background_scale: number; background_blur: number; background_mask_opacity: number
  llm_provider: LlmConfig | null
}

const PROVIDER_ENDPOINTS: Record<string, string> = {
  openai: "https://api.openai.com/v1", anthropic: "https://api.anthropic.com",
  deepseek: "https://api.deepseek.com", gemini: "https://generativelanguage.googleapis.com",
  ollama: "http://localhost:11434", moonshot: "https://api.moonshot.cn/v1",
  zhipu: "https://open.bigmodel.cn/api/paas/v4", qwen: "https://dashscope.aliyuncs.com/compatible-mode/v1",
  minimax: "https://api.minimaxi.com/v1",
}
function defaultEndpointFor(id: string) { return PROVIDER_ENDPOINTS[id] ?? "" }
function needsKeyFor(id: string) { return id !== "ollama" }

const providers = ref<ProviderInfo[]>([])
const llmEnabled = ref(false)
const llmProvider = ref("ollama")
const llmEndpoint = ref("http://localhost:11434")
const llmModel = ref("qwen2.5")
const llmApiKey = ref("")

const appearanceSaving = ref(false)
const appearanceMsg = ref("")
const configSaving = ref(false)
const configMsg = ref("")
const testingConnection = ref(false)
const testSuccess = ref(false)
const bgUploading = ref(false)

const currentProvider = computed(() =>
  providers.value.find((p) => p.id === llmProvider.value) ?? providers.value.find((p) => p.id === "ollama")
)

function onProviderChange() {
  const p = currentProvider.value
  if (!p) return
  llmEndpoint.value = defaultEndpointFor(p.id)
  llmModel.value = p.defaultModel
  llmApiKey.value = ""
}

onMounted(async () => {
  try {
    providers.value = await invoke<ProviderInfo[]>("get_llm_providers")
    const cfg = await invoke<AppConfig>("get_config")
    if (cfg.llm_provider) {
      llmEnabled.value = true
      llmProvider.value = cfg.llm_provider.provider_type
      llmEndpoint.value = cfg.llm_provider.endpoint
      llmModel.value = cfg.llm_provider.model
      llmApiKey.value = cfg.llm_provider.api_key || ""
      return
    }
    const ollama = providers.value.find((p) => p.id === "ollama")
    if (ollama) {
      llmProvider.value = ollama.id
      llmEndpoint.value = defaultEndpointFor(ollama.id)
      llmModel.value = ollama.defaultModel
    }
  } catch (e) { console.error("Load config:", e) }
})

async function saveAppearance() {
  appearanceSaving.value = true; appearanceMsg.value = ""
  try {
    const current = await invoke<AppConfig>("get_config")
    await invoke("save_config", {
      config: {
        ...current,
        theme: preset.value,
        accent: accent.value,
        background_mode: backgroundMode.value,
        surface_opacity: surfaceOpacity.value,
        background_scale: bgScale.value,
        background_blur: bgBlur.value,
        background_mask_opacity: bgMaskOpacity.value,
      }
    })
    appearanceMsg.value = "已保存"
    setTimeout(() => (appearanceMsg.value = ""), 2000)
  } catch (e) {
    appearanceMsg.value = `保存失败：${e}`
  } finally { appearanceSaving.value = false }
}

async function handleBgFileUpload(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  // validate
  const isVideo = file.type.startsWith("video/")
  const isImage = file.type.startsWith("image/")
  if (!isVideo && !isImage) return
  if (file.size > 100 * 1024 * 1024) { appearanceMsg.value = "文件不能超过 100MB"; return }

  bgUploading.value = true
  try {
    const ext = file.name.split(".").pop() || "jpg"
    const filename = isVideo ? `bg_video.${ext}` : `bg_image.${ext}`
    const buf = await file.arrayBuffer()
    const data = Array.from(new Uint8Array(buf))

    await invoke("save_background_file", { data, filename })

    // Read back and set URL
    const path = await invoke<string | null>("get_background_path")
    if (path) {
      const url = `asset://localhost/${path}`
      if (isVideo) {
        setBackgroundMode("video")
        setBackgroundVideo(url)
      } else {
        setBackgroundMode("image")
        setBackgroundImage(url)
      }
    }
    appearanceMsg.value = "背景已更新，记得保存外观"
  } catch (e) {
    appearanceMsg.value = `上传失败：${e}`
  } finally {
    bgUploading.value = false
    input.value = ""
  }
}

function clearBackground() {
  setBackgroundMode("solid")
  setBackgroundImage("")
  setBackgroundVideo("")
}

async function saveLlmConfig() {
  configSaving.value = true; configMsg.value = ""
  try {
    const current = await invoke<AppConfig>("get_config")
    await invoke("save_config", {
      config: {
        ...current,
        llm_provider: llmEnabled.value ? {
          provider_type: llmProvider.value,
          api_format: currentProvider.value?.apiFormat || "openai_chat",
          endpoint: llmEndpoint.value,
          model: llmModel.value,
          api_key: llmApiKey.value || undefined,
        } : null,
      }
    })
    configMsg.value = "已保存"
    setTimeout(() => (configMsg.value = ""), 2000)
  } catch (e) {
    configMsg.value = `保存失败：${e}`
  } finally { configSaving.value = false }
}

async function testWithCurrentForm() {
  testingConnection.value = true; configMsg.value = ""; testSuccess.value = false
  try {
    const result = await invoke<{ success: boolean; message: string; model?: string; latency_ms?: number }>(
      "test_llm_connection_with_config", {
        providerType: llmProvider.value, endpoint: llmEndpoint.value,
        model: llmModel.value, apiKey: llmApiKey.value || null,
      }
    )
    testSuccess.value = result.success
    configMsg.value = result.message
  } catch (e) {
    testSuccess.value = false
    configMsg.value = String(e)
  } finally { testingConnection.value = false }
}
</script>

<template>
  <div class="settings-view">
    <!-- ── Appearance Card ── -->
    <section class="card">
      <div class="card-header"><h3 class="card-title">外观</h3></div>

      <div class="field-group">
        <label class="field-label">主题预设</label>
        <div class="preset-grid">
          <button v-for="[key, val] in presets" :key="key"
            :class="['preset-btn', { active: preset === key }]"
            @click="setPreset(key)"
            :style="{ '--preset-color': val.accent }">
            <span class="preset-swatch" />
            <span class="preset-label">{{ val.label }}</span>
          </button>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">强调色</label>
        <div class="color-row">
          <input type="color" :value="accent"
            @input="setAccent(($event.target as HTMLInputElement).value)" class="color-input" />
          <span class="color-value">{{ accent }}</span>
          <button class="reset-btn" @click="setAccent(THEME_PRESETS[preset].accent)">重置</button>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">透明度</label>
        <div class="slider-row">
          <input type="range" min="35" max="88" :value="surfaceOpacity"
            @input="setSurfaceOpacity(Number(($event.target as HTMLInputElement).value))" class="opacity-slider" />
          <span class="slider-value">{{ surfaceOpacity }}%</span>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">背景模式</label>
        <div class="mode-row">
          <button :class="['mode-btn', { active: backgroundMode === 'solid' }]" @click="setBackgroundMode('solid')">纯色</button>
          <button :class="['mode-btn', { active: backgroundMode === 'image' }]" @click="setBackgroundMode('image')">图片</button>
          <button :class="['mode-btn', { active: backgroundMode === 'video' }]" @click="setBackgroundMode('video')">视频</button>
          <button v-if="backgroundMode !== 'solid'" class="reset-btn" @click="clearBackground">清除</button>
        </div>
      </div>

      <div v-if="backgroundMode !== 'solid'" class="field-group">
        <label class="field-label">{{ backgroundMode === 'video' ? '选择视频' : '选择图片' }}</label>
        <input type="file"
          :accept="backgroundMode === 'video' ? 'video/*' : 'image/*'"
          @change="handleBgFileUpload" class="file-input"
          :disabled="bgUploading" />
        <span v-if="bgUploading" class="msg">上传中...</span>
      </div>

      <template v-if="backgroundMode !== 'solid'">
        <div class="field-group">
          <label class="field-label">缩放</label>
          <div class="slider-row">
            <input type="range" min="100" max="200" :value="Math.round(bgScale * 100)"
              @input="setBgScale(Number(($event.target as HTMLInputElement).value) / 100)" class="opacity-slider" />
            <span class="slider-value">{{ Math.round(bgScale * 100) }}%</span>
          </div>
        </div>
        <div class="field-group">
          <label class="field-label">模糊</label>
          <div class="slider-row">
            <input type="range" min="0" max="40" :value="bgBlur"
              @input="setBgBlur(Number(($event.target as HTMLInputElement).value))" class="opacity-slider" />
            <span class="slider-value">{{ bgBlur }}px</span>
          </div>
        </div>
        <div class="field-group">
          <label class="field-label">遮罩深度</label>
          <div class="slider-row">
            <input type="range" min="0" max="80" :value="bgMaskOpacity"
              @input="setBgMaskOpacity(Number(($event.target as HTMLInputElement).value))" class="opacity-slider" />
            <span class="slider-value">{{ bgMaskOpacity }}%</span>
          </div>
        </div>
      </template>

      <div class="card-actions">
        <button class="save-btn" :disabled="appearanceSaving" @click="saveAppearance">
          {{ appearanceSaving ? "保存中..." : "保存外观" }}
        </button>
        <span v-if="appearanceMsg" class="msg" :class="{ error: appearanceMsg.startsWith('保存失败') }">{{ appearanceMsg }}</span>
      </div>
    </section>

    <!-- ── Model Card ── -->
    <section class="card">
      <div class="card-header">
        <h3 class="card-title">模型</h3>
        <label class="toggle-switch">
          <input type="checkbox" v-model="llmEnabled" />
          <span class="toggle-track" />
        </label>
      </div>

      <div v-if="llmEnabled" class="model-fields">
        <div class="field-group">
          <label class="field-label">Provider</label>
          <div class="provider-row">
            <select v-model="llmProvider" class="form-input" @change="onProviderChange">
              <option v-for="p in providers" :key="p.id" :value="p.id">{{ p.name }}</option>
            </select>
            <span v-if="currentProvider?.iconColor" class="provider-dot" :style="{ background: currentProvider.iconColor }" />
            <span v-if="currentProvider" class="provider-id">{{ currentProvider.id }}</span>
            <a v-if="currentProvider?.websiteUrl" :href="currentProvider.websiteUrl" target="_blank" class="provider-link">官网</a>
          </div>
        </div>
        <div class="field-group">
          <label class="field-label">Endpoint</label>
          <input v-model="llmEndpoint" class="form-input" type="text" :placeholder="currentProvider ? defaultEndpointFor(currentProvider.id) : ''" />
        </div>
        <div class="field-group">
          <label class="field-label">Model</label>
          <input v-model="llmModel" class="form-input" type="text" :placeholder="currentProvider?.defaultModel || ''" />
        </div>
        <div v-if="currentProvider && needsKeyFor(currentProvider.id)" class="field-group">
          <label class="field-label">API Key</label>
          <input v-model="llmApiKey" class="form-input" type="password" placeholder="sk-..." />
        </div>
        <div class="card-actions">
          <button class="save-btn" :disabled="configSaving" @click="saveLlmConfig">{{ configSaving ? "保存中..." : "保存" }}</button>
          <button class="test-btn" :disabled="testingConnection" @click="testWithCurrentForm">{{ testingConnection ? "测试中..." : "测试连接" }}</button>
          <span v-if="configMsg" class="msg" :class="{ error: !testSuccess && configMsg !== '已保存' }">{{ configMsg }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-view { padding: 24px 32px; max-width: 560px; display: flex; flex-direction: column; gap: 20px; }

.card {
  background: var(--surface-2); border: 1px solid var(--border-color);
  border-radius: var(--radius-xl); padding: 20px 24px;
  box-shadow: var(--shadow-surface); transition: background 0.3s ease, border-color 0.3s ease;
}
.card-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; }
.card-title { font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: 0.3px; }

.field-group { margin-bottom: 16px; }
.field-group:last-child { margin-bottom: 0; }
.field-label { display: block; font-size: 11px; font-weight: 600; color: var(--text-tertiary); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; }

.preset-grid { display: flex; gap: 8px; }
.preset-btn {
  display: flex; align-items: center; gap: 8px; padding: 8px 14px;
  border: 1px solid var(--border-color); border-radius: var(--radius-md);
  background: var(--surface-1); color: var(--text-primary);
  font-size: 13px; cursor: pointer; font-family: inherit; transition: all 0.15s ease;
}
.preset-btn:hover { border-color: var(--preset-color); background: var(--surface-2); }
.preset-btn.active { border-color: var(--preset-color); background: color-mix(in srgb, var(--preset-color) 15%, transparent); box-shadow: 0 0 12px color-mix(in srgb, var(--preset-color) 30%, transparent); }
.preset-swatch { width: 12px; height: 12px; border-radius: 50%; background: var(--preset-color); }

.color-row { display: flex; align-items: center; gap: 10px; }
.color-input { width: 36px; height: 36px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); padding: 2px; background: none; cursor: pointer; }
.color-input::-webkit-color-swatch-wrapper { padding: 0; }
.color-input::-webkit-color-swatch { border: none; border-radius: 4px; }
.color-value { font-family: var(--font-mono); font-size: 13px; color: var(--text-secondary); }
.reset-btn { padding: 4px 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--surface-1); color: var(--text-secondary); font-size: 12px; cursor: pointer; font-family: inherit; }
.reset-btn:hover { border-color: var(--text-tertiary); color: var(--text-primary); }

.mode-row { display: flex; gap: 8px; align-items: center; }
.mode-btn {
  padding: 6px 14px; border: 1px solid var(--border-color); border-radius: var(--radius-sm);
  background: var(--surface-1); color: var(--text-secondary); font-size: 12px;
  cursor: pointer; font-family: inherit; transition: all 0.15s ease;
}
.mode-btn:hover { border-color: var(--border-hover); color: var(--text-primary); }
.mode-btn.active { border-color: var(--accent); color: var(--accent); background: var(--active-bg); }

.file-input { font-size: 12px; color: var(--text-secondary); width: 100%; }
.file-input::file-selector-button {
  padding: 4px 12px; border: 1px solid var(--border-color); border-radius: var(--radius-sm);
  background: var(--surface-1); color: var(--text-secondary); cursor: pointer;
  font-family: inherit; font-size: 12px; margin-right: 10px;
}

.slider-row { display: flex; align-items: center; gap: 12px; }
.opacity-slider { flex: 1; -webkit-appearance: none; appearance: none; height: 4px; border-radius: 2px; background: rgba(255,255,255,0.12); outline: none; cursor: pointer; }
.opacity-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 16px; height: 16px; border-radius: 50%; background: var(--accent); border: 2px solid var(--text-primary); cursor: pointer; }
.opacity-slider::-moz-range-thumb { width: 16px; height: 16px; border-radius: 50%; background: var(--accent); border: 2px solid var(--text-primary); cursor: pointer; }
.slider-value { min-width: 40px; font-size: 13px; font-family: var(--font-mono); color: var(--text-secondary); text-align: right; }

.model-fields { display: flex; flex-direction: column; gap: 14px; }
.toggle-switch input { display: none; }
.toggle-track { display: block; width: 36px; height: 20px; border-radius: 10px; background: rgba(255,255,255,0.1); position: relative; cursor: pointer; transition: background 0.2s ease; }
.toggle-track::after { content: ""; position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: var(--text-secondary); transition: all 0.2s ease; }
.toggle-switch input:checked + .toggle-track { background: var(--accent); }
.toggle-switch input:checked + .toggle-track::after { left: 18px; background: var(--text-on-accent); }
.provider-row { display: flex; align-items: center; gap: 8px; }
.provider-row select { flex: 1; }
.provider-dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }
.provider-id { font-size: 11px; color: var(--text-tertiary); font-family: var(--font-mono); }
.provider-link { font-size: 11px; color: var(--accent); text-decoration: none; }
.provider-link:hover { text-decoration: underline; }
.form-input { width: 100%; padding: 8px 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: rgba(0,0,0,0.2); color: var(--text-primary); font-size: 13px; font-family: inherit; outline: none; }
.form-input:focus { border-color: var(--accent); }
.form-input::placeholder { color: var(--text-tertiary); }
select.form-input { cursor: pointer; }

.card-actions { display: flex; align-items: center; gap: 10px; margin-top: 6px; }
.save-btn { padding: 6px 16px; border: none; border-radius: var(--radius-sm); background: var(--accent); color: var(--text-on-accent); font-size: 13px; font-weight: 600; cursor: pointer; font-family: inherit; }
.save-btn:disabled { opacity: 0.5; cursor: default; }
.test-btn { padding: 6px 12px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--surface-1); color: var(--text-secondary); font-size: 13px; cursor: pointer; font-family: inherit; transition: all 0.15s ease; }
.test-btn:disabled { opacity: 0.5; cursor: default; }
.test-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.msg { font-size: 12px; color: var(--text-secondary); }
.msg.error { color: var(--red); }
</style>
