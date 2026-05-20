<script setup lang="ts">
import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ConfirmDialog from "../components/ConfirmDialog.vue"
import ThemeSelect from "../components/ThemeSelect.vue"
import type { CapabilityInfo } from "../lib/tools"
import {
  MEMORY_SCOPE_FOLDERS,
  MEMORY_SCOPE_FOLDER_LABELS,
  defaultMemoryScopeLabel,
  summarizeMemoryScope,
  type AgentProfile,
  type MemoryScopeFolder,
  type MemoryScopeMode,
  useAgentStore,
} from "../stores/agents"

const store = useAgentStore()
const capabilities = ref<CapabilityInfo[]>([])
const capabilityMsg = ref("")
const deleteAgentId = ref<string | null>(null)
const recallLimitOptions = [1, 2, 3, 4, 5, 6, 7, 8]
const recallLimitSelectOptions = recallLimitOptions.map((limit) => ({
  label: `${limit} 条`,
  value: limit,
}))
const delegatePriorityOptions = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100].map((value) => ({
  label: `${value}`,
  value,
}))
const deleteAgentTarget = computed(() =>
  store.agents.find((agent) => agent.id === deleteAgentId.value) || null
)

const toolCapabilities = computed(() =>
  capabilities.value.filter((capability) => capability.enabled && capability.executable)
)

function defaultScopeLabel(mode: MemoryScopeMode, folders: readonly MemoryScopeFolder[]) {
  return defaultMemoryScopeLabel(mode, folders)
}

function syncScopeLabel(
  agent: AgentProfile,
  previousLabel: string,
  previousMode: MemoryScopeMode,
  previousFolders: readonly MemoryScopeFolder[]
) {
  const previousDefault = defaultScopeLabel(previousMode, previousFolders)
  const nextDefault = defaultScopeLabel(agent.memoryScope.mode, agent.memoryScope.folders)
  if (!agent.memoryScope.label.trim() || previousLabel === previousDefault) {
    agent.memoryScope.label = nextDefault
  }
}

function setScopeMode(agent: AgentProfile, mode: MemoryScopeMode) {
  const previousLabel = agent.memoryScope.label
  const previousMode = agent.memoryScope.mode
  const previousFolders = [...agent.memoryScope.folders]

  agent.memoryScope.mode = mode
  if (mode === "none") {
    agent.memoryScope.folders = []
  } else if (mode === "all") {
    agent.memoryScope.folders = [...MEMORY_SCOPE_FOLDERS]
  } else if (!agent.memoryScope.folders.length) {
    agent.memoryScope.folders = ["notes"]
  }

  syncScopeLabel(agent, previousLabel, previousMode, previousFolders)
}

function toggleFolder(agent: AgentProfile, folder: MemoryScopeFolder) {
  const previousLabel = agent.memoryScope.label
  const previousMode = agent.memoryScope.mode
  const previousFolders = [...agent.memoryScope.folders]

  if (agent.memoryScope.mode !== "focused") {
    agent.memoryScope.mode = "focused"
  }

  const next = new Set(agent.memoryScope.folders)
  if (next.has(folder)) {
    next.delete(folder)
  } else {
    next.add(folder)
  }
  agent.memoryScope.folders = MEMORY_SCOPE_FOLDERS.filter((item) => next.has(item))

  syncScopeLabel(agent, previousLabel, previousMode, previousFolders)
}

function toggleTool(agent: AgentProfile, name: string) {
  const next = new Set(agent.tools)
  if (next.has(name)) {
    next.delete(name)
  } else {
    next.add(name)
  }
  agent.tools = toolCapabilities.value
    .map((capability) => capability.name)
    .filter((capability) => next.has(capability))
}

function toolTitle(name: string) {
  return capabilities.value.find((capability) => capability.name === name)?.title || name
}

function tagText(agent: AgentProfile) {
  return agent.delegateTags.join(", ")
}

function updateDelegateTags(agent: AgentProfile, value: string) {
  agent.delegateTags = value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
}

async function loadCapabilities() {
  capabilityMsg.value = ""
  try {
    capabilities.value = await invoke<CapabilityInfo[]>("get_capabilities")
  } catch (error) {
    console.error("load capabilities:", error)
    capabilityMsg.value = String(error)
  }
}

async function refreshAll() {
  await Promise.all([store.refresh(), loadCapabilities()])
}

function savingLabel(agent: AgentProfile) {
  const key = agent.persistedId || agent.id
  return store.saving[key] ? "保存中" : "自动保存"
}

