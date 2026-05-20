<script setup lang="ts">
import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ConfirmDialog from "../components/ConfirmDialog.vue"

type ActionSource = "system" | "user"

interface Action {
  id: string
  title: string
  description: string
  source: ActionSource
  mode: string
  capabilities: string[]
  tags: string[]
  enabled: boolean
  editable: boolean
  deletable: boolean
  available: boolean
  sourceHint?: string | null
  supportingFiles: string[]
}

interface ActionBlueprint {
  id: string
  title: string
  description: string
  source: string
  mode: string
  capabilities: string[]
  tags: string[]
  sourceHint?: string | null
  compiledPrompt: string
  supportingFiles: string[]
}

const actions = ref<Action[]>([])
const loading = ref(false)
const refreshMsg = ref("")
const expandedActionId = ref<string | null>(null)
const actionBlueprints = ref<Record<string, ActionBlueprint>>({})
const detailLoading = ref<Record<string, boolean>>({})
const detailErrors = ref<Record<string, string | null>>({})
const deleteConfirmId = ref<string | null>(null)

const systemCount = computed(() => actions.value.filter((action) => action.source === "system").length)
const userCount = computed(() => actions.value.filter((action) => action.source === "user").length)
const enabledCount = computed(() => actions.value.filter((action) => action.enabled).length)
const readyCount = computed(() => actions.value.filter((action) => action.available).length)
const deleteActionTarget = computed(() =>
  actions.value.find((action) => action.id === deleteConfirmId.value) || null
)

function sourceLabel(source: ActionSource) {
  return source === "system" ? "system" : "user"
}

async function loadActions() {
  loading.value = true
  refreshMsg.value = ""
  try {
    actions.value = await invoke<Action[]>("get_actions")
  } catch (error) {
    console.error("load actions:", error)
    refreshMsg.value = String(error)
  } finally {
    loading.value = false
  }
}

async function refreshActions() {
  loading.value = true
  refreshMsg.value = ""
  try {
    actions.value = await invoke<Action[]>("refresh_actions")
    refreshMsg.value = "已同步 action 注册目录与当前 capability 绑定状态。"
  } catch (error) {
    console.error("refresh actions:", error)
    refreshMsg.value = String(error)
  } finally {
    loading.value = false
  }
}

async function toggleActionDetail(actionId: string) {
  if (expandedActionId.value === actionId) {
    expandedActionId.value = null
    return
  }

  expandedActionId.value = actionId
  if (actionBlueprints.value[actionId] || detailLoading.value[actionId]) {
    return
  }

  detailLoading.value = { ...detailLoading.value, [actionId]: true }
  detailErrors.value = { ...detailErrors.value, [actionId]: null }
  try {
    const blueprint = await invoke<ActionBlueprint>("get_action_blueprint", { actionId })
    actionBlueprints.value = {
      ...actionBlueprints.value,
      [actionId]: blueprint,
    }
  } catch (error) {
    console.error("get action blueprint:", error)
    detailErrors.value = {
      ...detailErrors.value,
      [actionId]: String(error),
    }
  } finally {
    detailLoading.value = { ...detailLoading.value, [actionId]: false }
  }
}

async function toggleAction(action: Action) {
  try {
    actions.value = await invoke<Action[]>("set_action_enabled", {
      id: action.id,
      enabled: !action.enabled,
    })
    deleteConfirmId.value = null
    refreshMsg.value = action.enabled ? `已停用 ${action.title}` : `已启用 ${action.title}`
  } catch (error) {
    console.error("toggle action:", error)
    refreshMsg.value = String(error)
  }
}

async function removeAction(action: Action) {
  try {
    actions.value = await invoke<Action[]>("delete_action", { id: action.id })
    deleteConfirmId.value = null
    if (expandedActionId.value === action.id) {
      expandedActionId.value = null
    }
    refreshMsg.value = `已删除 ${action.title}`
  } catch (error) {
    console.error("delete action:", error)
    refreshMsg.value = String(error)
  }
}

function confirmDeleteAction() {
  if (!deleteActionTarget.value) return
  removeAction(deleteActionTarget.value)
}

onMounted(loadActions)
</script>

