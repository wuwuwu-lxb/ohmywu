<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue"
import ChatMessage from "../components/ChatMessage.vue"
import ThemeSelect from "../components/ThemeSelect.vue"
import { useAgentStore } from "../stores/agents"
import { useChatStore, type SessionSummary } from "../stores/chat"

const store = useChatStore()
const agentStore = useAgentStore()
const input = ref("")
const chatEl = ref<HTMLElement>()
const categoryInput = ref("")
const renameDrafts = ref<Record<string, string>>({})
const categoryDrafts = ref<Record<string, string>>({})
const confirmingDeleteId = ref<string | null>(null)
const sessionBusyId = ref<string | null>(null)
const sessionActionMsg = ref("")

const emit = defineEmits<{
  "show-task": [taskId: string]
}>()

const renderedMessages = computed(() => {
  const items = [...store.messages]
  if (!store.pending || !store.activeTurnId) {
    return items
  }

  const liveMsg = {
    id: `turn-${store.activeTurnId}`,
    role: "agent" as const,
    content: store.streamingContent,
    turnId: store.activeTurnId,
    agentName: currentAgent.value?.name || "OhMyWu",
    agentIcon: "✦",
    timestamp: Date.now(),
  }

  const existingIndex = items.findIndex(
    (msg) => msg.role === "agent" && msg.turnId === store.activeTurnId
  )

  if (existingIndex >= 0) {
    items[existingIndex] = {
      ...items[existingIndex],
      content: store.streamingContent || items[existingIndex].content,
    }
    return items
  }

  items.push(liveMsg)
  return items
})

const currentAgent = computed(() =>
  agentStore.agents.find((agent) => agent.id === agentStore.activeAgentId) || agentStore.agents[0]
)

const categoryTabs = computed(() => {
  const uncategorizedCount = store.sessions.filter((session) => !(session.category || "").trim()).length
  return [
    {
      id: "all",
      label: "全部对话",
      note: "查看全部会话",
      count: store.sessions.length,
    },
    {
      id: "__uncategorized__",
      label: "未分类",
      note: "暂未归档",
      count: uncategorizedCount,
    },
    ...store.sessionCategories.map((category) => ({
      id: category,
      label: category,
      note: "自定义分类",
      count: store.sessions.filter((session) => (session.category || "").trim() === category).length,
    })),
  ]
})

const currentCategoryLabel = computed(() => {
  if (store.selectedCategory === "all") return "全部对话"
  if (store.selectedCategory === "__uncategorized__") return "未分类"
  return store.selectedCategory
})

const currentSessionCategoryLabel = computed(() => {
  const category = store.currentSession?.category?.trim()
  return category || "未分类"
})

const categoryOptions = computed(() => [
  { label: "未分类", value: "__uncategorized__" },
  ...store.sessionCategories.map((category) => ({
    label: category,
    value: category,
  })),
])

const agentOptions = computed(() =>
  agentStore.agents.map((agent) => ({
    label: agent.name,
    value: agent.id,
  }))
)

const send = async () => {
  const text = input.value.trim()
  if (!text || store.pending) return
  input.value = ""
  await store.sendMessage(text, currentAgent.value)
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

const syncSessionDrafts = (sessions: SessionSummary[]) => {
  const nextNames: Record<string, string> = {}
  const nextCategories: Record<string, string> = {}

  for (const session of sessions) {
    nextNames[session.id] = renameDrafts.value[session.id] ?? session.name
    nextCategories[session.id] =
      categoryDrafts.value[session.id] ?? ((session.category || "").trim() || "__uncategorized__")
  }

  renameDrafts.value = nextNames
  categoryDrafts.value = nextCategories
}

const formatSessionTime = (timestamp: string) => {
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return timestamp
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date)
}

const openSession = async (id: string) => {
  await store.loadSession(id)
  store.setPanel("conversation")
  scroll()
}

