<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ConfirmDialog from "../components/ConfirmDialog.vue"
import ThemeSelect from "../components/ThemeSelect.vue"
import {
  capabilitySourceLabel,
  getToolMeta,
  toolRiskLabel,
  type CapabilityInfo,
  type ToolRisk,
} from "../lib/tools"
import type { AgentMode } from "../stores/chat"

type PolicyMode = "Sandbox" | "Danger"

interface AppConfig {
  policy_mode: PolicyMode
  agent_mode: AgentMode
}

interface ToolCard extends CapabilityInfo {
  label: string
  short: string
  implementationLabel: string
  visible: boolean
  blockedByPolicy: boolean
  runtimeHint: string
}

interface CapabilityDraft {
  existingName: string | null
  name: string
  title: string
  description: string
  riskLevel: ToolRisk
  implementation: string
  enabled: boolean
}

const capabilities = ref<CapabilityInfo[]>([])
const loading = ref(false)
const saving = ref(false)
const refreshMsg = ref("")
const formError = ref("")
const deleteConfirmName = ref<string | null>(null)
const policyMode = ref<PolicyMode>("Sandbox")
const selectedAgentMode = ref<AgentMode>("agent")
const editorOpen = ref(false)

const draft = reactive<CapabilityDraft>({
  existingName: null,
  name: "",
  title: "",
  description: "",
  riskLevel: "ReadOnly",
  implementation: "read",
  enabled: true,
})

const riskOptions = [
  { label: "只读", value: "ReadOnly" },
  { label: "受控写入", value: "ControlledWrite" },
  { label: "高风险", value: "HighRisk" },
]

function riskWeight(risk: ToolRisk): number {
  switch (risk) {
    case "ReadOnly":
      return 0
    case "ControlledWrite":
      return 1
    case "HighRisk":
      return 2
    default:
      return 3
  }
}

function isToolVisible(implementation: string, risk: ToolRisk, mode: AgentMode): boolean {
  if (implementation === "checklist_write") {
    return true
  }
  return mode === "plan" ? risk === "ReadOnly" : true
}

function runtimeHintFor(tool: CapabilityInfo, visible: boolean, blockedByPolicy: boolean): string {
  if (!tool.enabled) {
    return "当前已停用，不会暴露给模型，也不会进入实际执行。"
  }
  if (!tool.executable) {
    return "底层执行器不可用，这条能力目前只是注册信息。"
  }
  if (!visible) {
    return "当前 Agent Mode 下隐藏，不会出现在模型工具清单里。"
  }
  if (blockedByPolicy) {
    return "当前会被策略层挡住，模型能看到，但执行阶段会被拒绝。"
  }
  if (tool.risk_level === "HighRisk") {
    return selectedAgentMode.value === "auto"
      ? "当前可直接执行，高风险操作不会再二次确认。"
      : "当前会要求确认，避免模型直接执行高风险操作。"
  }
  if (tool.risk_level === "ControlledWrite") {
    return "当前可执行，但仍受 allow/deny 规则限制。"
  }
  return "当前可直接执行，适合检索、读取和分析。"
}

const builtinCapabilities = computed(() =>
  capabilities.value.filter((tool) => tool.source === "builtin")
)

const implementationOptions = computed(() =>
  builtinCapabilities.value.map((tool) => ({
    label: `${tool.title} · ${tool.name}`,
    value: tool.name,
  }))
)

const capabilityCards = computed<ToolCard[]>(() =>
  [...capabilities.value]
    .sort((a, b) => {
      const sourceRank = (a.source === "builtin" ? 0 : 1) - (b.source === "builtin" ? 0 : 1)
      if (sourceRank !== 0) return sourceRank
      return (
        riskWeight(a.risk_level) - riskWeight(b.risk_level) ||
        a.title.localeCompare(b.title) ||
        a.name.localeCompare(b.name)
      )
    })
    .map((tool) => {
      const implementationMeta = getToolMeta(tool.implementation)
      const visible = tool.enabled && tool.executable && isToolVisible(tool.implementation, tool.risk_level, selectedAgentMode.value)
      const blockedByPolicy = tool.enabled && visible && policyMode.value === "Sandbox" && tool.risk_level !== "ReadOnly"
      return {
        ...tool,
        label: tool.title || implementationMeta.label,
        short:
          tool.source === "builtin"
            ? implementationMeta.short
            : `基于 ${implementationMeta.label} 的自定义能力包装`,
        implementationLabel: implementationMeta.label,
        visible,
        blockedByPolicy,
        runtimeHint: runtimeHintFor(tool, visible, blockedByPolicy),
      }
    })
)

