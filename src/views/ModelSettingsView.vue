<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ConfirmDialog from "../components/ConfirmDialog.vue"
import ThemeSelect from "../components/ThemeSelect.vue"

interface LlmConfig {
  provider_type: string
  api_format: string
  endpoint: string
  model: string
  api_key?: string
  max_tokens?: number
}

interface LlmProfile extends LlmConfig {
  id: string
  name: string
}

interface LlmModelOption {
  id: string
  label: string
}

interface AppConfig {
  active_llm_profile_id?: string | null
  compression_llm_profile_id?: string | null
  llm_profiles: LlmProfile[]
  llm_provider: LlmConfig | null
}

const API_FORMAT_OPTIONS = [
  { label: "OpenAI Chat", value: "openai_chat" },
  { label: "OpenAI Responses", value: "openai_responses" },
  { label: "Anthropic", value: "anthropic" },
  { label: "Gemini", value: "gemini" },
  { label: "Ollama", value: "ollama" },
]

const pageEl = ref<HTMLElement | null>(null)
const llmEnabled = ref(false)
const llmProfiles = ref<LlmProfile[]>([])
const activeLlmProfileId = ref<string | null>(null)
const compressionLlmProfileId = ref<string | null>(null)
const editingLlmProfileId = ref("")
const profileDraft = ref<LlmProfile | null>(null)
const fetchedModels = ref<Record<string, LlmModelOption[]>>({})
const fetchingModels = ref(false)
const deleteProfileId = ref<string | null>(null)
const configSaving = ref(false)
const configMsg = ref("")
const testingConnection = ref(false)
const testSuccess = ref(false)

const selectedProfile = computed(() =>
  llmProfiles.value.find((profile) => profile.id === editingLlmProfileId.value) || null
)
const deleteProfileTarget = computed(() =>
  llmProfiles.value.find((profile) => profile.id === deleteProfileId.value) || null
)
const activeProfile = computed(() =>
  llmProfiles.value.find((profile) => profile.id === activeLlmProfileId.value) || null
)
const compressionProfile = computed(() =>
  llmProfiles.value.find((profile) => profile.id === compressionLlmProfileId.value) || null
)
const apiFormatOptions = computed(() => API_FORMAT_OPTIONS)
const compressionProfileOptions = computed(() =>
  llmProfiles.value.map((profile) => ({
    label: `${profile.name || "未命名配置"} · ${profile.provider_type || "custom"} · ${profile.model || "未设置模型"}`,
    value: profile.id,
  }))
)
const selectedFetchedModelOptions = computed(() =>
  ((profileDraft.value && fetchedModels.value[profileDraft.value.id]) || []).map((item) => ({
    label: item.label,
    value: item.id,
  }))
)

function needsKeyFor(id: string) {
  return id !== "ollama"
}

function defaultProfileName(config: Pick<LlmConfig, "provider_type" | "model">) {
  const provider = config.provider_type.trim() || "custom"
  const model = config.model.trim()
  if (!model) return provider
  return `${provider} · ${model}`
}

function cloneProfile(profile: LlmProfile): LlmProfile {
  return {
    ...profile,
    api_key: profile.api_key || "",
  }
}

function createProfile(): LlmProfile {
  return {
    id: `profile-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: "新模型配置",
    provider_type: "",
    api_format: "openai_chat",
    endpoint: "",
    model: "",
    api_key: "",
    max_tokens: undefined,
  }
}

async function preserveScroll(work: () => void | Promise<void>) {
  const top = pageEl.value?.scrollTop ?? 0
  await work()
  await nextTick()
  if (pageEl.value) {
    pageEl.value.scrollTop = top
  }
}

function ensureEditingProfile() {
  if (selectedProfile.value || !llmProfiles.value.length) return
  editingLlmProfileId.value = activeLlmProfileId.value || llmProfiles.value[0].id
}

function syncProfileDraft() {
  profileDraft.value = selectedProfile.value ? cloneProfile(selectedProfile.value) : null
}

function mergedProfiles() {
  return llmProfiles.value.map((profile) => {
    if (profileDraft.value && profile.id === profileDraft.value.id) {
      return cloneProfile(profileDraft.value)
    }
    return cloneProfile(profile)
  })
}

function addProfile() {
  preserveScroll(() => {
    const profile = createProfile()
    llmProfiles.value = [...llmProfiles.value, profile]
    editingLlmProfileId.value = profile.id
    if (!activeLlmProfileId.value && llmEnabled.value) {
      activeLlmProfileId.value = profile.id
    }
  })
}

function duplicateProfile() {
  const source = profileDraft.value || selectedProfile.value
  if (!source) return
  preserveScroll(() => {
    const copy = {
      ...cloneProfile(source),
      id: `profile-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: `${source.name || "新模型配置"} 副本`,
    }
    llmProfiles.value = [...llmProfiles.value, copy]
    editingLlmProfileId.value = copy.id
  })
}

