<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue"
import ChatMessage from "../components/ChatMessage.vue"
import type { ChatMsg } from "../components/ChatMessage.vue"
import type { ExecutionInfo } from "../components/ExecutionCard.vue"

const messages = ref<ChatMsg[]>([])
const input = ref("")
const pending = ref(false)
const chatEl = ref<HTMLElement>()
let msgId = 0

const send = async () => {
  const text = input.value.trim()
  if (!text || pending.value) return

  messages.value.push({
    id: `msg-${++msgId}`,
    role: "user",
    content: text,
    timestamp: Date.now(),
  })
  input.value = ""
  pending.value = true
  scroll()

  // mock exec + response
  const execs: ExecutionInfo[] = [
    { action: "shell.exec", status: "success", input: text, output: `[模拟输出] ${text}`, duration: "0.3s" },
  ]

  await new Promise((r) => setTimeout(r, 800))

  messages.value.push({
    id: `msg-${++msgId}`,
    role: "agent",
    content: `已执行操作：${text}`,
    agentName: "OhMyWu",
    agentIcon: "🤖",
    execs,
    taskId: `task-${msgId}`,
    timestamp: Date.now(),
  })
  pending.value = false
  scroll()
}

const scroll = async () => {
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}

onMounted(() => {
  messages.value.push({
    id: "msg-0",
    role: "agent",
    content: "你好，我是 OhMyWu。有什么可以帮你的？",
    agentName: "OhMyWu",
    agentIcon: "🤖",
    timestamp: Date.now(),
  })
})
</script>

<template>
  <div class="chat-view">
    <div class="chat-messages" ref="chatEl">
      <ChatMessage
        v-for="msg in messages"
        :key="msg.id"
        :msg="msg"
      />
      <div v-if="pending" class="thinking-indicator">
        <span class="dot" />
        <span class="dot" />
        <span class="dot" />
      </div>
    </div>

    <div class="chat-input-bar">
      <textarea
        v-model="input"
        class="chat-input"
        rows="1"
        placeholder="输入消息，Enter 发送"
        :disabled="pending"
        @keydown="handleKeydown"
      />
      <button
        class="chat-send"
        :disabled="pending || !input.trim()"
        @click="send"
      >
        <span v-if="!pending">↵</span>
        <span v-else class="loading-spinner" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.thinking-indicator {
  display: flex;
  gap: 4px;
  padding: 12px 16px;
  max-width: 720px;
  margin: 0 auto;
  width: 100%;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-tertiary);
  animation: bounce 1.4s infinite ease-in-out;
}

.dot:nth-child(2) { animation-delay: 0.2s; }
.dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes bounce {
  0%, 80%, 100% { transform: scale(0.6); opacity: 0.4; }
  40% { transform: scale(1); opacity: 1; }
}

.chat-input-bar {
  display: flex;
  gap: 8px;
  padding: 12px 16px 16px;
  max-width: 720px;
  margin: 0 auto;
  width: 100%;
}

.chat-input {
  flex: 1;
  resize: none;
  padding: 10px 14px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  outline: none;
  line-height: 1.5;
}

.chat-input:focus {
  border-color: var(--accent);
}

.chat-input::placeholder {
  color: var(--text-tertiary);
}

.chat-send {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: none;
  background: var(--accent);
  color: var(--text-on-accent);
  font-size: 16px;
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  align-self: flex-end;
}

.chat-send:disabled {
  opacity: 0.3;
  cursor: default;
}

.chat-send:hover:not(:disabled) {
  opacity: 0.9;
}

.loading-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid transparent;
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