const builtinCount = computed(() => capabilityCards.value.filter((tool) => tool.source === "builtin").length)
const userCount = computed(() => capabilityCards.value.filter((tool) => tool.source === "user").length)
const enabledCount = computed(() => capabilityCards.value.filter((tool) => tool.enabled).length)
const visibleCount = computed(() => capabilityCards.value.filter((tool) => tool.visible).length)
const blockedCount = computed(() => capabilityCards.value.filter((tool) => tool.blockedByPolicy).length)
const deleteCapabilityTarget = computed(() =>
  capabilityCards.value.find((tool) => tool.name === deleteConfirmName.value) || null
)

const executionSummary = computed(() => {
  if (policyMode.value === "Sandbox") {
    return "当前是 Sandbox。你可以注册和组织能力，但非只读能力仍会在执行阶段被策略层限制。"
  }
  if (selectedAgentMode.value === "auto") {
    return "当前是 Danger + Auto。启用中的能力会完整暴露给模型，高风险也能连续执行。"
  }
  if (selectedAgentMode.value === "plan") {
    return "当前是 Danger + Plan。只读能力和 checklist 类能力会保留，写入与高风险能力隐藏。"
  }
  return "当前是 Danger + Agent。启用中的能力全部可见，但高风险默认仍要确认。"
})

function resetDraft() {
  draft.existingName = null
  draft.name = ""
  draft.title = ""
  draft.description = ""
  draft.riskLevel = "ReadOnly"
  draft.implementation = implementationOptions.value[0]?.value?.toString() || "read"
  draft.enabled = true
  formError.value = ""
}

function openCreateEditor() {
  resetDraft()
  editorOpen.value = true
}

function openEditEditor(tool: CapabilityInfo) {
  if (!tool.editable) return
  draft.existingName = tool.name
  draft.name = tool.name
  draft.title = tool.title
  draft.description = tool.description
  draft.riskLevel = tool.risk_level
  draft.implementation = tool.implementation
  draft.enabled = tool.enabled
  formError.value = ""
  editorOpen.value = true
}

function cancelEditor() {
  editorOpen.value = false
  resetDraft()
}

async function loadCapabilities() {
  loading.value = true
  refreshMsg.value = ""
  try {
    const [config, caps] = await Promise.all([
      invoke<AppConfig>("get_config"),
      invoke<CapabilityInfo[]>("get_capabilities"),
    ])
    policyMode.value = config.policy_mode
    selectedAgentMode.value = config.agent_mode
    capabilities.value = caps
    if (!draft.implementation && implementationOptions.value.length) {
      draft.implementation = String(implementationOptions.value[0].value)
    }
  } catch (error) {
    console.error("load capabilities:", error)
    refreshMsg.value = String(error)
  } finally {
    loading.value = false
  }
}

async function refreshCapabilities() {
  await loadCapabilities()
  if (!refreshMsg.value) {
    refreshMsg.value = "已同步当前能力注册表、策略模式和 Agent Mode。"
  }
}

async function saveDraft() {
  saving.value = true
  formError.value = ""
  refreshMsg.value = ""
  try {
    capabilities.value = await invoke<CapabilityInfo[]>("upsert_capability", {
      input: {
        existingName: draft.existingName,
        name: draft.name.trim(),
        title: draft.title.trim(),
        description: draft.description.trim(),
        riskLevel: draft.riskLevel,
        implementation: draft.implementation,
        enabled: draft.enabled,
      },
    })
    editorOpen.value = false
    deleteConfirmName.value = null
    refreshMsg.value = draft.existingName ? "自定义能力已更新。" : "自定义能力已注册。"
    resetDraft()
  } catch (error) {
    console.error("save capability:", error)
    formError.value = String(error)
  } finally {
    saving.value = false
  }
}

