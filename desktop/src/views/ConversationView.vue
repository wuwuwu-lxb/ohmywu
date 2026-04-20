<template>
  <div class="conversation">
    <!-- Header -->
    <header class="page-header">
      <div>
        <h1 class="page-title">对话</h1>
        <p class="page-subtitle">@agent · /action · 自然语言 · 手动拆任务</p>
      </div>
      <div class="mode-badge">
        <svg width="8" height="8" viewBox="0 0 8 8">
          <circle cx="4" cy="4" r="3" fill="#7EC8A0"/>
        </svg>
        User Mode
      </div>
    </header>

    <!-- Messages -->
    <div class="messages-container" ref="containerRef">
      <div v-if="messages.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
            <circle cx="20" cy="20" r="18" stroke="#C9A96E" stroke-width="1.5" opacity="0.4"/>
            <path d="M14 20C14 16.5, 16.5 14, 20 14C23.5 14, 26 16.5, 26 20" stroke="#C9A96E" stroke-width="1.5" stroke-linecap="round" fill="none" opacity="0.6"/>
            <circle cx="20" cy="24" r="2" fill="#C9A96E" opacity="0.5"/>
          </svg>
        </div>
        <p class="empty-title">开始一段对话</p>
        <p class="empty-hint">输入自然语言，或使用 <code>@agent</code> 和 <code>/action</code> 引用</p>
      </div>

      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="message-wrap"
        :class="msg.role"
        :style="{ animationDelay: `${i * 0.03}s` }"
      >
        <div class="message-avatar">
          <span v-if="msg.role === 'user'">
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <circle cx="9" cy="6" r="3" stroke="#C9A96E" stroke-width="1.2"/>
              <path d="M3 15c0-3 2.5-5 6-5s6 2 6 5" stroke="#C9A96E" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
          </span>
          <span v-else>
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <circle cx="9" cy="9" r="7.5" stroke="#C9A96E" stroke-width="1.2"/>
              <path d="M6 9C6 7 7.5 5.5 9 5.5C10.5 5.5 12 7 12 9" stroke="#C9A96E" stroke-width="1.2" stroke-linecap="round" fill="none"/>
              <circle cx="9" cy="11.5" r="1.2" fill="#C9A96E" opacity="0.5"/>
            </svg>
          </span>
        </div>
        <div class="message-bubble">
          <div class="message-content">{{ msg.content }}</div>
          <div class="message-time">{{ msg.time }}</div>
        </div>
      </div>
    </div>

    <!-- Input -->
    <div class="input-area">
      <div class="input-hints">
        <span class="hint-tag">@agent</span>
        <span class="hint-tag">/action</span>
        <span class="hint-text">显式引用已就绪</span>
      </div>
      <div class="input-row">
        <textarea
          v-model="input"
          placeholder="输入自然语言或命令..."
          class="input-box"
          @keydown.enter.exact.prevent="send"
          rows="1"
        ></textarea>
        <button class="send-btn" @click="send" :disabled="!input.trim()">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M12.5 7L1.5 1.5M12.5 7L7 12.5M12.5 7L6 1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'

const input = ref('')
const containerRef = ref<HTMLElement>()
const messages = ref<Array<{ role: string; content: string; time: string }>>([])

function send() {
  if (!input.value.trim()) return
  const now = new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  messages.value.push({ role: 'user', content: input.value, time: now })
  input.value = ''
  nextTick(() => {
    containerRef.value?.scrollTo({ top: containerRef.value.scrollHeight, behavior: 'smooth' })
  })
}
</script>

<style scoped>
.conversation {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 40px 48px 24px;
  max-width: 900px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 32px;
  animation: fadeSlideIn 0.5s ease both;
}

.page-title {
  font-size: 32px;
  font-weight: 600;
  letter-spacing: -0.02em;
  font-family: var(--font-display);
}

.page-subtitle {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 5px;
  font-family: monospace;
}

.mode-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  background: rgba(126,200,160,0.1);
  border: 1px solid rgba(126,200,160,0.2);
  border-radius: 20px;
  font-size: 12px;
  color: #5AAD7E;
  font-weight: 500;
}

/* Messages */
.messages-container {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding-bottom: 16px;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 0;
}

.empty-icon { opacity: 0.8; }

.empty-title {
  font-size: 16px;
  font-weight: 500;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.empty-hint {
  font-size: 13px;
  color: var(--text-muted);
}

.empty-hint code {
  font-family: monospace;
  background: var(--bg-secondary);
  padding: 1px 5px;
  border-radius: 3px;
  color: var(--accent);
}

/* Message */
.message-wrap {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  animation: fadeSlideIn 0.3s ease both;
}

.message-wrap.user {
  flex-direction: row-reverse;
}

.message-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 2px;
}

.message-bubble {
  max-width: 68%;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.message-content {
  padding: 12px 16px;
  border-radius: 16px;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.message-wrap.user .message-content {
  background: var(--bg-dark);
  color: var(--text-on-dark);
  border-bottom-right-radius: 4px;
}

.message-wrap.assistant .message-content {
  background: var(--bg-card);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-bottom-left-radius: 4px;
}

.message-time {
  font-size: 11px;
  color: var(--text-muted);
  padding: 0 4px;
}

.message-wrap.user .message-time {
  text-align: right;
}

/* Input */
.input-area {
  padding-top: 16px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.input-hints {
  display: flex;
  align-items: center;
  gap: 8px;
}

.hint-tag {
  font-size: 11px;
  font-family: monospace;
  color: var(--accent);
  background: var(--accent-subtle);
  padding: 2px 8px;
  border-radius: 4px;
}

.hint-text {
  font-size: 11px;
  color: var(--text-muted);
}

.input-row {
  display: flex;
  gap: 10px;
  align-items: flex-end;
}

.input-box {
  flex: 1;
  padding: 12px 16px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  font-size: 14px;
  font-family: var(--font-body);
  resize: none;
  background: var(--bg-card);
  color: var(--text-primary);
  transition: border-color var(--transition-fast);
  min-height: 48px;
  max-height: 120px;
  line-height: 1.5;
}

.input-box:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-subtle);
}

.input-box::placeholder {
  color: var(--text-muted);
}

.send-btn {
  width: 44px;
  height: 44px;
  background: var(--bg-dark);
  color: var(--text-on-dark);
  border: none;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  transition: all var(--transition-fast);
}

.send-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
  transform: translateY(-1px);
}

.send-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
