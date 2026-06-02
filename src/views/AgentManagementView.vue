<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue"
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
const selectedAgentId = ref<string>("")

const recallLimitOptions = [1, 2, 3, 4, 5, 6, 7, 8].map((limit) => ({
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

const selectedAgent = computed(() => {
  if (!store.agents.length) return null
  return store.agents.find((agent) => agent.id === selectedAgentId.value) || store.agents[0]
})

const primaryAgent = computed(() => store.agents.find((agent) => agent.primary) || null)

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

function updateRecallLimit(agent: AgentProfile | null, value: string | number) {
  if (!agent) return
  agent.memoryScope.recallLimit = Number(value)
}

function updateDelegatePriority(agent: AgentProfile | null, value: string | number) {
  if (!agent) return
  agent.delegatePriority = Number(value)
}

function savingLabel(agent: AgentProfile) {
  const key = agent.persistedId || agent.id
  return store.saving[key] ? "保存中" : "自动保存"
}

function selectAgent(id: string) {
  selectedAgentId.value = id
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

async function addAgent() {
  await store.addAgent()
  if (store.activeAgentId) {
    selectedAgentId.value = store.activeAgentId
  }
}

async function duplicateSelected() {
  if (!selectedAgent.value) return
  await store.duplicateAgent(selectedAgent.value.id)
  if (store.activeAgentId) {
    selectedAgentId.value = store.activeAgentId
  }
}

async function confirmDeleteAgent() {
  if (!deleteAgentId.value) return
  const deletingId = deleteAgentId.value
  deleteAgentId.value = null
  await store.removeAgent(deletingId)
  if (selectedAgentId.value === deletingId) {
    selectedAgentId.value = store.agents[0]?.id || ""
  }
}

watch(
  () => store.agents.map((agent) => agent.id),
  (ids) => {
    if (!ids.length) {
      selectedAgentId.value = ""
      return
    }
    if (!selectedAgentId.value || !ids.includes(selectedAgentId.value)) {
      selectedAgentId.value = store.activeAgentId && ids.includes(store.activeAgentId)
        ? store.activeAgentId
        : ids[0]
    }
  },
  { immediate: true }
)

onMounted(async () => {
  await Promise.all([store.init(), loadCapabilities()])
})
</script>

<template>
  <div class="agents-page">
    <header class="page-head">
      <div>
        <h2 class="page-title">Agent 管理</h2>
        <p class="page-subtitle">把目录、编辑和状态拆开，减少堆叠和视觉噪音。</p>
      </div>
      <div class="page-actions">
        <button type="button" class="ghost-btn" :disabled="store.loading" @click="refreshAll">
          {{ store.loading ? "同步中" : "刷新目录" }}
        </button>
        <button type="button" class="primary-btn" @click="addAgent">新增 Agent</button>
      </div>
    </header>

    <div class="summary-strip">
      <div class="summary-card">
        <span class="summary-label">总数</span>
        <strong class="summary-value">{{ store.agents.length }}</strong>
      </div>
      <div class="summary-card">
        <span class="summary-label">主 Agent</span>
        <strong class="summary-value">{{ primaryAgent?.name || "未设置" }}</strong>
      </div>
      <div class="summary-card">
        <span class="summary-label">可绑定能力</span>
        <strong class="summary-value">{{ toolCapabilities.length }} 个</strong>
      </div>
    </div>

    <div v-if="store.syncError || capabilityMsg" class="sync-msg">
      {{ store.syncError || capabilityMsg }}
    </div>

    <section class="layout-shell">
      <aside class="agent-sidebar">
        <div class="sidebar-head">
          <div>
            <div class="sidebar-title">Agent 列表</div>
            <div class="sidebar-note">先选中，再编辑。</div>
          </div>
        </div>

        <div class="sidebar-list">
          <button
            v-for="agent in store.agents"
            :key="agent.persistedId || agent.id"
            type="button"
            class="agent-row"
            :class="{ active: selectedAgent?.id === agent.id }"
            @click="selectAgent(agent.id)"
          >
            <div class="agent-row-top">
              <span class="agent-row-name">{{ agent.name }}</span>
              <span v-if="agent.primary" class="row-pill">Primary</span>
              <span v-else-if="agent.id === store.activeAgentId" class="row-pill active">Active</span>
            </div>
            <div class="agent-row-role">{{ agent.role }}</div>
            <div class="agent-row-meta">
              <span>{{ summarizeMemoryScope(agent.memoryScope) }}</span>
              <span>{{ agent.tools.length }} tools</span>
            </div>
          </button>
        </div>
      </aside>

      <section v-if="selectedAgent" class="agent-detail">
        <div class="detail-head">
          <div>
            <div class="detail-title-row">
              <h3 class="detail-title">{{ selectedAgent.name }}</h3>
              <span class="detail-pill">{{ savingLabel(selectedAgent) }}</span>
              <span v-if="selectedAgent.id === store.activeAgentId" class="detail-pill active">当前使用中</span>
            </div>
            <p class="detail-subtitle">{{ selectedAgent.role }}</p>
          </div>

          <div class="detail-actions">
            <button type="button" class="ghost-btn" @click="store.setActiveAgent(selectedAgent.id)">设为当前</button>
            <button type="button" class="ghost-btn" @click="duplicateSelected">复制</button>
            <button
              type="button"
              class="ghost-btn danger"
              :disabled="!selectedAgent.deletable"
              @click="deleteAgentId = selectedAgent.id"
            >
              删除
            </button>
          </div>
        </div>

        <div class="detail-grid">
          <section class="panel-card">
            <div class="panel-title">基础信息</div>
            <div class="field-grid">
              <label class="field">
                <span>名称</span>
                <input v-model="selectedAgent.name" class="field-input" type="text" />
              </label>
              <label class="field">
                <span>角色</span>
                <input v-model="selectedAgent.role" class="field-input" type="text" />
              </label>
            </div>

            <label class="field">
              <span>人格</span>
              <textarea v-model="selectedAgent.persona" rows="5" class="field-input multiline" />
            </label>

            <div class="meta-line">
              <span class="meta-key">Agent ID</span>
              <span class="meta-value mono">{{ selectedAgent.id }}</span>
            </div>
          </section>

          <section class="panel-card">
            <div class="panel-title">记忆范围</div>
            <div class="scope-head">
              <div class="scope-summary">{{ summarizeMemoryScope(selectedAgent.memoryScope) }}</div>
              <div class="scope-mode-group">
                <button
                  v-for="mode in ['none', 'focused', 'all']"
                  :key="mode"
                  class="scope-mode-chip"
                  type="button"
                  :class="{ active: selectedAgent.memoryScope.mode === mode }"
                  @click="setScopeMode(selectedAgent, mode as MemoryScopeMode)"
                >
                  {{ mode === "none" ? "禁用" : mode === "all" ? "全量" : "定向" }}
                </button>
              </div>
            </div>

            <div class="field-grid">
              <label class="field">
                <span>Scope 名称</span>
                <input
                  v-model="selectedAgent.memoryScope.label"
                  class="field-input"
                  type="text"
                  placeholder="比如：工程上下文 / 长期偏好"
                />
              </label>
              <label class="field">
                <span>召回上限</span>
                <ThemeSelect
                  class="field-input"
                  :model-value="selectedAgent.memoryScope.recallLimit"
                  :options="recallLimitOptions"
                  @update:model-value="(value) => updateRecallLimit(selectedAgent, value)"
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
                    active: selectedAgent.memoryScope.folders.includes(folder),
                    disabled: selectedAgent.memoryScope.mode !== 'focused',
                  }"
                  @click="toggleFolder(selectedAgent, folder)"
                >
                  {{ MEMORY_SCOPE_FOLDER_LABELS[folder] }}
                </button>
              </div>
            </div>

            <label class="field">
              <span>记忆策略说明</span>
              <textarea
                v-model="selectedAgent.memoryScope.notes"
                rows="4"
                class="field-input multiline"
                placeholder="说明这个 agent 应该优先记住什么，避免什么噪音。"
              />
            </label>
          </section>

          <section class="panel-card">
            <div class="panel-title">工具范围</div>
            <div class="scope-summary tools-summary">
              {{ selectedAgent.tools.length ? `${selectedAgent.tools.length} 个已绑定能力` : "未绑定能力，将只保留系统工具" }}
            </div>

            <div v-if="selectedAgent.tools.length" class="selected-tools">
              <span v-for="tool in selectedAgent.tools" :key="tool" class="tool-pill selected">
                {{ toolTitle(tool) }}
              </span>
            </div>

            <div class="tool-grid">
              <button
                v-for="capability in toolCapabilities"
                :key="capability.name"
                class="tool-pill"
                type="button"
                :class="{ selected: selectedAgent.tools.includes(capability.name) }"
                @click="toggleTool(selectedAgent, capability.name)"
              >
                <span class="tool-title">{{ capability.title }}</span>
                <span class="tool-code">{{ capability.name }}</span>
              </button>
            </div>
          </section>

          <section class="panel-card">
            <div class="panel-title">委派设置</div>
            <div class="field-grid">
              <label class="field">
                <span>推荐标签</span>
                <input
                  class="field-input"
                  :value="tagText(selectedAgent)"
                  type="text"
                  placeholder="例如：代码, 构建, 测试"
                  @input="updateDelegateTags(selectedAgent, ($event.target as HTMLInputElement).value)"
                />
              </label>

              <label class="field">
                <span>推荐说明</span>
                <input
                  v-model="selectedAgent.delegateNote"
                  class="field-input"
                  type="text"
                  placeholder="说明何时应该把任务交给它"
                />
              </label>
            </div>

            <div class="field-grid">
              <label class="field">
                <span>允许自动委派</span>
                <button
                  class="toggle-btn"
                  type="button"
                  :class="{ active: selectedAgent.delegatable }"
                  @click="selectedAgent.delegatable = !selectedAgent.delegatable"
                >
                  {{ selectedAgent.delegatable ? "已允许" : "未允许" }}
                </button>
              </label>

              <label class="field">
                <span>委派优先级</span>
                <ThemeSelect
                  class="field-input"
                  :model-value="selectedAgent.delegatePriority"
                  :options="delegatePriorityOptions"
                  @update:model-value="(value) => updateDelegatePriority(selectedAgent, value)"
                />
              </label>
            </div>
          </section>
        </div>
      </section>

      <section v-else class="agent-detail empty-detail">
        <div class="empty-copy">
          <h3>还没有 Agent</h3>
          <p>先新增一个 Agent，再单独编辑它的角色、记忆和工具范围。</p>
        </div>
      </section>
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
.agents-page {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 32px 36px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.page-head,
.page-actions,
.detail-head,
.detail-actions,
.detail-title-row,
.scope-head,
.scope-mode-group,
.folder-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.page-head,
.detail-head,
.scope-head {
  justify-content: space-between;
}

.page-title,
.detail-title,
.panel-title,
.sidebar-title {
  margin: 0;
  color: var(--text-primary);
}

.page-title {
  font-size: 22px;
}

.page-subtitle,
.sidebar-note,
.detail-subtitle,
.scope-summary {
  margin: 6px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.6;
}

.summary-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.summary-card,
.agent-sidebar,
.agent-detail,
.panel-card {
  border: 1px solid var(--border-color);
  border-radius: 22px;
  background: var(--panel-bg);
  box-shadow: var(--shadow-surface);
}

.summary-card {
  padding: 16px 18px;
}

.summary-label,
.meta-key {
  display: block;
  color: var(--text-tertiary);
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.summary-value {
  display: block;
  margin-top: 8px;
  color: var(--text-primary);
  font-size: 18px;
}

.sync-msg {
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid rgba(255, 122, 122, 0.22);
  background: rgba(255, 122, 122, 0.08);
  color: var(--text-primary);
  font-size: 13px;
}

.layout-shell {
  min-height: 0;
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
  gap: 16px;
}

.agent-sidebar {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 0;
}

.sidebar-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
  min-height: 0;
}

.agent-row {
  width: 100%;
  padding: 14px;
  border-radius: 18px;
  border: 1px solid transparent;
  background: var(--control-bg);
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
  transition: border-color 120ms ease, background 120ms ease;
}

.agent-row:hover,
.agent-row.active {
  border-color: rgba(var(--accent-rgb), 0.26);
  background: rgba(var(--accent-rgb), 0.08);
}

.agent-row-top,
.agent-row-meta,
.selected-tools,
.tool-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.agent-row-top {
  align-items: center;
  justify-content: space-between;
}

.agent-row-name {
  font-size: 13px;
  font-weight: 600;
}

.agent-row-role,
.agent-row-meta {
  margin-top: 6px;
  color: var(--text-secondary);
  font-size: 12px;
}

.agent-row-meta {
  color: var(--text-tertiary);
}

.agent-detail {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  min-height: 0;
}

.detail-title {
  font-size: 20px;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.panel-card {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field-grid {
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
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: var(--control-bg);
  color: var(--text-primary);
  padding: 12px 14px;
  outline: none;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}

.field-input:focus {
  border-color: rgba(var(--accent-rgb), 0.34);
  box-shadow: var(--focus-ring);
}

.multiline {
  resize: vertical;
  min-height: 96px;
}

.meta-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 4px;
}

.meta-value {
  color: var(--text-primary);
  font-size: 12px;
}

.mono {
  font-family: var(--font-mono);
}

.row-pill,
.detail-pill,
.scope-mode-chip,
.folder-chip,
.tool-pill,
.ghost-btn,
.primary-btn {
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--control-bg);
  color: var(--text-primary);
}

.row-pill,
.detail-pill {
  padding: 5px 10px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.row-pill.active,
.detail-pill.active,
.scope-mode-chip.active,
.folder-chip.active,
.tool-pill.selected {
  border-color: rgba(var(--accent-rgb), 0.3);
  background: rgba(var(--accent-rgb), 0.12);
}

.primary-btn,
.ghost-btn,
.scope-mode-chip,
.folder-chip,
.tool-pill,
.toggle-btn {
  cursor: pointer;
}

.primary-btn {
  padding: 10px 14px;
  font-size: 12px;
}

.ghost-btn {
  padding: 8px 12px;
  font-size: 12px;
}

.ghost-btn.danger:disabled {
  opacity: 0.45;
  cursor: default;
}

.folder-chip {
  padding: 8px 12px;
  font-size: 12px;
}

.folder-chip.disabled {
  opacity: 0.5;
}

.tool-pill {
  padding: 10px 12px;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 148px;
}

.tool-title {
  font-size: 12px;
}

.tool-code {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.tools-summary {
  margin-top: -4px;
}

.toggle-btn {
  min-height: 44px;
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: var(--control-bg);
  color: var(--text-secondary);
  padding: 0 14px;
  text-align: left;
}

.toggle-btn.active {
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.3);
  background: rgba(var(--accent-rgb), 0.12);
}

.empty-detail {
  justify-content: center;
  align-items: center;
}

.empty-copy {
  text-align: center;
  color: var(--text-secondary);
}

.empty-copy h3 {
  margin: 0 0 8px;
  color: var(--text-primary);
}

@media (max-width: 1080px) {
  .layout-shell {
    grid-template-columns: 1fr;
  }

  .agent-sidebar {
    max-height: 260px;
  }

  .detail-grid,
  .field-grid,
  .summary-strip {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 720px) {
  .agents-page {
    padding: 20px 18px 28px;
  }

  .page-head,
  .detail-head,
  .scope-head {
    flex-direction: column;
  }
}
</style>
