<script setup lang="ts">
import { ref, computed, onMounted } from "vue"
import { convertFileSrc, invoke } from "@tauri-apps/api/core"
import ThemeSelect from "../components/ThemeSelect.vue"
import { useTheme } from "../composables/useTheme"
import { useChatStore, type AgentMode } from "../stores/chat"
import { BACKGROUND_PRESETS, THEME_PRESETS } from "../lib/theme"
import type { CapabilityInfo, ToolRisk } from "../lib/tools"
import type { BackgroundPreset, ThemePreset } from "../lib/theme"

type PolicyMode = "Sandbox" | "Danger"

interface ProviderInfo {
  id: string
  name: string
  apiFormat: string
  icon?: string
  iconColor?: string
  defaultModel: string
  supportsTools: boolean
  websiteUrl?: string
}

interface LlmConfig {
  provider_type: string
  api_format: string
  endpoint: string
  model: string
  api_key?: string
  max_tokens?: number
}

interface PermissionRule {
  effect: "allow" | "deny"
  tool: string
}

interface AppConfig {
  policy_mode: PolicyMode
  theme: string
  accent: string
  background_preset: string
  background_mode: string
  surface_opacity: number
  background_scale: number
  background_blur: number
  background_mask_opacity: number
  background_auto_theme: boolean
  background_theme_color?: string | null
  agent_mode: AgentMode
  llm_provider: LlmConfig | null
  permissions: {
    rules: PermissionRule[]
  }
}

const chatStore = useChatStore()

const {
  preset,
  accent,
  backgroundPreset,
  backgroundMode,
  backgroundAutoTheme,
  backgroundThemeColor,
  surfaceOpacity,
  bgScale,
  bgBlur,
  bgMaskOpacity,
  backgroundImageUrl,
  setPreset,
  setAccent,
  setSurfaceOpacity,
  setBackgroundPreset,
  setBackgroundMode,
  setBgScale,
  setBgBlur,
  setBgMaskOpacity,
  setBackgroundAutoTheme,
  setBackgroundThemeColor,
  syncBackgroundTheme,
  setBackgroundImage,
} = useTheme()

const presets = Object.entries(THEME_PRESETS) as [ThemePreset, { label: string; accent: string }][]
const backgrounds = Object.entries(BACKGROUND_PRESETS) as [BackgroundPreset, { label: string; css: string }][]

const PROVIDER_ENDPOINTS: Record<string, string> = {
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  deepseek: "https://api.deepseek.com",
  gemini: "https://generativelanguage.googleapis.com",
  ollama: "http://localhost:11434",
  moonshot: "https://api.moonshot.cn/v1",
  zhipu: "https://open.bigmodel.cn/api/paas/v4",
  qwen: "https://dashscope.aliyuncs.com/compatible-mode/v1",
  minimax: "https://api.minimaxi.com/v1",
}

const policyModeOptions: Array<{ value: PolicyMode; label: string; note: string }> = [
  { value: "Sandbox", label: "Sandbox", note: "只允许只读工具，写入和命令直接被策略层拒绝。" },
  { value: "Danger", label: "Danger", note: "允许进入工具执行阶段，再交给权限规则和确认逻辑处理。" },
]

const agentModeOptions: Array<{ value: AgentMode; label: string; note: string }> = [
  { value: "plan", label: "Plan", note: "仅暴露只读工具和 checklist，适合先分析再动手。" },
  { value: "agent", label: "Agent", note: "暴露全部工具，高风险操作默认需要确认。" },
  { value: "auto", label: "Auto", note: "暴露全部工具，高风险操作也会直接执行。" },
]

function defaultEndpointFor(id: string) {
  return PROVIDER_ENDPOINTS[id] ?? ""
}

function needsKeyFor(id: string) {
  return id !== "ollama"
}

function rulesToText(rules: PermissionRule[]): string {
  return rules.map((rule) => `${rule.effect}: ${rule.tool}`).join("\n")
}