<template>
  <div class="actions-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">Action 注册</h2>
        <p class="view-subtitle">
          管理 action 目录、状态和蓝图定义。
        </p>
      </div>
      <div class="head-actions">
        <div class="count-group">
          <span class="section-count">{{ actions.length }}</span>
          <span class="sub-count">system {{ systemCount }} · user {{ userCount }}</span>
        </div>
        <button type="button" class="refresh-btn" :disabled="loading" @click="refreshActions">
          {{ loading ? "同步中" : "同步 action 目录" }}
        </button>
      </div>
    </header>

    <section class="summary-panel">
      <article class="summary-card">
        <span class="summary-label">Action 规范</span>
        <strong class="summary-value">兼容外部 skill</strong>
        <p class="summary-note">支持把 skill、prompt 和工作流沉淀为可复用 action。</p>
      </article>
      <article class="summary-card">
        <span class="summary-label">注册方式</span>
        <strong class="summary-value">AI 自注册</strong>
        <p class="summary-note">通过 `action_list` 和 `action_register` 维护注册表。</p>
      </article>
      <article class="summary-card">
        <span class="summary-label">当前状态</span>
        <strong class="summary-value">{{ enabledCount }} enabled · {{ readyCount }} ready</strong>
        <p class="summary-note">ready 表示依赖能力已齐备，可直接测试。</p>
      </article>
    </section>

    <div v-if="refreshMsg" class="refresh-msg">{{ refreshMsg }}</div>

    <div class="action-list">
      <div v-for="a in actions" :key="a.id" class="action-card">
        <div class="action-main">
          <div class="action-top">
            <div>
              <span class="action-id">{{ a.id }}</span>
              <div class="action-title">{{ a.title }}</div>
            </div>
            <div class="pill-row">
              <span class="action-pill" :class="a.source">{{ sourceLabel(a.source) }}</span>
              <span class="action-pill subtle">{{ a.mode }}</span>
              <span :class="['action-pill', a.available ? 'ready' : 'warn']">
                {{ a.available ? "ready" : "missing dependency" }}
              </span>
              <span :class="['action-pill', a.enabled ? 'ready' : 'muted']">
                {{ a.enabled ? "enabled" : "disabled" }}
              </span>
            </div>
          </div>

          <div class="action-desc">{{ a.description }}</div>

          <div v-if="a.capabilities.length" class="meta-block">
            <span class="meta-label">Capabilities</span>
            <div class="chip-row">
              <span v-for="capability in a.capabilities" :key="capability" class="chip">
                {{ capability }}
              </span>
            </div>
          </div>

          <div v-if="a.tags.length" class="meta-block">
            <span class="meta-label">Tags</span>
            <div class="chip-row">
              <span v-for="tag in a.tags" :key="tag" class="chip subtle">
                {{ tag }}
              </span>
            </div>
          </div>

          <div v-if="a.sourceHint || a.supportingFiles.length" class="meta-block">
            <span class="meta-label">Source</span>
            <div v-if="a.sourceHint" class="meta-path">{{ a.sourceHint }}</div>
            <div v-if="a.supportingFiles.length" class="chip-row">
              <span v-for="file in a.supportingFiles" :key="file" class="chip subtle">
                {{ file }}
              </span>
            </div>
          </div>

          <div class="action-actions">
            <button type="button" class="detail-btn" @click="toggleActionDetail(a.id)">
              {{ expandedActionId === a.id ? "收起蓝图" : "查看蓝图" }}
            </button>
            <button
              v-if="a.source === 'user'"
              class="detail-btn"
              type="button"
              @click="toggleAction(a)"
            >
              {{ a.enabled ? "停用" : "启用" }}
            </button>
            <button
              v-if="a.deletable"
              class="detail-btn danger"
              type="button"
              @click="deleteConfirmId = a.id"
            >
              删除
            </button>
          </div>

          <div v-if="expandedActionId === a.id" class="blueprint-panel">
            <div v-if="detailLoading[a.id]" class="blueprint-empty">正在加载 action blueprint...</div>
            <div v-else-if="detailErrors[a.id]" class="blueprint-error">{{ detailErrors[a.id] }}</div>
            <template v-else-if="actionBlueprints[a.id]">
              <div class="blueprint-top">
                <span class="meta-label">Compiled Prompt</span>
                <span class="chip">{{ actionBlueprints[a.id].mode }}</span>
              </div>

              <div v-if="actionBlueprints[a.id].sourceHint" class="meta-path">
                {{ actionBlueprints[a.id].sourceHint }}
              </div>

              <pre class="blueprint-code">{{ actionBlueprints[a.id].compiledPrompt }}</pre>

              <div v-if="actionBlueprints[a.id].supportingFiles.length" class="meta-block">
                <span class="meta-label">Supporting Files</span>
                <div class="chip-row">
                  <span
                    v-for="file in actionBlueprints[a.id].supportingFiles"
                    :key="file"
                    class="chip subtle"
                  >
                    {{ file }}
                  </span>
                </div>
              </div>
            </template>
            <div v-else class="blueprint-empty">当前 action 还没有可展示的蓝图。</div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="!actions.length && !loading" class="empty-state">
      <p>当前还没有 action。先在对话中让 AI 通过 `action_register` 注册一条新的 action。</p>
    </div>

    <ConfirmDialog
      :open="!!deleteConfirmId"
      title="删除 Action"
      :message="deleteActionTarget ? `确定删除「${deleteActionTarget.title}」吗？删除后将移除这条 Action 配置。` : '删除后将移除这条 Action 配置。'"
      @cancel="deleteConfirmId = null"
      @confirm="confirmDeleteAction"
    />
  </div>
