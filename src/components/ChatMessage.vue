<script setup lang="ts">
import { computed, ref, watch } from "vue"
import ExecutionCard from "./ExecutionCard.vue"
import RuntimeSummary from "./RuntimeSummary.vue"
import ThemeSelect from "./ThemeSelect.vue"
import { renderMarkdown } from "../lib/markdown"
import type {
  ChatMsg,
  MemoryCandidate,
  RuntimeTurnView,
  SavedMemoryNote,
} from "../stores/chat"

const props = defineProps<{
  msg: ChatMsg
  runtime?: RuntimeTurnView
  pending?: boolean
  memoryCandidate?: MemoryCandidate
  memoryGenerating?: boolean
  memorySaving?: boolean
  memoryError?: string | null
  memorySaved?: SavedMemoryNote | null
  memoryCollapsed?: boolean
}>()

const emit = defineEmits<{
  "show-task": [taskId: string]
  "generate-memory": [turnId: string]
  "save-memory": [turnId: string]
  "clear-memory": [turnId: string]
  "reopen-memory": [turnId: string]
  "update-memory-candidate": [payload: { turnId: string, patch: Partial<MemoryCandidate> }]
}>()
const copied = ref(false)
const tagInput = ref("")

const renderedHtml = computed(() => renderMarkdown(props.msg.content || ""))
const hasMemoryData = computed(() =>
  !!props.memoryCandidate || !!props.memorySaved
)
const memoryOpen = computed(() =>
  (!!props.memoryCandidate || !!props.memoryError || !!props.memoryGenerating || !!props.memorySaved)
  && !props.memoryCollapsed
)
const memoryFolderOptions = [
  { label: "concepts", value: "concepts" },
  { label: "notes", value: "notes" },
  { label: "daily", value: "daily" },
  { label: "profile", value: "profile" },
]

watch(
  () => props.memoryCandidate?.tags,
  (tags) => {
    tagInput.value = tags?.join(", ") || ""
  },
  { immediate: true, deep: true }
)

async function copyMessage() {
  try {
    await navigator.clipboard.writeText(props.msg.content)
    copied.value = true
    window.setTimeout(() => {
      copied.value = false
    }, 1400)
  } catch (error) {
    console.error("Copy message:", error)
  }
}

function updateCandidate(patch: Partial<MemoryCandidate>) {
  if (!props.msg.turnId || !props.memoryCandidate) return
  emit("update-memory-candidate", {
    turnId: props.msg.turnId,
    patch,
  })
}

function updateTags(value: string) {
  tagInput.value = value
  updateCandidate({
    tags: value
      .split(",")
      .map((tag) => tag.trim())
      .filter(Boolean),
  })
}
</script>