function parseRules(text: string): PermissionRule[] {
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))

  return lines.map((line, index) => {
    const matched = line.match(/^(allow|deny)\s*:\s*(.+)$/i)
    if (!matched) {
      throw new Error(`第 ${index + 1} 行格式错误，示例：allow: read`)
    }
    return {
      effect: matched[1].toLowerCase() as "allow" | "deny",
      tool: matched[2].trim(),
    }
  })
}

function isToolVisible(name: string, risk: ToolRisk, mode: AgentMode): boolean {
  if (name === "checklist_write") {
    return true
  }
  return mode === "plan" ? risk === "ReadOnly" : true
}

const providers = ref<ProviderInfo[]>([])
const capabilities = ref<CapabilityInfo[]>([])

const llmEnabled = ref(false)
const llmProvider = ref("ollama")
const llmEndpoint = ref("http://localhost:11434")
const llmModel = ref("qwen2.5")
const llmApiKey = ref("")

const policyMode = ref<PolicyMode>("Sandbox")
const selectedAgentMode = ref<AgentMode>("agent")
const permissionRulesInput = ref("")

const appearanceSaving = ref(false)
const appearanceMsg = ref("")
const configSaving = ref(false)
const configMsg = ref("")
const executionSaving = ref(false)
const executionMsg = ref("")
const testingConnection = ref(false)
const testSuccess = ref(false)
const bgUploading = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)
const backgroundFileLabel = ref("未上传背景")

const currentProvider = computed(() =>
  providers.value.find((p) => p.id === llmProvider.value) ?? providers.value.find((p) => p.id === "ollama")
)
const providerOptions = computed(() =>
  providers.value.map((provider) => ({
    label: provider.name,
    value: provider.id,
  }))
)

const parsedRuleError = computed(() => {
  try {
    parseRules(permissionRulesInput.value)
    return ""
  } catch (error) {
    return String(error)
  }
})

const parsedRules = computed(() => {
  try {
    return parseRules(permissionRulesInput.value)
  } catch {
    return []
  }
})

const allowRuleCount = computed(() => parsedRules.value.filter((rule) => rule.effect === "allow").length)
const denyRuleCount = computed(() => parsedRules.value.filter((rule) => rule.effect === "deny").length)
const hasAllowRules = computed(() => allowRuleCount.value > 0)

const visibleToolCount = computed(() =>
  capabilities.value.filter((cap) => isToolVisible(cap.name, cap.risk_level, selectedAgentMode.value)).length
)

const executionSummary = computed(() => {
  if (policyMode.value === "Sandbox") {
    return "当前是 Sandbox，只读工具可用，写入和 bash 这类高权限能力会先被策略层挡住。"
  }
  if (selectedAgentMode.value === "auto") {
    return "当前是 Danger + Auto，所有工具都可见，高风险工具也会直接执行。"
  }
  if (selectedAgentMode.value === "plan") {
    return "当前是 Danger + Plan，策略允许全部风险等级，但前端只暴露只读工具和 checklist。"
  }
  return "当前是 Danger + Agent，所有工具可见，高风险工具默认要求确认。"
})

function onProviderChange() {
  const provider = currentProvider.value
  if (!provider) return
  llmEndpoint.value = defaultEndpointFor(provider.id)
  llmModel.value = provider.defaultModel
  llmApiKey.value = ""
}

function selectProvider(value: string | number) {
  llmProvider.value = String(value)
  onProviderChange()
}

async function loadSettings() {
  const cfg = await invoke<AppConfig>("get_config")
  providers.value = await invoke<ProviderInfo[]>("get_llm_providers")
  capabilities.value = await invoke<CapabilityInfo[]>("get_capabilities")

  policyMode.value = cfg.policy_mode
  selectedAgentMode.value = cfg.agent_mode
  permissionRulesInput.value = rulesToText(cfg.permissions?.rules ?? [])

  if (cfg.background_mode === "image") {
    const bgPath = await invoke<string | null>("get_background_path").catch(() => null)
    if (bgPath) {
      backgroundFileLabel.value = decodeURIComponent(bgPath.split(/[\\/]/).pop() || "已应用背景")
    }
  }

  if (cfg.llm_provider) {
    llmEnabled.value = true
    llmProvider.value = cfg.llm_provider.provider_type
    llmEndpoint.value = cfg.llm_provider.endpoint
    llmModel.value = cfg.llm_provider.model
    llmApiKey.value = cfg.llm_provider.api_key || ""
  } else {
    const ollama = providers.value.find((provider) => provider.id === "ollama")
    if (ollama) {
      llmProvider.value = ollama.id
      llmEndpoint.value = defaultEndpointFor(ollama.id)
      llmModel.value = ollama.defaultModel
    }
  }
}

