<script setup lang="ts">
import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"

type ActionSource = "builtin" | "skill"

interface Action {
  id: string
  title: string
  description: string
  source: ActionSource
  capabilities: string[]
  tags: string[]
  path?: string | null
  entry?: string | null
  available: boolean
}

interface ActionBlueprint {
  id: string
  title: string
  description: string
  source: string
  mode: string
  capabilities: string[]
  tags: string[]
  path?: string | null
  entry?: string | null
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

const builtinCount = computed(() => actions.value.filter((action) => action.source === "builtin").length)
const skillCount = computed(() => actions.value.filter((action) => action.source === "skill").length)

function sourceLabel(source: ActionSource) {
  return source === "skill" ? "skill" : "builtin"
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
    refreshMsg.value = "已刷新 action 注册表与本地 skill 兼容层。"
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

onMounted(loadActions)
</script>

<template>
  <div class="actions-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">Actions</h2>
        <p class="view-subtitle">
          稳定动作入口与 skill 兼容层。这里展示当前桌面 Agent 已注册的内建 action，以及从本地 `SKILL.md` 生态发现到的兼容 skill。
        </p>
      </div>
      <div class="head-actions">
        <div class="count-group">
          <span class="section-count">{{ actions.length }}</span>
          <span class="sub-count">builtin {{ builtinCount }} · skill {{ skillCount }}</span>
        </div>
        <button class="refresh-btn" :disabled="loading" @click="refreshActions">
          {{ loading ? "刷新中" : "刷新" }}
        </button>
      </div>
    </header>

    <div v-if="refreshMsg" class="refresh-msg">{{ refreshMsg }}</div>

    <div class="action-list">
      <div v-for="a in actions" :key="a.id" class="action-card">
        <div class="action-main">
          <div class="action-top">
            <span class="action-id">{{ a.id }}</span>
            <div class="pill-row">
              <span class="action-pill" :class="a.source">{{ sourceLabel(a.source) }}</span>
              <span class="action-pill subtle">{{ a.available ? "ready" : "missing" }}</span>
              <button class="detail-btn" @click="toggleActionDetail(a.id)">
                {{ expandedActionId === a.id ? "收起转化结果" : "查看转化结果" }}
              </button>
            </div>
          </div>

          <div class="action-title">{{ a.title }}</div>
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

          <div v-if="a.path || a.entry" class="meta-block">
            <span class="meta-label">Compatibility Evidence</span>
            <div v-if="a.path" class="meta-path">{{ a.path }}</div>
            <div v-if="a.entry" class="meta-path">{{ a.entry }}</div>
          </div>

          <div v-if="expandedActionId === a.id" class="blueprint-panel">
            <div v-if="detailLoading[a.id]" class="blueprint-empty">正在编译 action blueprint...</div>
            <div v-else-if="detailErrors[a.id]" class="blueprint-error">{{ detailErrors[a.id] }}</div>
            <template v-else-if="actionBlueprints[a.id]">
              <div class="blueprint-top">
                <span class="meta-label">Action Mode</span>
                <span class="chip">{{ actionBlueprints[a.id].mode }}</span>
              </div>

              <div class="meta-block">
                <span class="meta-label">Compiled Prompt</span>
                <pre class="blueprint-code">{{ actionBlueprints[a.id].compiledPrompt }}</pre>
              </div>

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
            <div v-else class="blueprint-empty">当前 action 还没有可展示的 blueprint。</div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="!actions.length && !loading" class="empty-state">
      <p>当前还没有可展示的 action 或 skill 兼容项。</p>
    </div>
  </div>
</template>

<style scoped>
.actions-view {
  padding: 28px 32px 32px;
  max-width: 980px;
  height: 100%;
  overflow-y: auto;
}

.section-head,
.head-actions,
.count-group,
.action-top,
.pill-row,
.chip-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.section-head,
.action-top {
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
  max-width: 640px;
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
.action-pill,
.refresh-btn,
.chip,
.detail-btn {
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

.detail-btn {
  padding: 5px 10px;
  font-size: 11px;
  cursor: pointer;
}

.refresh-btn:disabled {
  opacity: 0.55;
  cursor: default;
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

.action-card {
  padding: 18px 18px 16px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  border-radius: 18px;
  box-shadow: var(--shadow-surface);
  transition: background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}

.action-card:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.05);
  transform: translateY(-1px);
}

.action-main {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.action-id {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
}

.action-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.action-desc {
  font-size: 13px;
  line-height: 1.65;
  color: var(--text-primary);
}

.action-pill {
  padding: 5px 10px;
  font-size: 11px;
  text-transform: uppercase;
}

.action-pill.builtin {
  background: rgba(var(--accent-rgb), 0.12);
  border-color: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.action-pill.skill {
  background: rgba(255, 255, 255, 0.04);
}

.action-pill.subtle,
.chip.subtle {
  opacity: 0.8;
}

.meta-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.meta-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
}

.chip-row {
  flex-wrap: wrap;
}

.chip {
  padding: 5px 10px;
  font-size: 11px;
}

.meta-path {
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  word-break: break-all;
}

.blueprint-panel {
  margin-top: 2px;
  padding: 14px;
  border-radius: 16px;
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  background: rgba(var(--accent-rgb), 0.05);
}

.blueprint-top {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.blueprint-code {
  margin: 0;
  padding: 12px 14px;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.65;
  font-family: var(--font-mono);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 360px;
  overflow: auto;
}

.blueprint-empty,
.blueprint-error {
  font-size: 12px;
  color: var(--text-secondary);
}

.blueprint-error {
  color: #fecaca;
}

.empty-state {
  margin-top: 14px;
  padding: 34px;
  text-align: center;
  border: 1px dashed var(--border-color);
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.018);
  color: var(--text-tertiary);
  font-size: 14px;
}

@media (max-width: 800px) {
  .actions-view {
    padding: 20px 18px 24px;
  }

  .section-head,
  .head-actions,
  .action-top {
    flex-direction: column;
  }

  .count-group {
    align-items: flex-start;
  }
}
</style>
