<script setup lang="ts">
import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { getToolMeta, toolRiskLabel, type CapabilityInfo, type ToolRisk } from "../lib/tools"
import type { AgentMode } from "../stores/chat"

type PolicyMode = "Sandbox" | "Danger"

interface AppConfig {
  policy_mode: PolicyMode
  agent_mode: AgentMode
}

interface ToolCard extends CapabilityInfo {
  label: string
  short: string
  detail: string
  example?: string
  visible: boolean
  blockedByPolicy: boolean
  runtimeHint: string
}

const capabilities = ref<CapabilityInfo[]>([])
const loading = ref(false)
const refreshMsg = ref("")
const policyMode = ref<PolicyMode>("Sandbox")
const selectedAgentMode = ref<AgentMode>("agent")

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

function isToolVisible(name: string, risk: ToolRisk, mode: AgentMode): boolean {
  if (name === "checklist_write") {
    return true
  }
  return mode === "plan" ? risk === "ReadOnly" : true
}

function runtimeHintFor(
  risk: ToolRisk,
  visible: boolean,
  blockedByPolicy: boolean,
  mode: AgentMode
): string {
  if (!visible) {
    return "当前模式下隐藏，不会暴露给模型。"
  }
  if (blockedByPolicy) {
    return "当前会被策略层直接拦截，权限规则也不会生效。"
  }
  if (risk === "HighRisk") {
    return mode === "auto"
      ? "当前会直接执行，高风险不再二次确认。"
      : "当前会要求确认，避免模型直接执行高风险操作。"
  }
  if (risk === "ControlledWrite") {
    return "当前可执行，但仍受 allow/deny 规则限制。"
  }
  return "当前可直接执行，适合检索、读取和分析。"
}

const capabilityCards = computed<ToolCard[]>(() =>
  [...capabilities.value]
    .sort((a, b) => riskWeight(a.risk_level) - riskWeight(b.risk_level) || a.name.localeCompare(b.name))
    .map((cap) => {
      const meta = getToolMeta(cap.name)
      const visible = isToolVisible(cap.name, cap.risk_level, selectedAgentMode.value)
      const blockedByPolicy = policyMode.value === "Sandbox" && cap.risk_level !== "ReadOnly"
      return {
        ...cap,
        label: meta.label,
        short: meta.short,
        detail: meta.detail,
        example: meta.example,
        visible,
        blockedByPolicy,
        runtimeHint: runtimeHintFor(
          cap.risk_level,
          visible,
          blockedByPolicy,
          selectedAgentMode.value
        ),
      }
    })
)

const visibleToolCount = computed(() => capabilityCards.value.filter((tool) => tool.visible).length)
const blockedToolCount = computed(() => capabilityCards.value.filter((tool) => tool.blockedByPolicy).length)

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
    refreshMsg.value = "已同步当前策略模式、Agent Mode 和工具清单。"
  }
}

onMounted(loadCapabilities)
</script>

<template>
  <div class="atomic-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">原子化能力</h2>
        <p class="view-subtitle">
          单独查看当前工具暴露、风险分级和策略状态。设置页只负责配置，这里负责解释系统到底能做什么。
        </p>
      </div>
      <div class="head-actions">
        <div class="count-group">
          <span class="section-count">{{ capabilityCards.length }}</span>
          <span class="sub-count">visible {{ visibleToolCount }} · blocked {{ blockedToolCount }}</span>
        </div>
        <button class="refresh-btn" :disabled="loading" @click="refreshCapabilities">
          {{ loading ? "同步中" : "同步能力视图" }}
        </button>
      </div>
    </header>

    <section class="panel">
      <div class="panel-head">
        <div>
          <h3 class="panel-title">工具暴露</h3>
          <p class="panel-subtitle">每个工具的用途、风险等级、当前是否对模型可见，以及实际执行时会发生什么。</p>
        </div>
        <div class="tool-stats">
          <span class="pill stat">{{ capabilityCards.length }} total</span>
          <span class="pill stat ok">{{ visibleToolCount }} visible</span>
          <span class="pill stat warn">{{ blockedToolCount }} blocked</span>
        </div>
      </div>

      <div class="status-banner">
        <span class="status-dot" />
        <span>{{ executionSummary }}</span>
      </div>

      <div v-if="refreshMsg" class="refresh-msg">{{ refreshMsg }}</div>

      <div class="tool-grid">
        <article v-for="tool in capabilityCards" :key="tool.name" class="tool-card">
          <div class="tool-top">
            <div>
              <div class="tool-label">{{ tool.label }}</div>
              <div class="tool-name">{{ tool.name }}</div>
            </div>
            <div class="tool-tags">
              <span class="pill risk">{{ toolRiskLabel(tool.risk_level) }}</span>
              <span :class="['pill', tool.visible ? 'ok' : 'muted']">{{ tool.visible ? "已暴露" : "已隐藏" }}</span>
              <span v-if="tool.blockedByPolicy" class="pill warn">策略拦截</span>
            </div>
          </div>

          <p class="tool-short">{{ tool.short }}</p>
          <p class="tool-detail">{{ tool.detail }}</p>
          <p class="tool-runtime">{{ tool.runtimeHint }}</p>
          <p v-if="tool.example" class="tool-example">示例：{{ tool.example }}</p>
        </article>
      </div>
    </section>
  </div>
</template>

<style scoped>
.atomic-view {
  padding: 28px 32px 32px;
  max-width: 1080px;
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
.panel-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.section-head,
.panel-head {
  justify-content: space-between;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.view-subtitle,
.panel-subtitle {
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

.refresh-btn {
  padding: 8px 12px;
  font-size: 11px;
  cursor: pointer;
}

.refresh-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.panel {
  padding: 22px 24px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.panel-head {
  margin-bottom: 16px;
}

.panel-title {
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
  margin-bottom: 16px;
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

.refresh-msg {
  margin-bottom: 14px;
  padding: 10px 12px;
  border-radius: 14px;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  color: var(--text-secondary);
  font-size: 12px;
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 12px;
}

.tool-card {
  padding: 16px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: rgba(var(--accent-rgb), 0.03);
  box-shadow: var(--shadow-surface);
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
}

.tool-card:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.06);
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
.tool-runtime,
.tool-example {
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
  margin-bottom: 8px;
}

.tool-runtime {
  color: var(--accent);
  margin-bottom: 8px;
}

.tool-example {
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  word-break: break-word;
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

@media (max-width: 800px) {
  .atomic-view {
    padding: 20px 18px 24px;
  }

  .section-head,
  .head-actions,
  .panel-head {
    flex-direction: column;
  }

  .count-group {
    align-items: flex-start;
  }
}
</style>