function removeProfile(id: string) {
  preserveScroll(() => {
    llmProfiles.value = llmProfiles.value.filter((profile) => profile.id !== id)
    deleteProfileId.value = null
    if (activeLlmProfileId.value === id) {
      activeLlmProfileId.value = null
      llmEnabled.value = false
    }
    if (editingLlmProfileId.value === id) {
      editingLlmProfileId.value = llmProfiles.value[0]?.id || ""
    }
  })
}

function confirmDeleteProfile() {
  if (!deleteProfileId.value) return
  removeProfile(deleteProfileId.value)
}

function setActiveProfile(id: string) {
  preserveScroll(() => {
    llmProfiles.value = mergedProfiles()
    activeLlmProfileId.value = id
    llmEnabled.value = true
  })
}

function toggleLlmEnabled(enabled: boolean) {
  preserveScroll(() => {
    llmEnabled.value = enabled
    if (!enabled) {
      activeLlmProfileId.value = null
      return
    }
    if (!llmProfiles.value.length) {
      const profile = createProfile()
      llmProfiles.value = [profile]
      editingLlmProfileId.value = profile.id
    }
    activeLlmProfileId.value = activeLlmProfileId.value || editingLlmProfileId.value || llmProfiles.value[0]?.id || null
  })
}

function updateSelectedProfileApiFormat(value: string | number) {
  if (!profileDraft.value) return
  profileDraft.value.api_format = String(value)
}

async function fetchModelsForSelectedProfile() {
  if (!profileDraft.value) return
  fetchingModels.value = true
  configMsg.value = ""
  try {
    const items = await invoke<LlmModelOption[]>("fetch_llm_models", {
      providerType: profileDraft.value.provider_type,
      apiFormat: profileDraft.value.api_format,
      endpoint: profileDraft.value.endpoint,
      apiKey: profileDraft.value.api_key || null,
    })
    fetchedModels.value = {
      ...fetchedModels.value,
      [profileDraft.value.id]: items,
    }
    if (!profileDraft.value.model && items.length) {
      profileDraft.value.model = items[0].id
    }
    configMsg.value = `已获取 ${items.length} 个模型`
    testSuccess.value = true
  } catch (error) {
    testSuccess.value = false
    configMsg.value = String(error)
  } finally {
    fetchingModels.value = false
  }
}

function normalizedProfiles() {
  return mergedProfiles().map((profile) => ({
    ...profile,
    name: profile.name.trim() || defaultProfileName(profile),
    provider_type: profile.provider_type.trim() || "custom",
    api_format: profile.api_format.trim() || "openai_chat",
    endpoint: profile.endpoint.trim(),
    model: profile.model.trim(),
    api_key: profile.api_key?.trim() || undefined,
    max_tokens: profile.max_tokens,
  }))
}

async function loadSettings() {
  const cfg = await invoke<AppConfig>("get_config")

  llmProfiles.value = (cfg.llm_profiles || []).map((profile) => ({
    ...cloneProfile(profile),
  }))
  llmEnabled.value = !!cfg.active_llm_profile_id
  activeLlmProfileId.value = cfg.active_llm_profile_id || null
  compressionLlmProfileId.value = cfg.compression_llm_profile_id || cfg.active_llm_profile_id || null
  editingLlmProfileId.value = cfg.active_llm_profile_id || llmProfiles.value[0]?.id || ""
  ensureEditingProfile()
  syncProfileDraft()
}