</template>

<style scoped>
.actions-view {
  padding: 28px 32px 32px;
  max-width: 1060px;
  width: 100%;
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.section-head,
.head-actions,
.count-group,
.action-top,
.pill-row,
.chip-row,
.action-actions,
.blueprint-top {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.section-head,
.action-top,
.blueprint-top {
  justify-content: space-between;
}

.section-head {
  margin-bottom: 18px;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.view-subtitle {
  max-width: 720px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.head-actions {
  flex-shrink: 0;
}

.count-group {
  flex-direction: column;
  gap: 6px;
  align-items: flex-end;
}

.section-count,
.refresh-btn,
.action-pill,
.chip {
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.section-count {
  min-width: 40px;
  padding: 8px 12px;
  font-size: 12px;
  text-align: center;
}

.sub-count {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.refresh-btn {
  padding: 8px 12px;
  font-size: 11px;
  cursor: pointer;
}

.summary-panel {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}

.summary-card,
.action-card {
  padding: 18px;
  border-radius: 20px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.summary-label,
.meta-label {
  display: block;
  margin-bottom: 8px;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.summary-value {
  display: block;
  margin-bottom: 8px;
  font-size: 16px;
  color: var(--text-primary);
}

.summary-note,
.action-desc {
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-secondary);
}

.refresh-msg {
  margin-bottom: 14px;
  padding: 10px 12px;
  border-radius: 14px;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  color: var(--text-secondary);
  font-size: 12px;
}

.action-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.action-id {
  display: block;
  margin-bottom: 8px;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.action-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.action-desc {
  margin: 12px 0 0;
}

.pill-row,
.chip-row,
.action-actions {
  flex-wrap: wrap;
}

.action-pill,
.chip {
  padding: 5px 10px;
  font-size: 11px;
}

.action-pill.system {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.24);
}

.action-pill.user {
  color: #cbe6ff;
  border-color: rgba(140, 190, 255, 0.22);
}

.action-pill.ready {
  color: #c8facc;
  border-color: rgba(115, 220, 145, 0.18);
  background: rgba(115, 220, 145, 0.08);
}

.action-pill.warn {
  color: #ffd6a0;
  border-color: rgba(255, 176, 90, 0.18);
  background: rgba(255, 176, 90, 0.08);
}

.action-pill.muted {
  opacity: 0.72;
}

.meta-block {
  margin-top: 14px;
}

.meta-path {
  margin-top: 8px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
  word-break: break-word;
  font-family: var(--font-mono);
}

.action-actions {
  margin-top: 16px;
}

.detail-btn {
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: var(--surface-2);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
}

.detail-btn.danger {
  color: #ffb0b0;
  border-color: rgba(255, 120, 120, 0.18);
}

.detail-btn.danger.solid {
  background: rgba(255, 120, 120, 0.1);
}

.blueprint-panel {
  margin-top: 16px;
  padding: 16px;
  border-radius: 18px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.blueprint-empty,
.blueprint-error {
  font-size: 12px;
  color: var(--text-secondary);
}

.blueprint-error {
  color: #ffb6b6;
}

.blueprint-code {
  margin: 12px 0 0;
  padding: 14px;
  border-radius: 16px;
  background: var(--surface-2);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-mono);
}

.empty-state {
  margin-top: 18px;
  padding: 22px;
  border-radius: 18px;
  border: 1px dashed var(--border-color);
  color: var(--text-secondary);
  text-align: center;
}

@media (max-width: 800px) {
  .actions-view {
    padding: 20px 18px 24px;
  }

  .section-head,
  .head-actions,
  .action-top,
  .blueprint-top {
    flex-direction: column;
  }

  .count-group {
    align-items: flex-start;
  }
}
</style>
