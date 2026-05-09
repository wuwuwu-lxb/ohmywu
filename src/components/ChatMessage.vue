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
    <div class="msg-avatar" v-if="msg.role === 'agent'">
      <span class="avatar-icon">{{ msg.agentIcon || "🤖" }}</span>
    </div>

    <div class="msg-content">
      <div class="msg-meta">
        <span class="msg-agent" v-if="msg.agentName">{{ msg.agentName }}</span>
        <span class="msg-time">{{ new Date(msg.timestamp).toLocaleTimeString() }}</span>
      </div>

      <div class="msg-bubble">
        <p class="msg-text">{{ msg.content }}</p>
      </div>

      <div v-if="msg.execs?.length" class="msg-execs">
        <ExecutionCard
          v-for="(exec, i) in msg.execs"
          :key="i"
          :exec="exec"
        />
      </div>

      <div v-if="msg.taskId" class="msg-task-link" @click="emit('show-task', msg.taskId!)">
        查看执行链路 ▸
      </div>
    </div>

    <div class="msg-avatar user-avatar" v-if="msg.role === 'user'">
      <span class="avatar-icon">🧑</span>
    </div>
  </div>
</template>

<style scoped>
.msg-row {
  display: flex;
  gap: 10px;
  padding: 0 16px;
  max-width: 720px;
  margin: 0 auto;
  width: 100%;
}

.msg-row.user {
  flex-direction: row-reverse;
}

.msg-avatar {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  margin-top: 2px;
}

.avatar-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  font-size: 16px;
  border-radius: 50%;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
}

.user-avatar .avatar-icon {
  background: var(--accent);
  border-color: var(--accent);
}

.msg-content {
  flex: 1;
  min-width: 0;
}

.msg-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.msg-agent {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.msg-time {
  font-size: 11px;
  color: var(--text-tertiary);
}

.msg-bubble {
  padding: 10px 14px;
  border-radius: var(--radius-lg);
  font-size: 14px;
  line-height: 1.6;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
}

.msg-row.user .msg-bubble {
  background: color-mix(in srgb, var(--accent) 20%, var(--bg-surface));
  border-color: color-mix(in srgb, var(--accent) 30%, transparent);
}

.msg-text {
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

.msg-row.user .msg-text {
  color: var(--text-primary);
}

.msg-execs {
  margin-top: 6px;
}

.msg-task-link {
  margin-top: 6px;
  font-size: 12px;
  color: var(--accent);
  cursor: pointer;
  opacity: 0.7;
}

.msg-task-link:hover {
  opacity: 1;
}
</style>
