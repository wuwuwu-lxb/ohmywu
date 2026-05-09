<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue"

interface Message {
  role: "user" | "agent"
  content: string
}

const messages = ref<Message[]>([])
const input = ref("")
const pending = ref(false)
const chatEl = ref<HTMLElement>()

const send = async () => {
  const text = input.value.trim()
  if (!text || pending.value) return
  messages.value.push({ role: "user", content: text })
  input.value = ""
  pending.value = true
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight

  // placeholder echo until backend is wired
  await new Promise((r) => setTimeout(r, 600))
  messages.value.push({
    role: "agent",
    content: `[echo] ${text}`,
  })
  pending.value = false
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
    role: "agent",
    content: "你好，我是 OhMyWu。有什么可以帮你的？",
  })
})
</script>

<template>
  <div class="chat-view">
    <div class="chat-messages" ref="chatEl">
      <div
        v-for="(m, i) in messages"
        :key="i"
        :class="['chat-bubble', m.role]"
      >
        <span class="chat-label">{{ m.role === "user" ? "你" : "OhMyWu" }}</span>
        <p class="chat-text">{{ m.content }}</p>
      </div>
      <div v-if="pending" class="chat-bubble agent">
        <span class="chat-label">OhMyWu</span>
        <p class="chat-text thinking">...</p>
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
      <button class="chat-send" :disabled="pending || !input.trim()" @click="send">
        发送
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 0 16px;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.chat-bubble {
  max-width: 85%;
  padding: 10px 14px;
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.6;
}

.chat-bubble.user {
  align-self: flex-end;
  background: #1e3a5f;
  color: #d0e0ff;
}

.chat-bubble.agent {
  align-self: flex-start;
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
}

.chat-label {
  display: block;
  font-size: 11px;
  color: #666;
  margin-bottom: 4px;
}

.chat-text {
  margin: 0;
  white-space: pre-wrap;
}

.chat-text.thinking {
  color: #555;
  font-style: italic;
}

.chat-input-bar {
  display: flex;
  gap: 8px;
  padding: 12px 0 16px;
  border-top: 1px solid #1a1a1a;
}

.chat-input {
  flex: 1;
  resize: none;
  padding: 10px 12px;
  border-radius: 6px;
  border: 1px solid #2a2a2a;
  background: #0f0f0f;
  color: #e0e0e0;
  font-size: 14px;
  font-family: inherit;
  outline: none;
}

.chat-input:focus {
  border-color: #3a6fb5;
}

.chat-send {
  padding: 0 16px;
  border-radius: 6px;
  border: none;
  background: #1e3a5f;
  color: #d0e0ff;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
}

.chat-send:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