async function confirmDeleteAgent() {
  if (!deleteAgentId.value) return
  const id = deleteAgentId.value
  deleteAgentId.value = null
  await store.removeAgent(id)
}

onMounted(async () => {
  await Promise.all([store.init(), loadCapabilities()])
})
</script>

<template>
  <div class="agents-view">
    <header class="page-head">
      <div>
        <h2 class="page-title">Agent 管理</h2>
        <p class="page-subtitle">
          管理 Agent 的角色、记忆范围和能力边界。
        </p>
      </div>
      <div class="head-actions">
        <button type="button" class="small-btn" :disabled="store.loading" @click="refreshAll">
          {{ store.loading ? "同步中" : "刷新目录" }}
        </button>
        <button type="button" class="primary-btn" @click="store.addAgent">新增 Agent</button>
      </div>
    </header>

    <section class="overview-grid">
      <article class="overview-card">
        <span class="overview-label">当前目录</span>
        <strong class="overview-value">{{ store.agents.length }} 个 Agent</strong>
        <p class="overview-note">支持主 Agent、专用 Agent 和长期角色配置。</p>
      </article>
      <article class="overview-card">
        <span class="overview-label">注册路径</span>
        <strong class="overview-value">前端直改 + AI 自注册</strong>
        <p class="overview-note">支持界面编辑，也支持模型注册和更新。</p>
      </article>
      <article class="overview-card">
        <span class="overview-label">工具治理</span>
        <strong class="overview-value">{{ toolCapabilities.length }} 个可绑定能力</strong>
        <p class="overview-note">按 Agent 控制能力暴露，减少无关工具进入上下文。</p>
      </article>
    </section>

    <div v-if="store.syncError || capabilityMsg" class="sync-msg">
      {{ store.syncError || capabilityMsg }}
    </div>

    <section class="agent-list">
      <article v-for="agent in store.agents" :key="agent.persistedId || agent.id" class="agent-card">
        <div class="agent-top">
          <div class="agent-head-main">
            <div class="agent-name-row">
              <h3 class="agent-name">{{ agent.name }}</h3>
              <span v-if="agent.primary" class="agent-badge">Primary</span>
              <span v-if="agent.id === store.activeAgentId" class="agent-badge active">Active</span>
              <span class="agent-badge subtle">{{ savingLabel(agent) }}</span>
            </div>
            <p class="agent-role-preview">{{ agent.role }}</p>
          </div>
          <div class="agent-actions">
            <button type="button" class="small-btn" @click="store.setActiveAgent(agent.id)">切换</button>
            <button type="button" class="small-btn" @click="store.duplicateAgent(agent.id)">复制</button>
            <button
              class="small-btn danger"
              type="button"
              :disabled="!agent.deletable"
              @click="deleteAgentId = agent.id"
            >
              删除
            </button>
          </div>
        </div>

        <div class="two-col">
          <label class="field">
            <span>名称</span>
            <input v-model="agent.name" class="field-input" type="text" />
          </label>
          <label class="field">
            <span>角色</span>
            <input v-model="agent.role" class="field-input" type="text" />
          </label>
        </div>

        <label class="field">
          <span>人格</span>
          <textarea v-model="agent.persona" rows="4" class="field-input multiline" />
        </label>

        <section class="scope-panel">
          <div class="scope-head">
            <div>
              <div class="scope-title">记忆 Scope</div>
              <div class="scope-summary">{{ summarizeMemoryScope(agent.memoryScope) }}</div>
            </div>
            <div class="scope-mode-group">
              <button
                v-for="mode in ['none', 'focused', 'all']"
                :key="mode"
                class="scope-mode-chip"
                type="button"
                :class="{ active: agent.memoryScope.mode === mode }"
                @click="setScopeMode(agent, mode as MemoryScopeMode)"
              >
                {{ mode === "none" ? "禁用" : mode === "all" ? "全量" : "定向" }}
              </button>
            </div>
          </div>

          <div class="scope-grid">
            <label class="field">
              <span>Scope 名称</span>
              <input
                v-model="agent.memoryScope.label"
                class="field-input"
                type="text"
                placeholder="比如：产品研究 / 长期偏好 / 项目上下文"
              />
            </label>

            <label class="field">
              <span>召回上限</span>
              <ThemeSelect
                class="field-input"
                :model-value="agent.memoryScope.recallLimit"
                :options="recallLimitSelectOptions"
                @update:model-value="(value) => agent.memoryScope.recallLimit = Number(value)"
              />
            </label>
          </div>

          <div class="field">
            <span>知识目录</span>
            <div class="folder-row">
              <button
                v-for="folder in MEMORY_SCOPE_FOLDERS"
                :key="folder"
                class="folder-chip"
                type="button"
                :class="{
                  active: agent.memoryScope.folders.includes(folder),
                  disabled: agent.memoryScope.mode !== 'focused',
                }"
                @click="toggleFolder(agent, folder)"
              >
                {{ MEMORY_SCOPE_FOLDER_LABELS[folder] }}
                <span class="folder-code">{{ folder }}</span>
              </button>
            </div>
          </div>

          <label class="field">
            <span>记忆策略说明</span>
            <textarea
              v-model="agent.memoryScope.notes"
              rows="3"
              class="field-input multiline"
              placeholder="比如：优先召回产品决策、用户偏好和近期复盘，不要把零碎临时信息都塞进上下文。"
            />
          </label>
        </section>

        <section class="tool-panel">
          <div class="tool-head">
            <div>
              <div class="scope-title">工具范围</div>
              <div class="scope-summary">
                {{ agent.tools.length ? `${agent.tools.length} 个能力` : "未绑定能力，将只剩系统工具" }}
              </div>
            </div>
            <div class="tool-selected">
              <span v-for="tool in agent.tools" :key="tool" class="tool-chip selected">
                {{ toolTitle(tool) }}
              </span>
            </div>
          </div>

          <div class="tool-grid">
            <button
              v-for="capability in toolCapabilities"
              :key="capability.name"
              class="tool-chip"
              type="button"
              :class="{ selected: agent.tools.includes(capability.name) }"
              @click="toggleTool(agent, capability.name)"
            >
              <span class="tool-chip-title">{{ capability.title }}</span>
              <span class="tool-chip-code">{{ capability.name }}</span>
            </button>
          </div>
        </section>

        <section class="tool-panel delegate-panel">
          <div class="tool-head">
            <div>
              <div class="scope-title">委派推荐</div>
              <div class="scope-summary">
                {{
                  agent.delegatable
                    ? `允许委派 · 优先级 ${agent.delegatePriority}`
                    : "当前不暴露给 agent_list"
                }}
              </div>
            </div>
            <div class="tool-selected">
              <span v-for="tag in agent.delegateTags" :key="tag" class="tool-chip selected">
                {{ tag }}
              </span>
            </div>
          </div>

          <div class="scope-grid">
            <label class="field">
              <span>推荐标签</span>
              <input
                class="field-input"
                :value="tagText(agent)"
                type="text"
                placeholder="例如：代码, 修复, 构建, 测试"
                @input="updateDelegateTags(agent, ($event.target as HTMLInputElement).value)"
              />
            </label>

            <label class="field">
              <span>推荐说明</span>
              <input
                v-model="agent.delegateNote"
                class="field-input"
                type="text"
                placeholder="例如：适合前端修复、构建失败排查和代码落地"
              />
            </label>
          </div>

          <div class="scope-grid">
            <label class="field">
              <span>允许自动委派</span>
              <button
                class="toggle-btn"
                type="button"
                :class="{ active: agent.delegatable }"
                @click="agent.delegatable = !agent.delegatable"
              >
                {{ agent.delegatable ? "已允许" : "未允许" }}
              </button>
            </label>

            <label class="field">
              <span>委派优先级</span>
              <ThemeSelect
                class="field-input"
                :model-value="agent.delegatePriority"
                :options="delegatePriorityOptions"
                @update:model-value="(value) => agent.delegatePriority = Number(value)"
              />
            </label>
          </div>
        </section>

        <div class="agent-id-row">
          <span class="agent-id">{{ agent.id }}</span>
        </div>
      </article>
    </section>

    <ConfirmDialog
      :open="!!deleteAgentId"
      title="删除 Agent"
      :message="deleteAgentTarget ? `确定删除「${deleteAgentTarget.name}」吗？删除后将移除这条 Agent 配置。` : '删除后将移除这条 Agent 配置。'"
      @cancel="deleteAgentId = null"
      @confirm="confirmDeleteAgent"
    />
  </div>
