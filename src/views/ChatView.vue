<script setup lang="ts">
import { ref, nextTick, onMounted, watch } from "vue"
import ChatMessage from "../components/ChatMessage.vue"
import { useChatStore } from "../stores/chat"

const store = useChatStore()
const input = ref("")
const chatEl = ref<HTMLElement>()

const emit = defineEmits<{
  "show-task": [taskId: string]
}>()

const send = async () => {
  const text = input.value.trim()
  if (!text || store.pending) return
  input.value = ""
  await store.sendMessage(text)
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

const selectSession = async (id: string) => {
  await store.loadSession(id)
  scroll()
}

const newSession = async () => {
  const name = `对话 ${store.sessions.length + 1}`
  await store.createSession(name)
}

watch(() => store.messages.length, () => scroll())
watch(() => store.streamingContent, () => scroll())

onMounted(async () => {
  await store.init()
  scroll()
})
</script>

<template>
  <div class="chat-view">
    <!-- session bar -->
    <div class="session-bar">
      <select
        class="session-select"
        :value="store.currentSessionId"
        @change="selectSession(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="s in store.sessions" :key="s.id" :value="s.id">
          {{ s.name }}
        </option>
      </select>
      <button class="session-btn" @click="newSession" title="新建对话">
        <span>+</span>
      </button>
    </div>

    <!-- messages area -->
    <div class="chat-messages" ref="chatEl">
      <!-- empty state -->
      <div v-if="!store.messages.length && !store.pending" class="empty-state">
        <div class="empty-icon">✦</div>
        <div class="empty-title">OhMyWu</div>
        <div class="empty-desc">你的桌面 AI 助手。可以直接对话，也可以执行命令。</div>
        <div class="empty-hints">
          <span class="hint">试试问：帮我看看当前目录有什么文件</span>
        </div>
      </div>

      <!-- messages -->
      <ChatMessage
        v-for="(msg, i) in store.messages"
        :key="msg.id"
        :msg="msg"
        :style="{ animationDelay: `${i * 15}ms` }"
        class="msg-animate"
        @show-task="(taskId: string) => emit('show-task', taskId)"
      />

      <!-- streaming -->
      <div v-if="store.streamingContent" class="streaming-row">
        <div class="msg-icon">
          <span>✦</span>
        </div>
        <div class="msg-body">
          <div class="msg-header">
            <span class="msg-sender">OhMyWu</span>
          </div>
          <div class="streaming-text">
            {{ store.streamingContent }}<span class="cursor-blink">|</span>
          </div>
        </div>
      </div>

      <!-- waiting for first token -->
      <div v-else-if="store.pending && !store.streamingContent" class="thinking-row">
        <div class="msg-icon">
          <span>✦</span>
        </div>
        <div class="thinking-dots">
          <span class="dot" />
          <span class="dot" />
          <span class="dot" />
        </div>
      </div>
    </div>

    <!-- input bar -->
    <div class="input-bar">
      <div class="input-wrapper">
        <textarea
          v-model="input"
          class="chat-input"
          rows="1"
          placeholder="输入消息，Enter 发送 · Shift+Enter 换行"
          :disabled="store.pending"
          @keydown="handleKeydown"
        />
        <button
          class="send-btn"
          :class="{ pending: store.pending }"
          :disabled="store.pending || !input.trim()"
          @click="send"
        >
          <svg v-if="!store.pending" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M2 8L14 2L8 14L6 9L2 8Z" fill="currentColor" />
          </svg>
          <span v-else class="spinner" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

/* ── Session Bar ──────────────────────────────────────────────── */
.session-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  background: var(--surface-2);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}

.session-select {
  flex: 1;
  padding: 5px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-family: var(--font);
  font-weight: 500;
  outline: none;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.session-select:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
}

.session-select:focus {
  border-color: var(--border-hover);
  background: var(--surface-2);
}

.session-btn {
  width: 26px;
  height: 26px;
  border-radius: var(--radius-sm);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--duration-fast) var(--ease-out);
}

.session-btn:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
  border-color: var(--border-color);
}

/* ── Messages Area ────────────────────────────────────────────── */
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px 0;
  display: flex;
  flex-direction: column;
}

.msg-animate {
  animation: fadeUp 0.3s var(--ease-out) both;
}

@keyframes fadeUp {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ── Empty State ──────────────────────────────────────────────── */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
}

.empty-icon {
  width: 52px;
  height: 52px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-lg);
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 22px;
  margin-bottom: 16px;
}

.empty-title {
  font-size: var(--text-lg);
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.empty-desc {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  max-width: 320px;
  line-height: 1.6;
  margin-bottom: 20px;
}

.empty-hints {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}

.hint {
  padding: 6px 12px;
  border-radius: var(--radius-md);
  background: var(--surface-2);
  border: 1px solid var(--border-color);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.hint:hover {
  border-color: var(--border-hover);
  color: var(--text-primary);
  background: var(--hover-bg);
}

/* ── Streaming ────────────────────────────────────────────────── */
.streaming-row {
  display: flex;
  gap: 12px;
  padding: 8px 20px;
  max-width: 760px;
  margin: 0 auto;
  width: 100%;
}

.msg-icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 14px;
  margin-top: 2px;
}

.msg-body {
  flex: 1;
  min-width: 0;
}

.msg-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 3px;
}

.msg-sender {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
}

.streaming-text {
  font-size: var(--text-base);
  line-height: 1.65;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}

.cursor-blink {
  color: var(--accent);
  animation: blink 0.8s infinite;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

/* ── Thinking ─────────────────────────────────────────────────── */
.thinking-row {
  display: flex;
  gap: 12px;
  padding: 8px 20px;
  max-width: 760px;
  margin: 0 auto;
  width: 100%;
}

.thinking-dots {
  display: flex;
  gap: 5px;
  align-items: center;
  padding-top: 4px;
}

.dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text-disabled);
  animation: bounce 1.4s infinite ease-in-out;
}

.dot:nth-child(2) { animation-delay: 0.2s; }
.dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes bounce {
  0%, 80%, 100% { transform: scale(0.5); opacity: 0.3; }
  40% { transform: scale(1); opacity: 1; }
}

/* ── Input Bar ────────────────────────────────────────────────── */
.input-bar {
  padding: 12px 20px 16px;
  flex-shrink: 0;
}

.input-wrapper {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  max-width: 740px;
  margin: 0 auto;
  padding: 6px 6px 6px 16px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-color);
  background: var(--surface-2);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out);
}

.input-wrapper:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.chat-input {
  flex: 1;
  resize: none;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: var(--text-base);
  font-family: var(--font);
  line-height: 1.55;
  outline: none;
  padding: 4px 0;
  max-height: 120px;
}

.chat-input::placeholder {
  color: var(--text-disabled);
}

.chat-input:disabled {
  opacity: 0.5;
}

.send-btn {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  border: none;
  background: var(--accent);
  color: var(--text-on-accent);
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--duration-fast) var(--ease-out);
}

.send-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 90%, white);
  transform: scale(1.05);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: default;
}

.send-btn.pending {
  background: var(--hover-bg);
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid transparent;
  border-top-color: var(--text-tertiary);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
