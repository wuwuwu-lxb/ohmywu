<script setup lang="ts">
import ExecutionCard from "./ExecutionCard.vue"
import type { ExecutionInfo } from "./ExecutionCard.vue"

export interface ChatMsg {
  id: string
  role: "user" | "agent"
  content: string
  agentName?: string
  agentIcon?: string
  execs?: ExecutionInfo[]
  taskId?: string
  timestamp: number
}

defineProps<{ msg: ChatMsg }>()
const emit = defineEmits<{ "show-task": [taskId: string] }>()
</script>

<template>
  <div :class="['msg-row', msg.role]">
    <!-- Agent: icon + name on the left -->
    <template v-if="msg.role === 'agent'">
      <div class="msg-icon">
        <span>{{ msg.agentIcon || "✦" }}</span>
      </div>
      <div class="msg-body">
        <div class="msg-header">
          <span class="msg-sender">{{ msg.agentName || "OhMyWu" }}</span>
          <span class="msg-time">{{ new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
        </div>
        <div class="msg-text">{{ msg.content }}</div>
        <div v-if="msg.execs?.length" class="msg-execs">
          <ExecutionCard v-for="(exec, i) in msg.execs" :key="i" :exec="exec" />
        </div>
        <div v-if="msg.taskId" class="msg-task-link" @click="emit('show-task', msg.taskId!)">
          <span>查看执行链路</span>
          <span class="link-arrow">→</span>
        </div>
      </div>
    </template>

    <!-- User: right-aligned, no avatar -->
    <template v-else>
      <div class="msg-body user-body">
        <div class="msg-header user-header">
          <span class="msg-time">{{ new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
        </div>
        <div class="msg-text user-text">{{ msg.content }}</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.msg-row {
  display: flex;
  gap: 12px;
  padding: 10px 24px;
  max-width: 820px;
  margin: 0 auto;
  width: 100%;
}

.msg-row.user {
  justify-content: flex-end;
}

/* Agent icon */
.msg-icon {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: rgba(var(--accent-rgb), 0.16);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  color: #f6f8ff;
  font-size: 14px;
  margin-top: 2px;
  box-shadow: var(--shadow-glow);
}

/* Message body */
.msg-body {
  flex: 1;
  min-width: 0;
  max-width: 85%;
}

.user-body {
  max-width: 70%;
}

/* Header row */
.msg-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 3px;
}

.user-header {
  justify-content: flex-end;
}

.msg-sender {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
}

.msg-time {
  font-size: 10.5px;
  font-family: var(--font-mono);
  color: var(--text-disabled);
}

/* Text content */
.msg-text {
  font-size: var(--text-base);
  line-height: 1.65;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  padding: 12px 14px;
  border-radius: 16px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-surface);
}

.msg-row.agent .msg-text {
  background: rgba(var(--accent-rgb), 0.07);
  border-color: rgba(var(--accent-rgb), 0.14);
}

.user-text {
  background: rgba(var(--accent-rgb), 0.12);
  border-color: rgba(var(--accent-rgb), 0.22);
  color: var(--text-primary);
  box-shadow: 0 12px 28px rgba(0, 0, 0, 0.14);
}

/* Executions */
.msg-execs {
  margin-top: 8px;
}

/* Task link */
.msg-task-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border-color);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.msg-task-link:hover {
  border-color: rgba(var(--accent-rgb), 0.22);
  color: var(--text-primary);
  background: rgba(var(--accent-rgb), 0.08);
}

.link-arrow {
  color: var(--accent);
}
</style>