<template>
  <div :class="['msg-row', msg.role]">
    <template v-if="msg.role === 'agent'">
      <div class="msg-icon">
        <span>{{ msg.agentIcon || "✦" }}</span>
      </div>
      <div class="msg-body">
        <div class="msg-header">
          <div class="msg-meta">
            <span class="msg-sender">{{ msg.agentName || "OhMyWu" }}</span>
            <span class="msg-time">{{ new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
          </div>
          <div class="msg-actions">
            <button
              v-if="msg.turnId"
              class="copy-btn"
              type="button"
              :disabled="memoryGenerating || pending"
              @click="emit('generate-memory', msg.turnId)"
            >
              {{ memoryGenerating ? "生成中" : memoryCandidate ? "重生成记忆" : "记忆候选" }}
            </button>
            <button
              v-if="msg.turnId && hasMemoryData && memoryCollapsed"
              class="copy-btn"
              type="button"
              @click="emit('reopen-memory', msg.turnId)"
            >
              展开记忆
            </button>
            <button class="copy-btn" type="button" @click="copyMessage">
              {{ copied ? "已复制" : "复制" }}
            </button>
          </div>
        </div>
        <div v-if="pending && !msg.content" class="thinking-dots inline-thinking">
          <span class="dot" />
          <span class="dot" />
          <span class="dot" />
        </div>
        <div v-else class="msg-text markdown-body">
          <div v-html="renderedHtml" />
          <span v-if="pending" class="inline-cursor">|</span>
        </div>
        <RuntimeSummary v-if="runtime" :runtime="runtime" />
        <div v-else-if="msg.execs?.length" class="msg-execs">
          <ExecutionCard v-for="(exec, i) in msg.execs" :key="i" :exec="exec" />
        </div>
        <div v-if="msg.turnId && memoryOpen" class="memory-card">
          <div class="memory-head">
            <div>
              <div class="memory-title">记忆候选</div>
              <div v-if="memoryCandidate" class="memory-note">
                {{ memoryCandidate.shouldSave ? "建议沉淀" : "建议忽略" }} · {{ memoryCandidate.reason }}
              </div>
              <div v-else-if="memorySaved" class="memory-note">
                已写入 {{ memorySaved.folder }}/{{ memorySaved.slug }}
              </div>
            </div>
            <button class="copy-btn subtle" type="button" @click="emit('clear-memory', msg.turnId)">
              收起
            </button>
          </div>

          <div v-if="memoryError" class="memory-error">{{ memoryError }}</div>

          <template v-if="memoryCandidate">
            <div class="memory-grid">
              <label class="memory-field">
                <span>标题</span>
                <input
                  class="memory-input"
                  :value="memoryCandidate.title"
                  type="text"
                  @input="updateCandidate({ title: ($event.target as HTMLInputElement).value })"
                />
              </label>

              <label class="memory-field">
                <span>范围</span>
                <ThemeSelect
                  class="memory-input"
                  :model-value="memoryCandidate.folder"
                  :options="memoryFolderOptions"
                  @update:model-value="(value) => updateCandidate({ folder: String(value) })"
                />
              </label>
            </div>

            <label class="memory-field">
              <span>标签</span>
              <input
                class="memory-input"
                :value="tagInput"
                type="text"
                placeholder="逗号分隔"
                @input="updateTags(($event.target as HTMLInputElement).value)"
              />
            </label>

            <label class="memory-field">
              <span>正文</span>
              <textarea
                class="memory-textarea"
                :value="memoryCandidate.body"
                rows="8"
                @input="updateCandidate({ body: ($event.target as HTMLTextAreaElement).value })"
              />
            </label>

            <div class="memory-actions">
              <button
                class="memory-save-btn"
                type="button"
                :disabled="memorySaving"
                @click="emit('save-memory', msg.turnId)"
              >
                {{ memorySaving ? "写入中" : "写入知识库" }}
              </button>
              <div v-if="memorySaved" class="memory-saved">
                已保存为 {{ memorySaved.title }}
              </div>
            </div>
          </template>
        </div>
        <button
          v-else-if="msg.turnId && hasMemoryData && memoryCollapsed"
          class="memory-collapsed-pill"
          type="button"
          @click="emit('reopen-memory', msg.turnId)"
        >
          <span class="memory-collapsed-title">记忆候选已收起</span>
          <span class="memory-collapsed-action">点击展开</span>
        </button>
        <div v-if="msg.taskId" class="msg-task-link" @click="emit('show-task', msg.taskId!)">
          <span>查看执行链路</span>
          <span class="link-arrow">→</span>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="msg-body user-body">
        <div class="msg-header user-header">
          <button class="copy-btn subtle" type="button" @click="copyMessage">
            {{ copied ? "已复制" : "复制" }}
          </button>
          <span class="msg-time">{{ new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}</span>
        </div>
        <div class="msg-text user-text markdown-body" v-html="renderedHtml" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.msg-row {
  display: flex;
  gap: 12px;
  padding: 10px 24px;
  max-width: 860px;
  margin: 0 auto;
  width: 100%;
}

.msg-row.user {
  justify-content: flex-end;
}

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
  box-shadow: none;
}

.msg-body {
  flex: 1;
  min-width: 0;
  max-width: 85%;
}

.user-body {
  max-width: 74%;
}

.msg-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 6px;
}

.msg-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.msg-meta,
.user-header {
  display: flex;
  align-items: center;
  gap: 8px;
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

.copy-btn {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.copy-btn:hover {
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.12);
}

.copy-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.copy-btn.subtle {
  background: rgba(255, 255, 255, 0.03);
  border-color: var(--border-color);
}

.msg-text {
  font-size: var(--text-base);
  line-height: 1.72;
  color: var(--text-primary);
  word-break: break-word;
  padding: 14px 16px;
  border-radius: 18px;
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
  box-shadow: none;
}

.memory-card {
  margin-top: 10px;
  padding: 14px;
  border-radius: 18px;
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  background: rgba(var(--accent-rgb), 0.05);
}

.memory-head,
.memory-actions,
.memory-grid {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.memory-grid {
  margin-top: 12px;
}

.memory-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
}

.memory-note,
.memory-saved {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}

.memory-error {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(239, 68, 68, 0.12);
  border: 1px solid rgba(239, 68, 68, 0.18);
  color: #fecaca;
  font-size: 12px;
  line-height: 1.5;
}

.memory-field {
  display: block;
  flex: 1;
  margin-top: 12px;
}

.memory-field span {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
}

.memory-input,
.memory-textarea {
  width: 100%;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.03);
  color: var(--text-primary);
  font: inherit;
}

.memory-input {
  min-height: 42px;
  padding: 0 12px;
}

.memory-textarea {
  min-height: 168px;
  padding: 12px;
  line-height: 1.65;
  resize: vertical;
}

.memory-save-btn {
  min-height: 40px;
  padding: 0 14px;
  border: 1px solid rgba(var(--accent-rgb), 0.2);
  border-radius: 999px;
  background: rgba(var(--accent-rgb), 0.14);
  color: var(--text-primary);
  font-size: 12px;
  font-family: var(--font-mono);
  cursor: pointer;
}

.memory-save-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.memory-collapsed-pill {
  margin-top: 10px;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px dashed rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.04);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.memory-collapsed-pill:hover {
  border-color: rgba(var(--accent-rgb), 0.32);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
}

.memory-collapsed-title,
.memory-collapsed-action {
  font-size: 12px;
}

.memory-collapsed-action {
  font-family: var(--font-mono);
  color: var(--text-tertiary);
}

@media (max-width: 720px) {
  .memory-grid,
  .memory-head,
  .memory-actions,
  .msg-actions {
    flex-direction: column;
    align-items: stretch;
  }
}

.markdown-body :deep(p) {
  margin: 0 0 12px;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3) {
  margin: 0 0 12px;
  color: var(--text-primary);
  line-height: 1.35;
}

.markdown-body :deep(h1) {
  font-size: 18px;
}

.markdown-body :deep(h2) {
  font-size: 16px;
}

.markdown-body :deep(h3) {
  font-size: 14px;
  color: var(--text-secondary);
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0 0 12px;
  padding-left: 20px;
}

.markdown-body :deep(li) {
  margin-bottom: 4px;
}

.markdown-body :deep(blockquote) {
  margin: 0 0 12px;
  padding: 10px 12px;
  border-left: 3px solid rgba(var(--accent-rgb), 0.36);
  background: rgba(255, 255, 255, 0.03);
  color: var(--text-secondary);
  border-radius: 0 12px 12px 0;
}

.markdown-body :deep(code) {
  padding: 2px 6px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  color: #f7f8ff;
  font-family: var(--font-mono);
  font-size: 12px;
}

.markdown-body :deep(pre) {
  margin: 0 0 12px;
  padding: 14px;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  background: rgba(6, 8, 12, 0.72);
  overflow-x: auto;
}

.markdown-body :deep(pre code) {
  padding: 0;
  background: transparent;
  color: var(--text-secondary);
}

.markdown-body :deep(strong) {
  color: var(--text-primary);
  font-weight: 700;
}

.markdown-body :deep(a) {
  color: var(--accent);
  text-decoration: none;
  border-bottom: 1px dashed rgba(var(--accent-rgb), 0.5);
}

.markdown-body :deep(a:hover) {
  border-bottom-style: solid;
}

.msg-execs {
  margin-top: 8px;
}

.inline-cursor {
  margin-left: 2px;
  color: var(--accent);
  animation: blink 1s steps(2, start) infinite;
}

.inline-thinking {
  margin-top: 4px;
}

.thinking-dots {
  display: inline-flex;
  gap: 8px;
  padding: 14px 16px;
  border-radius: 16px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-surface);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: var(--accent);
  opacity: 0.4;
  animation: pulseDot 1.2s ease-in-out infinite;
}

.dot:nth-child(2) {
  animation-delay: 0.15s;
}

.dot:nth-child(3) {
  animation-delay: 0.3s;
}

@keyframes blink {
  to {
    visibility: hidden;
  }
}

@keyframes pulseDot {
  0%, 100% {
    opacity: 0.3;
    transform: translateY(0);
  }
  50% {
    opacity: 1;
    transform: translateY(-1px);
  }
}

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