onMounted(async () => {
  try {
    await loadSettings()
  } catch (error) {
    console.error("Load config:", error)
  }
})

function chooseBackgroundFile() {
  if (bgUploading.value) return
  fileInputRef.value?.click()
}

async function persistAppearanceConfig() {
  const current = await invoke<AppConfig>("get_config")
  await invoke("save_config", {
    config: {
      ...current,
      theme: preset.value,
      accent: accent.value,
      background_preset: backgroundPreset.value,
      background_mode: backgroundMode.value,
      surface_opacity: surfaceOpacity.value,
      background_scale: bgScale.value,
      background_blur: bgBlur.value,
      background_mask_opacity: bgMaskOpacity.value,
      background_auto_theme: backgroundAutoTheme.value,
      background_theme_color: backgroundThemeColor.value || null,
    },
  })
}

function applyPreset(key: ThemePreset) {
  setPreset(key)
}

function applyAccent(color: string) {
  setAccent(color)
}

async function toggleBackgroundAutoTheme(enabled: boolean) {
  setBackgroundAutoTheme(enabled)
  if (enabled && backgroundMode.value === "image" && backgroundImageUrl.value) {
    const synced = await syncBackgroundTheme(backgroundImageUrl.value)
    if (synced) {
      appearanceMsg.value = `已同步主题色 ${synced}`
    }
  }
}

async function saveAppearance() {
  appearanceSaving.value = true
  appearanceMsg.value = ""
  try {
    await persistAppearanceConfig()
    appearanceMsg.value = "已保存"
    window.setTimeout(() => {
      appearanceMsg.value = ""
    }, 2000)
  } catch (error) {
    appearanceMsg.value = `保存失败：${error}`
  } finally {
    appearanceSaving.value = false
  }
}

async function handleBgFileUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  if (!file.type.startsWith("image/")) {
    appearanceMsg.value = "只支持图片背景"
    input.value = ""
    return
  }

  if (file.size > 100 * 1024 * 1024) {
    appearanceMsg.value = "文件不能超过 100MB"
    input.value = ""
    return
  }

  bgUploading.value = true
  backgroundFileLabel.value = file.name

  try {
    const ext = file.name.split(".").pop() || "jpg"
    const filename = `bg_image.${ext}`
    const buf = await file.arrayBuffer()
    const data = Array.from(new Uint8Array(buf))
    const previewUrl = URL.createObjectURL(file)

    try {
      const path = await invoke<string>("save_background_file", { data, filename })
      const url = convertFileSrc(path)
      setBackgroundMode("image")
      setBackgroundAutoTheme(true)
      const synced = await syncBackgroundTheme(previewUrl)
      await setBackgroundImage(previewUrl || url, { syncTheme: false })
      appearanceMsg.value = synced ? `背景与主题色已更新 ${synced}` : "图片背景已更新"
    } catch (error) {
      URL.revokeObjectURL(previewUrl)
      throw error
    }

    await persistAppearanceConfig()
  } catch (error) {
    appearanceMsg.value = `上传失败：${error}`
  } finally {
    bgUploading.value = false
    input.value = ""
  }
}

async function clearBackground() {
  try {
    await invoke("clear_background_file")
  } catch (error) {
    console.error("Clear background:", error)
  }

  setBackgroundMode("solid")
  setBackgroundAutoTheme(true)
  setBackgroundThemeColor("")
  backgroundFileLabel.value = "未上传背景"
  await setBackgroundImage("", { syncTheme: false })

  try {
    await persistAppearanceConfig()
  } catch (error) {
    console.error("Persist appearance after clear:", error)
  }
}

