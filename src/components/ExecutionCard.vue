<script setup lang="ts">
import { computed, ref } from "vue"
import { getToolMeta, toolStatusLabel } from "../lib/tools"

export interface ExecutionInfo {
  action: string
  status: "running" | "success" | "failed" | "denied" | "needs_confirm"
  input?: string
  output?: string
  artifactPath?: string
  error?: string
  duration?: string
  delegated?: {
    agentId: string
    agentName: string
    role?: string
    task?: string
    content?: string
    reasoningContent?: string | null
    executionCount?: number
    executions: ExecutionInfo[]
  } | null
}

const props = defineProps<{ exec: ExecutionInfo }>()

const expanded = ref(false)
const meta = computed(() => getToolMeta(props.exec.action))
const statusText = computed(() => toolStatusLabel(props.exec.status))
const executionStateText = computed(() => {
  if (props.exec.status === "success" || props.exec.status === "failed") return "已执行"
  if (props.exec.status === "denied" || props.exec.status === "needs_confirm") return "未执行"
  return "状态未知"
})
</script>

<template>
  <div :class="['exec-card', exec.status]">
    <button class="exec-header" @click="expanded = !expanded">
      <span :class="['exec-status-dot', exec.status]" />
      <span class="exec-copy">
        <span class="exec-action truncate">{{ meta.label }}</span>
        <span class="exec-action-sub">{{ exec.action }} · {{ statusText }} · {{ executionStateText }}</span>
      </span>
      <span v-if="exec.duration" class="exec-duration">{{ exec.duration }}</span>
      <span v-if="exec.artifactPath" class="exec-badge">artifact</span>
      <span class="exec-chevron">{{ expanded ? "▾" : "▸" }}</span>
    </button>

    <div v-if="expanded" class="exec-body">
      <div class="exec-summary">{{ meta.detail }}</div>
      <div v-if="exec.input" class="exec-section">
        <span class="exec-label">输入</span>
        <pre class="exec-code">{{ exec.input }}</pre>
      </div>
      <div v-if="exec.output" class="exec-section">
        <span class="exec-label">输出</span>
        <pre class="exec-code">{{ exec.output }}</pre>
      </div>
      <div v-if="exec.artifactPath" class="exec-section">
        <span class="exec-label">完整输出</span>
        <div class="exec-artifact-note">大结果已落到本地 artifact，需要时可再次读取这个路径。</div>
        <pre class="exec-code exec-artifact-path">{{ exec.artifactPath }}</pre>
      </div>
      <div v-if="exec.error" class="exec-section error">
        <span class="exec-label">错误</span>
        <pre class="exec-code">{{ exec.error }}</pre>
      </div>
      <div v-if="exec.delegated" class="delegate-block">
        <div class="delegate-head">
          <div class="delegate-title-row">
            <span class="delegate-title">{{ exec.delegated.agentName || "子 Agent" }}</span>
            <span class="delegate-pill">{{ exec.delegated.agentId }}</span>
            <span v-if="exec.delegated.executionCount != null" class="delegate-pill">
              {{ exec.delegated.executionCount }} 个子工具
            </span>
          </div>
          <div v-if="exec.delegated.role" class="delegate-role">{{ exec.delegated.role }}</div>
        </div>
        <div v-if="exec.delegated.task" class="exec-section">
          <span class="exec-label">委派任务</span>
          <pre class="exec-code">{{ exec.delegated.task }}</pre>
        </div>
        <div v-if="exec.delegated.content" class="exec-section">
          <span class="exec-label">子 Agent 输出</span>
          <pre class="exec-code">{{ exec.delegated.content }}</pre>
        </div>
        <div v-if="exec.delegated.executions.length" class="delegate-tools">
          <span class="exec-label">子 Agent 工具链</span>
          <ExecutionCard
            v-for="(childExec, index) in exec.delegated.executions"
            :key="`${exec.delegated.agentId}-${index}`"
            :exec="childExec"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.exec-card {
  border: 1px solid var(--border-color);
  border-radius: 14px;
  overflow: hidden;
  background: var(--surface-1);
  margin: 10px 0;
  box-shadow: var(--shadow-surface);
}

.exec-card.success { border-left: 3px solid #22c55e; }
.exec-card.failed { border-left: 3px solid #ef4444; }
.exec-card.running { border-left: 3px solid var(--accent); }
.exec-card.denied { border-left: 3px solid #f59e0b; }
.exec-card.needs_confirm { border-left: 3px solid #38bdf8; }

.exec-header {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 12px;
  font-family: var(--font-mono);
  cursor: pointer;
  text-align: left;
}

.exec-header:hover {
  background: var(--surface-2);
}

.exec-status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.exec-status-dot.success { background: #22c55e; }
.exec-status-dot.failed { background: #ef4444; }
.exec-status-dot.denied { background: #f59e0b; }
.exec-status-dot.needs_confirm { background: #38bdf8; }
.exec-status-dot.running {
  background: var(--accent);
  animation: pulse 1.2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.exec-action {
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
}

.exec-copy {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.exec-action-sub {
  color: var(--text-tertiary);
  font-size: 10px;
  font-family: var(--font-mono);
}

.exec-duration {
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  font-size: 11px;
}

.exec-badge {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-tertiary);
  font-size: 10px;
  line-height: 1;
  text-transform: uppercase;
}

.exec-chevron {
  color: var(--text-tertiary);
  font-size: 10px;
}

.exec-body {
  border-top: 1px solid var(--border-color);
  padding: 10px 12px;
}

.exec-summary {
  margin-bottom: 8px;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-secondary);
}

.exec-section {
  margin-bottom: 6px;
}

.exec-section:last-child {
  margin-bottom: 0;
}

.exec-artifact-note {
  margin-bottom: 4px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.exec-artifact-path {
  word-break: break-all;
}

.delegate-block {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed rgba(var(--accent-rgb), 0.18);
}

.delegate-head {
  margin-bottom: 8px;
}

.delegate-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.delegate-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.delegate-role {
  margin-top: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.55;
}

.delegate-pill {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-tertiary);
  font-size: 10px;
  font-family: var(--font-mono);
}

.delegate-tools {
  margin-top: 8px;
}

.exec-label {
  display: block;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-tertiary);
  margin-bottom: 4px;
  letter-spacing: 0.5px;
}

.exec-section.error .exec-label {
  color: #ef4444;
}

.exec-code {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  background: var(--surface-2);
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-secondary);
}
</style>