</template>

<style scoped>
.agents-view {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 32px 40px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.page-head,
.agent-top,
.head-actions,
.agent-actions,
.scope-head,
.scope-mode-group,
.two-col,
.agent-name-row,
.folder-row,
.tool-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.page-head,
.agent-top,
.scope-head,
.tool-head {
  justify-content: space-between;
}

.page-title,
.agent-name,
.scope-title {
  margin: 0;
  color: var(--text-primary);
}

.page-title {
  font-size: 22px;
}

.page-subtitle,
.overview-note,
.agent-role-preview,
.scope-summary {
  margin: 6px 0 0;
  color: var(--text-secondary);
  line-height: 1.6;
  font-size: 13px;
}

.head-actions {
  align-items: center;
}

.overview-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}

.overview-card,
.agent-card,
.scope-panel,
.tool-panel {
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  background: rgba(var(--surface-rgb), 0.68);
  border-radius: 22px;
}

.overview-card {
  padding: 18px;
}

.overview-label {
  display: block;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-tertiary);
}

.overview-value {
  display: block;
  margin-top: 8px;
  font-size: 18px;
  color: var(--text-primary);
}

.sync-msg {
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid rgba(255, 122, 122, 0.22);
  background: rgba(255, 122, 122, 0.08);
  color: var(--text-primary);
  font-size: 13px;
}