async function resyncBackgroundTheme() {
  if (backgroundMode.value !== "image" || !backgroundImageUrl.value) return
  setBackgroundAutoTheme(true)
  const synced = await syncBackgroundTheme(backgroundImageUrl.value)
  appearanceMsg.value = synced ? `已重新匹配主题色 ${synced}` : "未能从当前图片提取主题色"
}

async function saveLlmConfig() {
  configSaving.value = true
  configMsg.value = ""
  try {
    const current = await invoke<AppConfig>("get_config")
    await invoke("save_config", {
      config: {
        ...current,
        llm_provider: llmEnabled.value
          ? {
              provider_type: llmProvider.value,
              api_format: currentProvider.value?.apiFormat || "openai_chat",
              endpoint: llmEndpoint.value,
              model: llmModel.value,
              api_key: llmApiKey.value || undefined,
            }
          : null,
      },
    })
    configMsg.value = "已保存"
    window.setTimeout(() => {
      configMsg.value = ""
    }, 2000)
  } catch (error) {
    configMsg.value = `保存失败：${error}`
  } finally {
    configSaving.value = false
  }
}

async function testWithCurrentForm() {
  testingConnection.value = true
  configMsg.value = ""
  testSuccess.value = false

  try {
    const result = await invoke<{ success: boolean; message: string }>("test_llm_connection_with_config", {
      providerType: llmProvider.value,
      endpoint: llmEndpoint.value,
      model: llmModel.value,
      apiKey: llmApiKey.value || null,
    })
    testSuccess.value = result.success
    configMsg.value = result.message
  } catch (error) {
    testSuccess.value = false
    configMsg.value = String(error)
  } finally {
    testingConnection.value = false
  }
}

async function saveExecutionSettings() {
  executionSaving.value = true
  executionMsg.value = ""

  try {
    const rules = parseRules(permissionRulesInput.value)
    const current = await invoke<AppConfig>("get_config")
    await invoke<PolicyMode>("set_policy_mode", { mode: policyMode.value })
    await chatStore.setAgentMode(selectedAgentMode.value)
    await invoke("save_config", {
      config: {
        ...current,
        policy_mode: policyMode.value,
        agent_mode: selectedAgentMode.value,
        permissions: {
          rules,
        },
      },
    })
    executionMsg.value = "执行设置已保存"
    window.setTimeout(() => {
      executionMsg.value = ""
    }, 2400)
  } catch (error) {
    executionMsg.value = `保存失败：${error}`
  } finally {
    executionSaving.value = false
  }
}
</script>