const createSessionInCategory = async () => {
  sessionBusyId.value = "__create__"
  sessionActionMsg.value = ""
  try {
    const label =
      store.selectedCategory !== "all" && store.selectedCategory !== "__uncategorized__"
        ? store.selectedCategory
        : ""
    const name = label
      ? `${label} 对话 ${store.filteredSessions.length + 1}`
      : `对话 ${store.sessions.length + 1}`
    await store.createSession(name)
    store.setPanel("conversation")
  } catch (error) {
    sessionActionMsg.value = String(error)
  } finally {
    sessionBusyId.value = null
  }
}

const saveSessionName = async (session: SessionSummary) => {
  const nextName = (renameDrafts.value[session.id] || "").trim()
  if (!nextName || nextName === session.name) return

  sessionBusyId.value = session.id
  sessionActionMsg.value = ""
  try {
    await store.updateSessionMeta(session.id, { name: nextName })
    sessionActionMsg.value = "名称已保存"
  } catch (error) {
    sessionActionMsg.value = String(error)
  } finally {
    sessionBusyId.value = null
  }
}

const applySessionCategory = async (session: SessionSummary) => {
  const raw = categoryDrafts.value[session.id] || "__uncategorized__"
  const nextCategory = raw === "__uncategorized__" ? "" : raw

  sessionBusyId.value = session.id
  sessionActionMsg.value = ""
  try {
    await store.updateSessionMeta(session.id, { category: nextCategory })
    sessionActionMsg.value = "分类已更新"
  } catch (error) {
    sessionActionMsg.value = String(error)
  } finally {
    sessionBusyId.value = null
  }
}

const addCategory = () => {
  const trimmed = categoryInput.value.trim()
  if (!trimmed) return
  store.addCustomCategory(trimmed)
  categoryInput.value = ""
  sessionActionMsg.value = `已切换到分类「${trimmed}」`
}

const toggleDelete = (id: string) => {
  confirmingDeleteId.value = confirmingDeleteId.value === id ? null : id
}

const confirmDeleteSession = async (id: string) => {
  sessionBusyId.value = id
  sessionActionMsg.value = ""
  try {
    await store.deleteSession(id)
    confirmingDeleteId.value = null
    sessionActionMsg.value = "对话已删除"
  } catch (error) {
    sessionActionMsg.value = String(error)
  } finally {
    sessionBusyId.value = null
  }
}

watch(
  () => store.sessions,
  (sessions) => syncSessionDrafts(sessions),
  { deep: true, immediate: true }
)

watch(() => store.messages.length, () => {
  if (store.panel === "conversation") scroll()
})
watch(() => store.streamingContent, () => {
  if (store.panel === "conversation") scroll()
})
watch(() => store.activeTurnId, () => {
  if (store.panel === "conversation") scroll()
})

onMounted(async () => {
  await store.init()
  if (store.panel === "conversation") {
    scroll()
  }
})
</script>