.agent-list {
  display: grid;
  gap: 16px;
}

.agent-card {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.agent-head-main {
  min-width: 0;
}

.agent-name-row {
  flex-wrap: wrap;
  align-items: center;
}

.agent-role-preview {
  margin-top: 8px;
}

.head-pill,
.agent-badge,
.tool-chip,
.small-btn,
.primary-btn,
.scope-mode-chip,
.folder-chip {
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
}

.agent-badge {
  padding: 6px 10px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.agent-badge.active,
.tool-chip.selected,
.scope-mode-chip.active,
.folder-chip.active {
  background: rgba(var(--accent-rgb), 0.18);
  border-color: rgba(var(--accent-rgb), 0.32);
}

.agent-badge.subtle {
  background: rgba(var(--surface-rgb), 0.62);
  border-color: rgba(var(--border-rgb), 0.8);
}

.primary-btn,
.small-btn,
.scope-mode-chip,
.folder-chip,
.tool-chip {
  cursor: pointer;
}

.primary-btn {
  padding: 10px 14px;
  font-size: 12px;
}

.small-btn {
  padding: 8px 11px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.small-btn.danger:disabled {
  opacity: 0.45;
  cursor: default;
}

.two-col,
.scope-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field span {
  font-size: 12px;
  color: var(--text-secondary);
}

.field-input {
  width: 100%;
  border-radius: 16px;
  border: 1px solid rgba(var(--border-rgb), 0.9);
  background: rgba(var(--surface-rgb), 0.78);
  color: var(--text-primary);
  padding: 12px 14px;
  outline: none;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}

.field-input:focus {
  border-color: rgba(var(--accent-rgb), 0.46);
  box-shadow: 0 0 0 1px rgba(var(--accent-rgb), 0.16);
}

.toggle-btn {
  min-height: 44px;
  border-radius: 16px;
  border: 1px solid rgba(var(--border-rgb), 0.9);
  background: rgba(var(--surface-rgb), 0.78);
  color: var(--text-secondary);
  padding: 0 14px;
  text-align: left;
  cursor: pointer;
  transition: border-color 120ms ease, box-shadow 120ms ease, color 120ms ease;
}

.toggle-btn.active {
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.32);
  box-shadow: 0 0 0 1px rgba(var(--accent-rgb), 0.12);
  background: rgba(var(--accent-rgb), 0.12);
}

.multiline {
  resize: vertical;
  min-height: 96px;
}

.scope-panel,
.tool-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.folder-row,
.tool-grid,
.tool-selected {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.folder-chip {
  padding: 8px 12px;
  font-size: 12px;
}

.folder-chip.disabled {
  opacity: 0.52;
}

.folder-code,
.tool-chip-code,
.agent-id {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-tertiary);
}

.tool-grid {
  gap: 12px;
}

.tool-chip {
  min-width: 170px;
  text-align: left;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tool-chip-title {
  font-size: 12px;
}

.tool-selected {
  justify-content: flex-end;
}

.agent-id-row {
  display: flex;
  justify-content: flex-end;
}

@media (max-width: 960px) {
  .agents-view {
    padding: 20px 18px 32px;
  }

  .overview-grid,
  .two-col,
  .scope-grid {
    grid-template-columns: 1fr;
  }

  .page-head,
  .agent-top,
  .scope-head,
  .tool-head {
    flex-direction: column;
  }

  .tool-selected {
    justify-content: flex-start;
  }
}
</style>