<template>
  <div class="settings-view">
    <header class="section-head">
      <div>
        <h2 class="hero-title">外观、模型与执行</h2>
        <p class="hero-subtitle">
          这里专注于主题、模型和执行策略。具体工具暴露与 action 兼容层已经拆到左侧的“原子化能力”页面。
        </p>
      </div>
    </header>

    <section class="card">
      <div class="card-header">
        <div>
          <h3 class="card-title">外观</h3>
          <p class="card-subtitle">先确定工作台氛围，再调颜色、透明度和背景源。</p>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">主题预设</label>
        <div class="preset-grid">
          <button
            v-for="[key, val] in presets"
            :key="key"
            :class="['preset-btn', { active: preset === key }]"
            :style="{ '--preset-color': val.accent }"
            @click="applyPreset(key)"
          >
            <span class="preset-swatch" />
            <span class="preset-label">{{ val.label }}</span>
          </button>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">强调色</label>
        <div class="color-row">
          <input
            type="color"
            :value="accent"
            class="color-input"
            @input="applyAccent(($event.target as HTMLInputElement).value)"
          />
          <span class="color-value">{{ accent }}</span>
          <button class="ghost-btn" @click="applyAccent(THEME_PRESETS[preset].accent)">重置</button>
          <button v-if="backgroundMode === 'image'" class="ghost-btn" @click="resyncBackgroundTheme">取背景主色</button>
        </div>
        <p v-if="backgroundMode === 'image'" class="field-note">
          {{ backgroundAutoTheme ? "当前图片会驱动主题色。" : "当前主题色已改为手动控制。" }}
        </p>
      </div>

      <div class="field-group">
        <label class="field-label">透明度</label>
        <div class="slider-row">
          <input
            type="range"
            min="35"
            max="88"
            :value="surfaceOpacity"
            class="opacity-slider"
            @input="setSurfaceOpacity(Number(($event.target as HTMLInputElement).value))"
          />
          <span class="slider-value">{{ surfaceOpacity }}%</span>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">背景模式</label>
        <div class="mode-row">
          <button :class="['mode-btn', { active: backgroundMode === 'solid' }]" @click="setBackgroundMode('solid')">纯色</button>
          <button :class="['mode-btn', { active: backgroundMode === 'image' }]" @click="setBackgroundMode('image')">图片</button>
          <button v-if="backgroundMode !== 'solid'" class="ghost-btn" @click="clearBackground">清除</button>
        </div>
      </div>

      <div v-if="backgroundMode === 'solid'" class="field-group">
        <label class="field-label">内置壁纸</label>
        <div class="wallpaper-grid">
          <button
            v-for="[key, val] in backgrounds"
            :key="key"
            :class="['wallpaper-btn', { active: backgroundPreset === key }]"
            @click="setBackgroundPreset(key)"
          >
            <span class="wallpaper-preview" :style="{ background: val.css }" />
            <span class="wallpaper-label">{{ val.label }}</span>
          </button>
        </div>
      </div>

      <div v-if="backgroundMode !== 'solid'" class="field-group">
        <label class="field-label">选择图片</label>
        <input
          ref="fileInputRef"
          class="hidden-file-input"
          type="file"
          accept="image/*"
          :disabled="bgUploading"
          @change="handleBgFileUpload"
        />
        <div class="file-row">
          <button class="ghost-btn file-btn" :disabled="bgUploading" @click="chooseBackgroundFile">
            {{ bgUploading ? "上传中..." : "选择图片" }}
          </button>
          <span class="file-name">{{ backgroundFileLabel }}</span>
        </div>
      </div>

      <template v-if="backgroundMode !== 'solid'">
        <div v-if="backgroundMode === 'image'" class="field-group">
          <div class="field-inline">
            <div>
              <label class="field-label">背景主色同步主题</label>
              <p class="field-note">从图片里提取主色，统一驱动强调色。</p>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                :checked="backgroundAutoTheme"
                @change="toggleBackgroundAutoTheme(($event.target as HTMLInputElement).checked)"
              />
              <span class="toggle-track" />
            </label>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">缩放</label>
          <div class="slider-row">
            <input
              type="range"
              min="100"
              max="200"
              :value="Math.round(bgScale * 100)"
              class="opacity-slider"
              @input="setBgScale(Number(($event.target as HTMLInputElement).value) / 100)"
            />
            <span class="slider-value">{{ Math.round(bgScale * 100) }}%</span>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">模糊</label>
          <div class="slider-row">
            <input
              type="range"
              min="0"
              max="40"
              :value="bgBlur"
              class="opacity-slider"
              @input="setBgBlur(Number(($event.target as HTMLInputElement).value))"
            />
            <span class="slider-value">{{ bgBlur }}px</span>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">遮罩深度</label>
          <div class="slider-row">
            <input
              type="range"
              min="0"
              max="80"
              :value="bgMaskOpacity"
              class="opacity-slider"
              @input="setBgMaskOpacity(Number(($event.target as HTMLInputElement).value))"
            />
            <span class="slider-value">{{ bgMaskOpacity }}%</span>
          </div>
        </div>
      </template>

      <div class="card-actions">
        <button class="save-btn" :disabled="appearanceSaving" @click="saveAppearance">
          {{ appearanceSaving ? "保存中..." : "保存外观" }}
        </button>
        <span v-if="appearanceMsg" class="msg" :class="{ error: appearanceMsg.startsWith('保存失败') }">
          {{ appearanceMsg }}
        </span>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <div>
          <h3 class="card-title">模型</h3>
          <p class="card-subtitle">连接本地或云端模型。测试连接直接使用当前表单值，不需要先保存。</p>
        </div>
        <label class="toggle-switch">
          <input v-model="llmEnabled" type="checkbox" />
          <span class="toggle-track" />
        </label>
      </div>

      <div v-if="llmEnabled" class="model-fields">
        <div class="field-group">
          <label class="field-label">Provider</label>
          <div class="provider-row">
            <ThemeSelect
              class="form-input provider-select"
              :model-value="llmProvider"
              :options="providerOptions"
              @update:model-value="selectProvider"
            />
            <span v-if="currentProvider?.iconColor" class="provider-dot" :style="{ background: currentProvider.iconColor }" />
            <span v-if="currentProvider" class="provider-id">{{ currentProvider.id }}</span>
            <a v-if="currentProvider?.websiteUrl" :href="currentProvider.websiteUrl" target="_blank" class="provider-link">官网</a>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">Endpoint</label>
          <input
            v-model="llmEndpoint"
            class="form-input"
            type="text"
            :placeholder="currentProvider ? defaultEndpointFor(currentProvider.id) : ''"
          />
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
          <button class="save-btn" :disabled="configSaving" @click="saveLlmConfig">
            {{ configSaving ? "保存中..." : "保存" }}
          </button>
          <button class="ghost-btn" :disabled="testingConnection" @click="testWithCurrentForm">
            {{ testingConnection ? "测试中..." : "测试连接" }}
          </button>
          <span v-if="configMsg" class="msg" :class="{ error: !testSuccess && configMsg !== '已保存' }">
            {{ configMsg }}
          </span>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="card-header">
        <div>
          <h3 class="card-title">执行与权限</h3>
          <p class="card-subtitle">把策略层、代理模式和 allow/deny 规则拆开解释，避免用户不知道工具为什么能跑或不能跑。</p>
        </div>
      </div>

      <div class="status-banner">
        <span class="status-dot" />
        <span>{{ executionSummary }}</span>
      </div>

      <div class="field-group">
        <label class="field-label">策略模式</label>
        <div class="chip-grid">
          <button
            v-for="option in policyModeOptions"
            :key="option.value"
            :class="['choice-card', { active: policyMode === option.value }]"
            @click="policyMode = option.value"
          >
            <span class="choice-title">{{ option.label }}</span>
            <span class="choice-note">{{ option.note }}</span>
          </button>
        </div>
      </div>

      <div class="field-group">
        <label class="field-label">代理模式</label>
        <div class="chip-grid">
          <button
            v-for="option in agentModeOptions"
            :key="option.value"
            :class="['choice-card', { active: selectedAgentMode === option.value }]"
            @click="selectedAgentMode = option.value"
          >
            <span class="choice-title">{{ option.label }}</span>
            <span class="choice-note">{{ option.note }}</span>
          </button>
        </div>
      </div>

      <div class="mini-grid">
        <article class="mini-card">
          <span class="mini-label">当前可见工具</span>
          <strong class="mini-value">{{ visibleToolCount }}</strong>
          <p class="mini-note">会展示给模型的工具数量，受 Agent Mode 影响。</p>
        </article>
        <article class="mini-card">
          <span class="mini-label">Allow 规则</span>
          <strong class="mini-value">{{ allowRuleCount }}</strong>
          <p class="mini-note">
            {{ hasAllowRules ? "已启用 allow 列表，未命中的工具会被拒绝。" : "未启用 allow 列表，默认不是白名单模式。" }}
          </p>
        </article>
        <article class="mini-card">
          <span class="mini-label">Deny 规则</span>
          <strong class="mini-value">{{ denyRuleCount }}</strong>
          <p class="mini-note">deny 永远优先，匹配后直接拒绝执行。</p>
        </article>
      </div>

      <div class="field-group">
        <label class="field-label">权限规则</label>
        <textarea
          v-model="permissionRulesInput"
          class="rules-input"
          rows="7"
          spellcheck="false"
          placeholder="# 一行一个规则&#10;allow: read&#10;allow: glob&#10;deny: bash(rm *)&#10;deny: write(/etc/*)"
        />
        <p class="field-note">
          支持 `allow:` / `deny:` 两种前缀，`#` 开头视为注释。参数模式遵循现有后端规则，例如 `bash(rm *)`、`write(/etc/*)`。
        </p>
        <p v-if="parsedRuleError" class="field-note error-text">{{ parsedRuleError }}</p>
      </div>

      <div class="card-actions">
        <button class="save-btn" :disabled="executionSaving || !!parsedRuleError" @click="saveExecutionSettings">
          {{ executionSaving ? "保存中..." : "保存执行设置" }}
        </button>
        <span v-if="executionMsg" class="msg" :class="{ error: executionMsg.startsWith('保存失败') }">
          {{ executionMsg }}
        </span>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-view {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 32px 40px;
  max-width: 980px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  scrollbar-gutter: stable;
}

