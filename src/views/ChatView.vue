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
        <div class="empty-stage">
          <section class="empty-main-card">
            <div class="empty-cover">
              <div class="empty-cover-mark">✦</div>
            </div>
            <div class="empty-copy">
              <div class="empty-title">OhMyWu</div>
              <div class="empty-desc">本地优先的桌面 Agent 工作台。你可以直接对话、读写本地内容、接入模型，或者把它当成一套可控执行面板。</div>
              <div class="empty-meta">
                <span class="meta-pill">Local-first</span>
                <span class="meta-pill">Tool Calling</span>
                <span class="meta-pill">Auditable</span>
              </div>
            </div>
          </section>

          <section class="empty-side-card">
            <div class="side-title">开始方式</div>
            <button class="hint" @click="input = '帮我看看当前目录有什么文件'">帮我看看当前目录有什么文件</button>
            <button class="hint" @click="input = '读取 README.md 并总结项目结构'">读取 README.md 并总结项目结构</button>
            <button class="hint" @click="input = '检查我现在配置的模型是否可用'">检查我现在配置的模型是否可用</button>
          </section>
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
        <button class="stop-btn" @click="store.cancelAgent" title="停止">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect width="12" height="12" rx="2" fill="currentColor" />
          </svg>
        </button>
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
        <button class="stop-btn" @click="store.cancelAgent" title="停止">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect width="12" height="12" rx="2" fill="currentColor" />
          </svg>
        </button>
      </div>
    </div>

    <!-- input bar -->
    <div class="input-bar">
      <div class="composer-meta">
        <div class="composer-cover">✦</div>
        <div class="composer-copy">
          <div class="composer-title">OhMyWu</div>
          <div class="composer-subtitle">{{ store.pending ? "Thinking" : "Local desktop agent workspace" }}</div>
        </div>
      </div>

      <div class="input-wrapper">
        <textarea
          v-model="input"
          class="chat-input"
          rows="1"
          placeholder="输入消息，Enter 发送 · Shift+Enter 换行"
          :disabled="store.pending"
          @keydown="handleKeydown"
        />
      </div>

      <div class="composer-actions">
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
  padding: 10px 20px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  background: var(--shell-bg-soft);
  backdrop-filter: blur(calc(var(--shell-blur) * 0.4));
  -webkit-backdrop-filter: blur(calc(var(--shell-blur) * 0.4));
}

.session-select {
  flex: 1;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font);
  font-weight: 500;
  outline: none;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  appearance: none;
  -webkit-appearance: none;
  color-scheme: dark;
}

.session-select:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.session-select:focus {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: var(--surface-2);
}

.session-select option {
  background: #10141b;
  color: var(--text-primary);
}

.session-btn {
  width: 32px;
  height: 32px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--duration-fast) var(--ease-out);
}

.session-btn:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.18);
}

/* ── Messages Area ────────────────────────────────────────────── */
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 24px 0 18px;
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
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
}

.empty-stage {
  width: min(920px, 100%);
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) minmax(240px, 0.8fr);
  gap: 18px;
}

.empty-main-card,
.empty-side-card {
  border: 1px solid var(--border-color);
  border-radius: 24px;
  background: var(--surface-1);
  box-shadow: var(--shadow-float);
}

.empty-main-card {
  display: grid;
  grid-template-columns: 184px minmax(0, 1fr);
  gap: 20px;
  align-items: center;
  padding: 22px;
}

.empty-cover {
  aspect-ratio: 1;
  border-radius: 22px;
  background: rgba(var(--accent-rgb), 0.16);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-glow);
}

.empty-cover-mark {
  width: 78px;
  height: 78px;
  border-radius: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.16);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  color: #fff;
  font-size: 28px;
}

.empty-copy {
  min-width: 0;
}

.empty-title {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.empty-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
  margin-bottom: 18px;
}

.empty-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.meta-pill {
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
}

.empty-side-card {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.side-title {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-tertiary);
  margin-bottom: 2px;
}

.hint {
  width: 100%;
  text-align: left;
  padding: 8px 14px;
  border-radius: 14px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.hint:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
  background: rgba(var(--accent-rgb), 0.08);
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
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid var(--border-color);
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.16);
  box-shadow: var(--shadow-surface);
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

/* ── Stop Button ──────────────────────────────────────────────── */
.stop-btn {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-tertiary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-top: 3px;
  transition: all var(--duration-fast) var(--ease-out);
}

.stop-btn:hover {
  background: var(--danger-soft, rgba(239, 68, 68, 0.12));
  color: var(--danger, #ef4444);
  border-color: var(--danger, #ef4444);
}

/* ── Input Bar ────────────────────────────────────────────────── */
.input-bar {
  min-height: 80px;
  padding: 0 8px 0 16px;
  flex-shrink: 0;
  border-top: 1px solid var(--border-color);
  background: var(--shell-bg-soft);
  display: grid;
  grid-template-columns: minmax(180px, 260px) minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
}

.composer-meta {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
}

.composer-cover {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(var(--accent-rgb), 0.14);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  box-shadow: var(--shadow-glow);
  color: #fff;
  font-size: 18px;
}

.composer-copy {
  min-width: 0;
}

.composer-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.composer-subtitle {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.input-wrapper {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0;
  border: none;
  background: transparent;
  box-shadow: none;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  transition: none;
}

.input-wrapper:focus-within {
  transform: none;
}

.chat-input {
  flex: 1;
  resize: none;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-primary);
  font-size: var(--text-base);
  font-family: var(--font);
  line-height: 1.55;
  outline: none;
  padding: 12px 16px;
  border-radius: 14px;
  max-height: 58px;
  min-height: 58px;
  transition: border-color 0.15s ease, background 0.15s ease, box-shadow 0.15s ease;
}

.chat-input:focus {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: var(--surface-2);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
}

.chat-input::placeholder {
  color: var(--text-disabled);
}

.chat-input:disabled {
  opacity: 0.5;
}

.composer-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
}

.send-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
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

@media (max-width: 900px) {
  .empty-stage {
    grid-template-columns: 1fr;
  }

  .empty-main-card {
    grid-template-columns: 1fr;
  }

  .empty-cover {
    max-width: 220px;
  }

  .input-bar {
    grid-template-columns: 1fr;
    gap: 12px;
    padding: 16px;
  }

  .composer-actions {
    justify-content: flex-start;
  }
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
