<script setup lang="ts">
import { ref } from "vue"

export interface ExecutionInfo {
  action: string
  status: "running" | "success" | "failed"
  input?: string
  output?: string
  error?: string
  duration?: string
}

defineProps<{ exec: ExecutionInfo }>()

const expanded = ref(false)
</script>

<template>
  <div :class="['exec-card', exec.status]">
    <button class="exec-header" @click="expanded = !expanded">
      <span :class="['exec-status-dot', exec.status]" />
      <span class="exec-action truncate">{{ exec.action }}</span>
      <span v-if="exec.duration" class="exec-duration">{{ exec.duration }}</span>
      <span class="exec-chevron">{{ expanded ? "▾" : "▸" }}</span>
    </button>

    <div v-if="expanded" class="exec-body">
      <div v-if="exec.input" class="exec-section">
        <span class="exec-label">输入</span>
        <pre class="exec-code">{{ exec.input }}</pre>
      </div>
      <div v-if="exec.output" class="exec-section">
        <span class="exec-label">输出</span>
        <pre class="exec-code">{{ exec.output }}</pre>
      </div>
      <div v-if="exec.error" class="exec-section error">
        <span class="exec-label">错误</span>
        <pre class="exec-code">{{ exec.error }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.exec-card {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--surface-2);
  margin: 8px 0;
}

.exec-card.success { border-left: 3px solid #22c55e; }
.exec-card.failed { border-left: 3px solid #ef4444; }
.exec-card.running { border-left: 3px solid var(--accent); }

.exec-header {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 12px;
  font-family: var(--font-mono);
  cursor: pointer;
  text-align: left;
}

.exec-header:hover {
  background: var(--hover-bg);
}

.exec-status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.exec-status-dot.success { background: #22c55e; }
.exec-status-dot.failed { background: #ef4444; }
.exec-status-dot.running {
  background: var(--accent);
  animation: pulse 1.2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.exec-action {
  flex: 1;
  color: var(--text-secondary);
}

.exec-duration {
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  font-size: 11px;
}

.exec-chevron {
  color: var(--text-tertiary);
  font-size: 10px;
}

.exec-body {
  border-top: 1px solid var(--border-color);
  padding: 8px 10px;
}

.exec-section {
  margin-bottom: 6px;
}

.exec-section:last-child {
  margin-bottom: 0;
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
  background: var(--surface-1);
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-secondary);
}
</style>