async function toggleCapability(tool: CapabilityInfo) {
  try {
    capabilities.value = await invoke<CapabilityInfo[]>("set_capability_enabled", {
      name: tool.name,
      enabled: !tool.enabled,
    })
    deleteConfirmName.value = null
    refreshMsg.value = tool.enabled ? `已停用 ${tool.title}` : `已启用 ${tool.title}`
  } catch (error) {
    console.error("toggle capability:", error)
    refreshMsg.value = String(error)
  }
}

async function removeCapability(tool: CapabilityInfo) {
  try {
    capabilities.value = await invoke<CapabilityInfo[]>("delete_capability", {
      name: tool.name,
    })
    deleteConfirmName.value = null
    if (draft.existingName === tool.name) {
      cancelEditor()
    }
    refreshMsg.value = `已删除 ${tool.title}`
  } catch (error) {
    console.error("delete capability:", error)
    refreshMsg.value = String(error)
  }
}

function confirmDeleteCapability() {
  if (!deleteCapabilityTarget.value) return
  removeCapability(deleteCapabilityTarget.value)
}

onMounted(async () => {
  await loadCapabilities()
  resetDraft()
})
</script>

<template>
  <div class="atomic-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">原子化能力</h2>
        <p class="view-subtitle">
          管理能力注册表、启停状态和风险分级。
        </p>
      </div>
      <div class="head-actions">
        <div class="count-group">
          <span class="section-count">{{ capabilityCards.length }}</span>
          <span class="sub-count">builtin {{ builtinCount }} · user {{ userCount }}</span>
        </div>
        <button type="button" class="refresh-btn" :disabled="loading" @click="refreshCapabilities">
          {{ loading ? "同步中" : "同步能力视图" }}
        </button>
      </div>
    </header>

    <section class="panel">
      <div class="panel-head">
        <div>
          <h3 class="panel-title">注册层状态</h3>
          <p class="panel-subtitle">用户能力映射到底层执行器，用于统一注册、展示和权限治理。</p>
        </div>
        <button type="button" class="primary-btn" @click="openCreateEditor">
          {{ editorOpen && !draft.existingName ? "正在新建" : "注册能力" }}
        </button>
      </div>

      <div class="tool-stats">
        <span class="pill stat">{{ enabledCount }} enabled</span>
        <span class="pill stat ok">{{ visibleCount }} visible</span>
        <span class="pill stat warn">{{ blockedCount }} blocked</span>
      </div>

      <div class="status-banner">
        <span class="status-dot" />
        <span>{{ executionSummary }}</span>
      </div>

      <div v-if="refreshMsg" class="refresh-msg">{{ refreshMsg }}</div>

      <div v-if="editorOpen" class="editor-panel">
        <div class="editor-top">
          <div>
            <div class="editor-title">{{ draft.existingName ? "编辑自定义能力" : "注册新能力" }}</div>
            <div class="editor-note">推荐把能力名控制在稳定、简短、机器可读的范围内，例如 `project_read_docs`、`memory_capture_note`。</div>
          </div>
          <button type="button" class="ghost-btn" @click="cancelEditor">收起</button>
        </div>

        <div class="editor-grid">
          <label class="field">
            <span>能力标题</span>
            <input v-model="draft.title" class="field-input" type="text" placeholder="比如：项目文档读取" />
          </label>

          <label class="field">
            <span>能力名</span>
            <input v-model="draft.name" class="field-input mono" type="text" placeholder="project_read_docs" />
          </label>

          <label class="field">
            <span>基础执行器</span>
            <ThemeSelect
              class="field-input"
              :model-value="draft.implementation"
              :options="implementationOptions"
              @update:model-value="(value) => draft.implementation = String(value)"
            />
          </label>

          <label class="field">
            <span>风险等级</span>
            <ThemeSelect
              class="field-input"
              :model-value="draft.riskLevel"
              :options="riskOptions"
              @update:model-value="(value) => draft.riskLevel = value as ToolRisk"
            />
          </label>
        </div>

        <label class="field full">
          <span>能力描述</span>
          <textarea
            v-model="draft.description"
            class="field-input multiline"
            rows="4"
            placeholder="描述模型在什么场景下应该调用这条能力。"
          />
        </label>

        <label class="toggle-row">
          <button
            class="toggle-btn"
            :class="{ active: draft.enabled }"
            type="button"
            @click="draft.enabled = !draft.enabled"
          >
            <span class="toggle-dot" />
          </button>
          <span>{{ draft.enabled ? "保存后立即启用" : "保存后先停用" }}</span>
        </label>

        <div v-if="formError" class="form-error">{{ formError }}</div>

        <div class="editor-actions">
          <button type="button" class="ghost-btn" :disabled="saving" @click="cancelEditor">取消</button>
          <button type="button" class="primary-btn" :disabled="saving" @click="saveDraft">
            {{ saving ? "保存中" : draft.existingName ? "更新能力" : "创建能力" }}
          </button>
        </div>
      </div>
    </section>

    <section class="panel">
      <div class="panel-head">
        <div>
          <h3 class="panel-title">能力清单</h3>
          <p class="panel-subtitle">这里显示真实注册结果。启停会同步影响模型实际可见的工具清单，用户能力则会继续映射到指定的内置执行器。</p>
        </div>
      </div>

      <div class="tool-grid">
        <article v-for="tool in capabilityCards" :key="tool.name" class="tool-card">
          <div class="tool-top">
            <div>
              <div class="tool-label">{{ tool.label }}</div>
              <div class="tool-name">{{ tool.name }}</div>
            </div>
            <div class="tool-tags">
              <span class="pill">{{ capabilitySourceLabel(tool.source) }}</span>
              <span class="pill risk">{{ toolRiskLabel(tool.risk_level) }}</span>
              <span :class="['pill', tool.enabled ? 'ok' : 'muted']">{{ tool.enabled ? "启用中" : "已停用" }}</span>
              <span v-if="tool.blockedByPolicy" class="pill warn">策略拦截</span>
            </div>
          </div>

          <p class="tool-short">{{ tool.short }}</p>
          <p class="tool-detail">{{ tool.description }}</p>
          <div class="tool-meta">
            <span>底层执行器</span>
            <strong>{{ tool.implementationLabel }}</strong>
            <code>{{ tool.implementation }}</code>
          </div>
          <p class="tool-runtime">{{ tool.runtimeHint }}</p>

          <div class="card-actions">
            <button type="button" class="small-btn" @click="toggleCapability(tool)">
              {{ tool.enabled ? "停用" : "启用" }}
            </button>
            <button v-if="tool.editable" type="button" class="small-btn" @click="openEditEditor(tool)">编辑</button>
            <button
              v-if="tool.deletable"
              class="small-btn danger"
              type="button"
              @click="deleteConfirmName = tool.name"
            >
              删除
            </button>
          </div>
        </article>
      </div>
    </section>

    <ConfirmDialog
      :open="!!deleteConfirmName"
      title="删除原子化能力"
      :message="deleteCapabilityTarget ? `确定删除「${deleteCapabilityTarget.title}」吗？删除后将移除这条用户能力配置。` : '删除后将移除这条用户能力配置。'"
      @cancel="deleteConfirmName = null"
      @confirm="confirmDeleteCapability"
    />
  </div>