.section-head {
  margin-bottom: 2px;
}

.hero-title {
  margin: 0 0 6px;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.hero-subtitle {
  margin: 0;
  max-width: 660px;
  font-size: 13px;
  line-height: 1.65;
  color: var(--text-secondary);
}

.card {
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  border-radius: 22px;
  padding: 22px 24px;
  box-shadow: var(--shadow-surface);
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 18px;
}

.card-title {
  margin: 0 0 4px;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.card-subtitle {
  margin: 0;
  max-width: 560px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.field-group {
  margin-bottom: 16px;
}

.field-group:last-child {
  margin-bottom: 0;
}

.field-label {
  display: block;
  margin-bottom: 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.field-note {
  margin: 8px 0 0;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-tertiary);
}

.error-text {
  color: #fda4af;
}

.field-inline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.status-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  margin-bottom: 16px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  border-radius: 16px;
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.6;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.4);
  flex-shrink: 0;
}

.mini-grid,
.chip-grid,
.tool-grid,
.wallpaper-grid {
  display: grid;
  gap: 10px;
}

.mini-grid {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  margin-bottom: 16px;
}

.chip-grid {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
}

.tool-grid {
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
}

.wallpaper-grid {
  grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
}

.mini-card,
.choice-card,
.tool-card,
.wallpaper-btn {
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--surface-1);
}