<template>
  <div class="chat-view">
    <section v-if="store.panel === 'manager'" class="session-manager">
      <aside class="manager-sidebar">
        <div class="manager-card">
          <div class="manager-card-title">分类</div>
          <div class="category-list">
            <button
              v-for="tab in categoryTabs"
              :key="tab.id"
              :class="['category-item', { active: store.selectedCategory === tab.id }]"
              @click="store.setSelectedCategory(tab.id)"
            >
              <div>
                <div class="category-label">{{ tab.label }}</div>
                <div class="category-note">{{ tab.note }}</div>
              </div>
              <span class="category-count">{{ tab.count }}</span>
            </button>
          </div>
        </div>

        <div class="manager-card">
          <div class="manager-card-title">新分类</div>
          <div class="category-create">
            <input
              v-model="categoryInput"
              class="manager-input"
              type="text"
              placeholder="输入分类名称"
              @keydown.enter.prevent="addCategory"
            />
            <button class="ghost-btn" @click="addCategory">添加</button>
          </div>
        </div>
      </aside>

      <section class="manager-main">
        <div class="manager-head">
          <div>
            <div class="manager-title">{{ currentCategoryLabel }}</div>
            <div class="manager-subtitle">
              当前共 {{ store.filteredSessions.length }} 个对话。你可以在这里命名、分类、删除，再切回主对话页。
            </div>
          </div>
          <button
            class="primary-btn"
            :disabled="sessionBusyId === '__create__'"
            @click="createSessionInCategory"
          >
            {{ sessionBusyId === "__create__" ? "创建中..." : "在当前分类新建对话" }}
          </button>
        </div>

        <div v-if="sessionActionMsg" class="manager-msg">{{ sessionActionMsg }}</div>

        <div class="session-list">
          <article
            v-for="session in store.filteredSessions"
            :key="session.id"
            :class="['session-card', { active: session.id === store.currentSessionId }]"
          >
            <button class="session-open" @click="openSession(session.id)">
              <div class="session-name">{{ session.name }}</div>
              <div class="session-meta">
                <span class="session-pill">{{ session.category || "未分类" }}</span>
                <span>{{ formatSessionTime(session.updated_at) }}</span>
                <span>{{ session.message_count }} 条消息</span>
              </div>
            </button>

            <div class="session-edit">
              <input
                v-model="renameDrafts[session.id]"
                class="manager-input"
                type="text"
                placeholder="对话名称"
              />

              <div class="session-actions">
                <ThemeSelect
                  :model-value="categoryDrafts[session.id]"
                  class="manager-select"
                  :options="categoryOptions"
                  @update:model-value="(value) => { categoryDrafts[session.id] = String(value); applySessionCategory(session) }"
                />

                <button
                  class="ghost-btn"
                  :disabled="sessionBusyId === session.id || renameDrafts[session.id]?.trim() === session.name"
                  @click="saveSessionName(session)"
                >
                  保存名称
                </button>

                <template v-if="confirmingDeleteId === session.id">
                  <button
                    class="danger-btn"
                    :disabled="sessionBusyId === session.id"
                    @click="confirmDeleteSession(session.id)"
                  >
                    确认删除
                  </button>
                  <button class="ghost-btn" @click="toggleDelete(session.id)">取消</button>
                </template>
                <button
                  v-else
                  class="ghost-btn danger-ghost"
                  :disabled="sessionBusyId === session.id"
                  @click="toggleDelete(session.id)"
                >
                  删除
                </button>
              </div>
            </div>
          </article>

          <div v-if="!store.filteredSessions.length" class="manager-empty">
            当前分类还没有对话。你可以直接在这里新建一个。
          </div>
        </div>
      </section>
    </section>

    <section v-else class="conversation-shell">
      <div class="chat-header">
        <div class="session-badges">
          <span class="session-pill strong">{{ store.currentSession?.name || "新对话" }}</span>
          <span class="session-pill">{{ currentSessionCategoryLabel }}</span>
        </div>
      </div>

      <div class="chat-messages" ref="chatEl">
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

        <ChatMessage
          v-for="(msg, i) in renderedMessages"
          :key="msg.role === 'agent' && msg.turnId ? msg.turnId : msg.id"
          :msg="msg"
          :runtime="msg.turnId ? store.runtimeByTurnId[msg.turnId] : undefined"
          :pending="store.pending && !!store.activeTurnId && msg.turnId === store.activeTurnId"
          :memory-candidate="msg.turnId ? store.memoryCandidates[msg.turnId] : undefined"
          :memory-generating="msg.turnId ? !!store.memoryGenerating[msg.turnId] : false"
          :memory-saving="msg.turnId ? !!store.memorySaving[msg.turnId] : false"
          :memory-error="msg.turnId ? store.memoryErrors[msg.turnId] : null"
          :memory-saved="msg.turnId ? store.memorySaved[msg.turnId] : null"
          :style="{ animationDelay: `${i * 15}ms` }"
          class="msg-animate"
          @show-task="(taskId: string) => emit('show-task', taskId)"
          @generate-memory="(turnId: string) => store.generateMemoryCandidate(turnId)"
          @save-memory="(turnId: string) => store.saveMemoryCandidate(turnId)"
          @clear-memory="(turnId: string) => store.clearMemoryCandidate(turnId)"
          @update-memory-candidate="({ turnId, patch }) => store.updateMemoryCandidate(turnId, patch)"
        />
      </div>

      <div class="input-bar">
        <div class="composer-toolbar">
          <div class="composer-meta">
            <div class="composer-cover">✦</div>
            <div class="composer-copy">
              <div class="composer-title">{{ currentAgent?.name || "OhMyWu" }}</div>
              <div class="composer-subtitle">
                {{
                  store.pending
                    ? "Thinking"
                    : currentAgent?.persona || "Local desktop agent workspace"
                }}
              </div>
            </div>
          </div>

          <div class="composer-toolbar-controls">
            <div class="mode-switch">
              <button
                v-for="mode in ['plan', 'agent', 'auto']"
                :key="mode"
                class="mode-chip"
                :class="{ active: store.agentMode === mode }"
                @click="store.setAgentMode(mode as 'plan' | 'agent' | 'auto')"
              >
                {{ mode }}
              </button>
            </div>

            <ThemeSelect
              class="agent-select"
              :model-value="agentStore.activeAgentId"
              :options="agentOptions"
              @update:model-value="(value) => agentStore.setActiveAgent(String(value))"
            />
          </div>
        </div>

        <div class="input-row">
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
    </section>
  </div>