async function saveLlmConfig() {
  configSaving.value = true
  configMsg.value = ""
  try {
    const profiles = normalizedProfiles()
    if (profiles.some((profile) => !profile.model)) {
      throw new Error("模型名称不能为空")
    }
    if (profiles.some((profile) => !profile.provider_type)) {
      throw new Error("Provider 不能为空")
    }

    const current = await invoke<AppConfig>("get_config")
    await invoke("save_config", {
      config: {
        ...current,
        active_llm_profile_id: llmEnabled.value
          ? (activeLlmProfileId.value || editingLlmProfileId.value || profiles[0]?.id || null)
          : null,
        compression_llm_profile_id: compressionLlmProfileId.value
          || activeLlmProfileId.value
          || editingLlmProfileId.value
          || profiles[0]?.id
          || null,
        llm_profiles: profiles,
        llm_provider: null,
      },
    })
    llmProfiles.value = profiles
    syncProfileDraft()
    if (llmEnabled.value && !activeLlmProfileId.value && profiles.length) {
      activeLlmProfileId.value = profiles[0].id
    }
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
  if (!profileDraft.value) {
    configMsg.value = "请先选择一条模型配置"
    testSuccess.value = false
    return
  }

  testingConnection.value = true
  configMsg.value = ""
  testSuccess.value = false

  try {
    const result = await invoke<{ success: boolean; message: string }>("test_llm_connection_with_config", {
      providerType: profileDraft.value.provider_type,
      apiFormat: profileDraft.value.api_format,
      endpoint: profileDraft.value.endpoint,
      model: profileDraft.value.model,
      apiKey: profileDraft.value.api_key || null,
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

onMounted(async () => {
  try {
    await loadSettings()
  } catch (error) {
    console.error("Load model settings:", error)
  }
})

watch(
  () => editingLlmProfileId.value,
  () => {
    syncProfileDraft()
  }
)
</script>

<template>
  <div ref="pageEl" class="model-settings-view">
    <header class="section-head">
      <div>
        <h2 class="hero-title">模型设置</h2>
        <p class="hero-subtitle">管理多套模型配置，选择当前对话使用的主模型。</p>
      </div>
    </header>

    <section class="card">
      <div class="card-header">
        <div>
          <h3 class="card-title">模型档案</h3>
          <p class="card-subtitle">支持多配置切换、拉取模型列表、连接测试与当前模型指定。</p>
        </div>
        <label class="toggle-switch">
          <input :checked="llmEnabled" type="checkbox" @change="toggleLlmEnabled(($event.target as HTMLInputElement).checked)" />
          <span class="toggle-track" />
        </label>
      </div>

      <div class="model-summary">
        <span class="status-chip" :class="{ active: llmEnabled && activeProfile }">
          {{ llmEnabled && activeProfile ? `当前模型：${activeProfile.name}` : "当前未启用模型" }}
        </span>
        <span class="status-chip subtle">{{ llmProfiles.length }} 条配置</span>
        <span class="status-chip subtle">
          {{ compressionProfile ? `压缩模型：${compressionProfile.name}` : "压缩模型跟随当前模型" }}
        </span>
      </div>

      <div class="model-profile-actions">
        <button class="ghost-btn" type="button" @click="addProfile()">新增配置</button>
        <button class="ghost-btn" type="button" :disabled="!selectedProfile" @click="duplicateProfile">复制当前配置</button>
        <button class="ghost-btn danger-ghost" type="button" :disabled="!selectedProfile" @click="selectedProfile && (deleteProfileId = selectedProfile.id)">删除当前配置</button>
      </div>

      <div v-if="llmProfiles.length" class="model-profile-list">
        <button
          v-for="profile in llmProfiles"
          :key="profile.id"
          type="button"
          :class="['model-profile-card', { active: profile.id === editingLlmProfileId }]"
          @click="editingLlmProfileId = profile.id"
        >
          <div class="model-profile-main">
            <div class="model-profile-name">{{ profile.name || "未命名配置" }}</div>
            <div class="model-profile-meta">
              {{ profile.provider_type }} · {{ profile.model || "未设置模型" }}
            </div>
          </div>
          <span v-if="profile.id === activeLlmProfileId && llmEnabled" class="model-profile-badge">当前</span>
        </button>
      </div>

      <div v-else class="empty-state">
        <p>暂无模型配置</p>
      </div>

      <template v-if="profileDraft">
        <div class="field-group">
          <label class="field-label">配置名称</label>
          <input v-model="profileDraft.name" class="form-input" type="text" placeholder="例如：主对话 / 编码 / 推理" />
        </div>

        <div class="field-group">
          <label class="field-label">Provider</label>
          <input v-model="profileDraft.provider_type" class="form-input" type="text" placeholder="例如：openai / deepseek / anthropic / openrouter" />
        </div>

        <div class="field-group">
          <label class="field-label">API Format</label>
          <ThemeSelect
            class="form-input provider-select"
            :model-value="profileDraft.api_format"
            :options="apiFormatOptions"
            @update:model-value="updateSelectedProfileApiFormat"
          />
        </div>

        <div class="field-group">
          <label class="field-label">Endpoint</label>
          <input v-model="profileDraft.endpoint" class="form-input" type="text" placeholder="例如：https://api.openai.com/v1" />
        </div>

        <div class="field-group">
          <label class="field-label">Model</label>
          <div class="provider-row">
            <input v-model="profileDraft.model" class="form-input" type="text" placeholder="手动填写模型名，或点右侧获取模型" />
            <button class="ghost-btn" type="button" :disabled="fetchingModels" @click="fetchModelsForSelectedProfile">
              {{ fetchingModels ? "获取中..." : "获取模型" }}
            </button>
          </div>
          <ThemeSelect
            v-if="selectedFetchedModelOptions.length"
            class="form-input provider-select"
            :model-value="profileDraft.model"
            :options="selectedFetchedModelOptions"
            @update:model-value="(value) => profileDraft && (profileDraft.model = String(value))"
          />
        </div>

        <div v-if="llmProfiles.length" class="field-group">
          <label class="field-label">压缩模型</label>
          <ThemeSelect
            class="form-input provider-select"
            :model-value="compressionLlmProfileId || activeLlmProfileId || profileDraft.id"
            :options="compressionProfileOptions"
            @update:model-value="(value) => compressionLlmProfileId = String(value)"
          />
        </div>

        <div v-if="needsKeyFor(profileDraft.api_format)" class="field-group">
          <label class="field-label">API Key</label>
          <input v-model="profileDraft.api_key" class="form-input" type="password" placeholder="sk-..." />
        </div>

        <div class="card-actions">
          <button class="save-btn" type="button" :disabled="configSaving" @click="saveLlmConfig">
            {{ configSaving ? "保存中..." : "保存模型配置" }}
          </button>
          <button class="ghost-btn" type="button" :disabled="testingConnection" @click="testWithCurrentForm">
            {{ testingConnection ? "测试中..." : "测试当前配置" }}
          </button>
          <button
            class="ghost-btn"
            type="button"
            :disabled="llmEnabled && activeLlmProfileId === profileDraft.id"
            @click="setActiveProfile(profileDraft.id)"
          >
            设为当前模型
          </button>
          <span v-if="configMsg" class="msg" :class="{ error: !testSuccess && configMsg !== '已保存' }">
            {{ configMsg }}
          </span>
        </div>
      </template>
    </section>

    <ConfirmDialog
      :open="!!deleteProfileId"
      title="删除模型配置"
      :message="deleteProfileTarget ? `确定删除「${deleteProfileTarget.name || '未命名配置'}」吗？删除后该模型配置将从本地移除，无法恢复。` : '删除后该模型配置将从本地移除，无法恢复。'"
      :loading="configSaving"
      @cancel="deleteProfileId = null"
      @confirm="confirmDeleteProfile"
    />
  </div>
</template>

<style scoped>
.model-settings-view {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 32px 40px;
  max-width: 980px;
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 18px;
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
  background: var(--panel-bg);
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

.model-summary,
.model-profile-actions,
.provider-row,
.card-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.model-summary,
.model-profile-actions {
  margin-bottom: 14px;
}

.model-profile-list {
  display: grid;
  gap: 10px;
  margin-bottom: 18px;
}

.model-profile-card {
  width: 100%;
  min-width: 0;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--panel-bg);
  color: var(--text-primary);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.model-profile-card:hover,
.model-profile-card.active {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.08);
}

.model-profile-main {
  flex: 1 1 auto;
  min-width: 0;
}

.model-profile-name {
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-profile-meta {
  margin-top: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-profile-badge,
.status-chip {
  display: inline-flex;
  align-items: center;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--panel-bg);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
  max-width: 100%;
}

.status-chip.active,
.model-profile-badge {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--text-primary);
}

.status-chip.subtle {
  color: var(--text-tertiary);
}

.form-input {
  width: 100%;
  padding: 12px 14px;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--control-bg);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
}

.form-input:focus {
  border-color: rgba(var(--accent-rgb), 0.46);
  box-shadow: var(--focus-ring);
}

.form-input::placeholder {
  color: var(--text-tertiary);
}

.provider-select {
  background: var(--control-bg);
}

.mode-btn,
.ghost-btn,
.save-btn {
  font-family: inherit;
  transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease, transform 0.15s ease;
}

.ghost-btn {
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--panel-bg);
  color: var(--text-secondary);
  cursor: pointer;
}

.ghost-btn:hover:not(:disabled) {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: var(--control-bg);
}

.ghost-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.save-btn {
  padding: 9px 16px;
  border: none;
  border-radius: 12px;
  background: var(--accent);
  color: var(--text-on-accent);
  cursor: pointer;
}

.save-btn:disabled {
  opacity: 0.55;
  cursor: default;
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

.msg {
  flex: 1 1 240px;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  word-break: break-word;
}

.msg.error {
  color: #fca5a5;
}

.empty-state {
  padding: 16px 0;
  color: var(--text-secondary);
}

@media (max-width: 960px) {
  .model-settings-view {
    padding: 20px 18px 32px;
  }

  .card-header {
    flex-direction: column;
  }
}
</style>