.mini-card {
  padding: 14px;
}

.mini-label {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.mini-value {
  display: block;
  font-size: 20px;
  color: var(--text-primary);
}

.mini-note {
  margin: 8px 0 0;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-secondary);
}

.choice-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px;
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
}

.choice-card:hover,
.tool-card:hover,
.wallpaper-btn:hover,
.preset-btn:hover,
.ghost-btn:hover:not(:disabled),
.mode-btn:hover {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: var(--surface-2);
}

.choice-card.active,
.wallpaper-btn.active,
.preset-btn.active,
.mode-btn.active {
  border-color: rgba(var(--accent-rgb), 0.28);
  background: rgba(var(--accent-rgb), 0.08);
  box-shadow: 0 0 0 1px rgba(var(--accent-rgb), 0.08), 0 10px 24px rgba(0, 0, 0, 0.12);
}

.choice-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
}

.choice-note {
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-secondary);
}

.tool-card {
  padding: 16px;
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
}

.tool-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.tool-label {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.tool-name {
  margin-top: 3px;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.tool-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-end;
}

.tool-tag {
  padding: 3px 8px;
  border-radius: 999px;
  font-size: 10px;
  color: var(--text-secondary);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.tool-tag.risk {
  color: var(--text-primary);
}

.tool-tag.ok {
  color: #86efac;
}

.tool-tag.warn {
  color: #fbbf24;
}

.tool-tag.muted {
  color: var(--text-tertiary);
}

.tool-short,
.tool-detail,
.tool-runtime,
.tool-example {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
}

.tool-short {
  color: var(--text-primary);
  margin-bottom: 6px;
}

.tool-detail {
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.tool-runtime {
  color: var(--text-tertiary);
  margin-bottom: 8px;
}

.tool-example {
  color: var(--accent);
  font-family: var(--font-mono);
}

.preset-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.preset-btn,
.mode-btn,
.ghost-btn,
.file-btn,
.save-btn {
  font-family: inherit;
  transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease, transform 0.15s ease;
}

.preset-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: var(--surface-1);
  color: var(--text-primary);
  cursor: pointer;
}

.preset-swatch {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--preset-color);
}

.preset-label {
  font-size: 13px;
}

.color-row,
.slider-row,
.mode-row,
.file-row,
.provider-row,
.card-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.color-input {
  width: 36px;
  height: 36px;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 2px;
  background: none;
  cursor: pointer;
}

.color-input::-webkit-color-swatch-wrapper {
  padding: 0;
}

.color-input::-webkit-color-swatch {
  border: none;
  border-radius: 6px;
}

.color-value,
.slider-value,
.provider-id,
.msg {
  font-family: var(--font-mono);
}

.color-value,
.slider-value {
  font-size: 13px;
  color: var(--text-secondary);
}

.slider-row {
  width: 100%;
}

.slider-value {
  min-width: 48px;
  text-align: right;
}

.opacity-slider {
  flex: 1;
  appearance: none;
  height: 4px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.12);
  outline: none;
}