</template>

<style scoped>
.chat-view {
  height: 100%;
  min-height: 0;
}

.conversation-shell {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.chat-header {
  padding: 16px 20px 0;
  flex-shrink: 0;
}

.session-badges {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.session-pill {
  display: inline-flex;
  align-items: center;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
}

.session-pill.strong {
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.08);
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 18px 0;
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

.session-manager {
  height: 100%;
  display: grid;
  grid-template-columns: 300px minmax(0, 1fr);
  gap: 18px;
  padding: 24px;
  overflow: hidden;
}

.manager-sidebar,
.manager-main {
  min-height: 0;
}

.manager-sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.manager-card,
.manager-main {
  border: 1px solid var(--border-color);
  border-radius: 24px;
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.manager-card {
  padding: 18px;
}

.manager-card-title,
.manager-title {
  color: var(--text-primary);
  font-weight: 700;
}

.manager-card-title {
  margin-bottom: 12px;
  font-size: 14px;
}

.category-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.category-item {
  width: 100%;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  text-align: left;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.category-item:hover,
.category-item.active {
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.category-label {
  font-size: 13px;
  font-weight: 600;
}

.category-note {
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.category-count {
  min-width: 28px;
  padding: 5px 8px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.04);
  font-size: 11px;
  font-family: var(--font-mono);
}

.category-create {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.manager-main {
  padding: 22px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.manager-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.manager-title {
  font-size: 22px;
  margin-bottom: 6px;
}

.manager-subtitle {
  max-width: 620px;
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.65;
}

.manager-msg {
  margin-bottom: 14px;
  padding: 11px 12px;
  border-radius: 14px;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  color: var(--text-secondary);
  font-size: 12px;
}

.session-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.session-card {
  padding: 16px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: rgba(var(--accent-rgb), 0.03);
  box-shadow: var(--shadow-surface);
}

.session-card.active {
  border-color: rgba(var(--accent-rgb), 0.2);
  background: rgba(var(--accent-rgb), 0.06);
}

.session-open {
  width: 100%;
  padding: 0;
  border: none;
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.session-name {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.session-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  color: var(--text-tertiary);
  font-size: 12px;
}

.session-edit {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.session-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.manager-input,
.manager-select,
.agent-select {
  height: 40px;
  width: 100%;
  padding: 0 12px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-primary);
  font-size: 13px;
  font-family: var(--font);
  outline: none;
  transition: all var(--duration-fast) var(--ease-out);
}

.manager-input:focus,
.manager-select:focus,
.agent-select:focus {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: var(--surface-2);
}

.manager-select,
.agent-select {
  appearance: none;
  -webkit-appearance: none;
  color-scheme: dark;
}

.manager-empty {
  padding: 28px;
  border-radius: 18px;
  border: 1px dashed var(--border-color);
  text-align: center;
  color: var(--text-tertiary);
  font-size: 14px;
}

.primary-btn,
.ghost-btn,
.danger-btn {
  border-radius: 12px;
  font-family: var(--font);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.primary-btn {
  height: 40px;
  padding: 0 14px;
  border: 1px solid rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.14);
  color: var(--text-primary);
}

.ghost-btn,
.danger-btn {
  height: 40px;
  padding: 0 12px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
}

.ghost-btn:hover,
.primary-btn:hover {
  border-color: rgba(var(--accent-rgb), 0.2);
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--text-primary);
}

.danger-btn,
.danger-ghost:hover {
  border-color: rgba(255, 96, 96, 0.2);
  background: rgba(255, 96, 96, 0.08);
  color: #ffc7c7;
}

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

.input-bar {
  padding: 14px 20px 18px;
  border-top: 1px solid var(--border-color);
  background: var(--shell-bg-soft);
  backdrop-filter: blur(calc(var(--shell-blur) * 0.5));
  -webkit-backdrop-filter: blur(calc(var(--shell-blur) * 0.5));
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.composer-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.composer-toolbar-controls {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.composer-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.composer-cover {
  width: 38px;
  height: 38px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(var(--accent-rgb), 0.16);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  color: #fff;
  box-shadow: var(--shadow-glow);
  flex-shrink: 0;
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
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  max-width: 460px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mode-switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
}

.mode-chip {
  height: 32px;
  padding: 0 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  font-family: var(--font-mono);
  text-transform: uppercase;
  transition: all var(--duration-fast) var(--ease-out);
}

.mode-chip:hover {
  color: var(--text-primary);
  background: var(--surface-2);
}

.mode-chip.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--active-bg);
}

.agent-select {
  width: 180px;
}

.input-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}

.input-wrapper {
  flex: 1;
  min-width: 0;
  padding: 10px 12px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.chat-input {
  width: 100%;
  min-height: 44px;
  max-height: 180px;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 14px;
  line-height: 1.6;
  font-family: var(--font);
}

.chat-input::placeholder {
  color: var(--text-disabled);
}

.composer-actions {
  display: flex;
  align-items: center;
}

.send-btn {
  width: 46px;
  height: 46px;
  border: 1px solid rgba(var(--accent-rgb), 0.2);
  border-radius: 16px;
  background: rgba(var(--accent-rgb), 0.14);
  color: var(--accent);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
  box-shadow: var(--shadow-surface);
}

.send-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  background: rgba(var(--accent-rgb), 0.18);
  border-color: rgba(var(--accent-rgb), 0.28);
}

.send-btn:disabled,
.ghost-btn:disabled,
.primary-btn:disabled,
.danger-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.send-btn.pending {
  color: var(--text-primary);
}

.spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.2);
  border-top-color: currentColor;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 960px) {
  .session-manager {
    grid-template-columns: 1fr;
    padding: 18px;
  }

  .manager-head,
  .composer-toolbar {
    flex-direction: column;
    align-items: flex-start;
  }

  .composer-toolbar-controls {
    width: 100%;
  }

  .agent-select {
    width: 100%;
  }

  .empty-stage {
    grid-template-columns: 1fr;
  }

  .empty-main-card {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .chat-header,
  .input-bar {
    padding-left: 14px;
    padding-right: 14px;
  }

  .input-row {
    align-items: stretch;
  }

  .session-actions {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