</template>

<style scoped>
.atomic-view {
  padding: 28px 32px 32px;
  max-width: 1120px;
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
.tool-top,
.tool-tags,
.tool-stats,
.panel-head,
.editor-top,
.editor-actions,
.card-actions {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.section-head,
.panel-head,
.editor-top {
  justify-content: space-between;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.view-subtitle,
.panel-subtitle,
.editor-note {
  max-width: 760px;
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
.primary-btn,
.ghost-btn,
.pill {
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

.refresh-btn,
.primary-btn,
.ghost-btn {
  padding: 9px 14px;
  font-size: 11px;
  cursor: pointer;
}

.primary-btn {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.28);
  background: rgba(var(--accent-rgb), 0.08);
}

.ghost-btn {
  background: var(--surface-2);
}

.panel {
  padding: 22px 24px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.panel-title,
.editor-title {
  margin: 0 0 4px;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.tool-stats,
.tool-tags {
  flex-wrap: wrap;
}

.status-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 16px;
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.6;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 0 4px rgba(var(--accent-rgb), 0.12);
  flex-shrink: 0;
}

.refresh-msg,
.form-error {
  margin-top: 14px;
  padding: 10px 12px;
  border-radius: 14px;
  font-size: 12px;
  line-height: 1.6;
}

.refresh-msg {
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  color: var(--text-secondary);
}

.form-error {
  background: rgba(255, 110, 110, 0.08);
  border: 1px solid rgba(255, 110, 110, 0.18);
  color: #ffb6b6;
}

.editor-panel {
  margin-top: 18px;
  padding: 18px;
  border-radius: 20px;
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.editor-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 12px;
  color: var(--text-secondary);
}

.field.full {
  width: 100%;
}

.field-input {
  min-height: 42px;
  padding: 0 14px;
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: var(--surface-2);
  color: var(--text-primary);
}

.field-input.multiline {
  min-height: 120px;
  padding: 12px 14px;
  resize: vertical;
}

.field-input.mono {
  font-family: var(--font-mono);
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: var(--text-secondary);
}

.toggle-btn {
  width: 48px;
  height: 28px;
  padding: 3px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-2);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.toggle-btn.active {
  background: rgba(var(--accent-rgb), 0.14);
  border-color: rgba(var(--accent-rgb), 0.26);
}

.toggle-dot {
  display: block;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--text-primary);
  transform: translateX(0);
  transition: transform 0.15s ease;
}

.toggle-btn.active .toggle-dot {
  transform: translateX(18px);
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 12px;
}

.tool-card {
  padding: 16px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
}

.tool-card:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: var(--surface-2);
  transform: translateY(-1px);
}

.tool-top {
  justify-content: space-between;
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

.tool-short,
.tool-detail,
.tool-runtime {
  margin: 0;
  font-size: 12px;
  line-height: 1.65;
}

.tool-short {
  color: var(--text-primary);
  font-weight: 600;
  margin-bottom: 6px;
}

.tool-detail {
  color: var(--text-secondary);
  margin-bottom: 10px;
}

.tool-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.tool-meta strong {
  color: var(--text-secondary);
  font-family: var(--font-sans);
  font-size: 12px;
}

.tool-runtime {
  color: var(--accent);
}

.pill {
  padding: 5px 10px;
  font-size: 11px;
}

.pill.stat {
  background: rgba(255, 255, 255, 0.03);
}

.pill.ok {
  color: #c8facc;
  border-color: rgba(115, 220, 145, 0.18);
  background: rgba(115, 220, 145, 0.08);
}

.pill.warn {
  color: #ffd6a0;
  border-color: rgba(255, 176, 90, 0.18);
  background: rgba(255, 176, 90, 0.08);
}

.pill.muted {
  opacity: 0.72;
}

.card-actions {
  margin-top: 14px;
  flex-wrap: wrap;
}

.small-btn {
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: var(--surface-2);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
}

.small-btn.danger {
  color: #ffb0b0;
  border-color: rgba(255, 120, 120, 0.18);
}

.small-btn.danger.solid {
  background: rgba(255, 120, 120, 0.1);
}

@media (max-width: 900px) {
  .atomic-view {
    padding: 20px 18px 24px;
  }

  .section-head,
  .head-actions,
  .panel-head,
  .editor-top,
  .editor-actions {
    flex-direction: column;
  }

  .count-group {
    align-items: flex-start;
  }

  .editor-grid {
    grid-template-columns: 1fr;
  }
}
</style>