.opacity-slider::-webkit-slider-thumb {
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: var(--accent);
  border: 2px solid var(--text-primary);
  cursor: pointer;
}

.opacity-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: var(--accent);
  border: 2px solid var(--text-primary);
  cursor: pointer;
}

.mode-btn,
.ghost-btn {
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--surface-1);
  color: var(--text-secondary);
  cursor: pointer;
}

.ghost-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.hidden-file-input {
  display: none;
}

.file-name {
  min-width: 0;
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.toggle-switch input {
  display: none;
}

.toggle-track {
  display: block;
  width: 36px;
  height: 20px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.1);
  position: relative;
  cursor: pointer;
  transition: background 0.2s ease;
}

.toggle-track::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: var(--text-secondary);
  transition: all 0.2s ease;
}

.toggle-switch input:checked + .toggle-track {
  background: var(--accent);
}

.toggle-switch input:checked + .toggle-track::after {
  left: 18px;
  background: var(--text-on-accent);
}

.wallpaper-btn {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px;
  cursor: pointer;
}

.wallpaper-preview {
  width: 100%;
  aspect-ratio: 16 / 10;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.wallpaper-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.provider-row select {
  flex: 1;
}

.provider-dot {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  flex-shrink: 0;
}

.provider-id {
  font-size: 11px;
  color: var(--text-tertiary);
}

.provider-link {
  font-size: 11px;
  color: var(--accent);
  text-decoration: none;
}

.provider-link:hover {
  text-decoration: underline;
}

.form-input,
.rules-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.022);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background 0.15s ease;
  appearance: none;
  color-scheme: dark;
}

.rules-input {
  min-height: 160px;
  resize: vertical;
  font-family: var(--font-mono);
  line-height: 1.6;
}

.form-input:focus,
.rules-input:focus {
  border-color: rgba(var(--accent-rgb), 0.22);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
  background: var(--surface-2);
}

.form-input::placeholder,
.rules-input::placeholder {
  color: var(--text-tertiary);
}

.provider-select {
  background: var(--surface-1);
}

.save-btn {
  padding: 9px 16px;
  border: none;
  border-radius: 12px;
  background: var(--accent);
  color: var(--text-on-accent);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 12px 28px rgba(var(--accent-rgb), 0.24);
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.msg {
  font-size: 12px;
  color: var(--text-secondary);
}

.msg.error {
  color: #fda4af;
}

@media (max-width: 768px) {
  .settings-view {
    padding: 20px 18px 28px;
  }

  .card {
    padding: 18px;
    border-radius: 18px;
  }

  .tool-grid,
  .chip-grid,
  .mini-grid {
    grid-template-columns: 1fr;
  }
}
</style>
