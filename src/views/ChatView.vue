<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ChatMessage from "../components/ChatMessage.vue"
import ConfirmDialog from "../components/ConfirmDialog.vue"
import ThemeSelect from "../components/ThemeSelect.vue"
import { useAgentStore } from "../stores/agents"
import { useChatStore, type SessionSummary } from "../stores/chat"

interface ActionReferenceOption {
  id: string
  title: string
  description: string
  enabled: boolean
  available: boolean
}

interface MemoryReferenceOption {
  slug: string
  title: string
  folder: string
}

const store = useChatStore()
const agentStore = useAgentStore()
const input = ref("")
const chatEl = ref<HTMLElement>()
const inputEl = ref<HTMLTextAreaElement>()
const categoryInput = ref("")
const renameDrafts = ref<Record<string, string>>({})
const categoryDrafts = ref<Record<string, string>>({})
const confirmingDeleteId = ref<string | null>(null)
const confirmingDeleteCategory = ref<string | null>(null)
const sessionBusyId = ref<string | null>(null)
const categoryBusyName = ref<string | null>(null)
const sessionActionMsg = ref("")
const inputFocused = ref(false)
const composing = ref(false)
const slashIndex = ref(0)
const llmProfiles = ref<Array<{ id: string; name: string; provider_type: string; model: string }>>([])
const llmProviders = ref<Array<{ id: string; name: string }>>([])
const actionReferences = ref<ActionReferenceOption[]>([])
const memoryReferences = ref<MemoryReferenceOption[]>([])
const deleteSessionTarget = computed(() =>
  store.sessions.find((session) => session.id === confirmingDeleteId.value) || null
)
const deleteCategorySessionCount = computed(() =>
  confirmingDeleteCategory.value
    ? store.sessions.filter(
      (session) => (session.category || "").trim() === confirmingDeleteCategory.value
    ).length
    : 0
)

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
    agentName: effectiveAgent.value?.name || "OhMyWu",
    agentIcon: effectiveAgent.value?.id === "memory" ? "◎" : effectiveAgent.value?.id === "coder" ? "</>" : "✦",
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
  agentStore.availableAgents.find((agent) => agent.id === agentStore.activeAgentId)
  || agentStore.availableAgents[0]
)

const effectiveAgent = computed(() => currentAgent.value)

const routeSubtitle = computed(() => {
  if (store.pending) return "Thinking"
  return currentAgent.value?.persona || currentAgent.value?.role || "Local desktop agent workspace"
})

const categoryTabs = computed(() => {
  const uncategorizedCount = store.sessions.filter((session) => !(session.category || "").trim()).length
  return [
    {
      id: "all",
      label: "全部对话",
      note: "查看全部会话",
      count: store.sessions.length,
      deletable: false,
    },
    {
      id: "__uncategorized__",
      label: "未分类",
      note: "暂未归档",
      count: uncategorizedCount,
      deletable: false,
    },
    ...store.sessionCategories.map((category) => ({
      id: category,
      label: category,
      note: "自定义分类",
      count: store.sessions.filter((session) => (session.category || "").trim() === category).length,
      deletable: true,
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
  agentStore.availableAgents.map((agent) => ({
    label: agent.name,
    value: agent.id,
  }))
)

const slashSuggestions = computed(() => {
  const raw = input.value.trimStart()
  if (raw.startsWith("@")) {
    const lower = raw.toLowerCase()

    if (lower === "@" || lower === "@a" || lower === "@ac" || lower.startsWith("@action")) {
      const query = raw.includes(":") ? raw.split(":").slice(1).join(":").trim().toLowerCase() : ""
      return actionReferences.value
        .filter((action) =>
          action.enabled
          && action.available
          && (
            !query
            || action.id.toLowerCase().includes(query)
            || action.title.toLowerCase().includes(query)
          )
        )
        .slice(0, 8)
        .map((action) => ({
          label: `@action:${action.id}`,
          description: action.title || action.description || "策略模板",
          insert: `@action:${action.id} `,
        }))
    }

    if (lower === "@m" || lower === "@me" || lower.startsWith("@memory")) {
      const query = raw.includes(":") ? raw.split(":").slice(1).join(":").trim().toLowerCase() : ""
      return memoryReferences.value
        .filter((note) =>
          !query
          || note.slug.toLowerCase().includes(query)
          || note.title.toLowerCase().includes(query)
        )
        .slice(0, 8)
        .map((note) => ({
          label: `@memory:${note.slug}`,
          description: `${note.title} · ${note.folder}`,
          insert: `@memory:${note.slug} `,
        }))
    }

    return [
      { label: "@action:", description: "引用一个 action 策略模板", insert: "@action:" },
      { label: "@memory:", description: "引用一条知识库记忆", insert: "@memory:" },
    ]
  }

  if (!raw.startsWith("/")) return []
  const text = raw.slice(1)
  const [command = "", ...rest] = text.split(/\s+/)
  const arg = rest.join(" ").trim().toLowerCase()

  if (!command) {
    return [
      { label: "/profile", description: "切换当前模型配置", insert: "/profile " },
      { label: "/profiles", description: "查看全部模型配置", insert: "/profiles" },
      { label: "/provider", description: "切换当前 provider", insert: "/provider " },
      { label: "/model", description: "切换当前 model", insert: "/model " },
    ]
  }

  if ("profiles".startsWith(command.toLowerCase())) {
    return [{ label: "/profiles", description: "查看全部模型配置", insert: "/profiles" }]
  }

  if ("profile".startsWith(command.toLowerCase())) {
    return llmProfiles.value
      .filter((profile) =>
        !arg
        || profile.id.toLowerCase().includes(arg)
        || profile.name.toLowerCase().includes(arg)
      )
      .slice(0, 8)
      .map((profile) => ({
        label: `/profile ${profile.id}`,
        description: `${profile.name} · ${profile.provider_type} · ${profile.model}`,
        insert: `/profile ${profile.id}`,
      }))
  }

  if ("provider".startsWith(command.toLowerCase())) {
    return llmProviders.value
      .filter((provider) =>
        !arg
        || provider.id.toLowerCase().includes(arg)
        || provider.name.toLowerCase().includes(arg)
      )
      .slice(0, 8)
      .map((provider) => ({
        label: `/provider ${provider.id}`,
        description: provider.name,
        insert: `/provider ${provider.id}`,
      }))
  }

  if ("model".startsWith(command.toLowerCase())) {
    return [...new Set(llmProfiles.value.map((profile) => profile.model).filter(Boolean))]
      .filter((model) => !arg || model.toLowerCase().includes(arg))
      .slice(0, 8)
      .map((model) => ({
        label: `/model ${model}`,
        description: "切换当前配置的模型名",
        insert: `/model ${model}`,
      }))
  }

  return []
})

const send = async () => {
  const text = input.value.trim()
  if (!text || store.pending) return
  input.value = ""
  await store.sendMessage(text, effectiveAgent.value, agentStore.availableAgents)
  if (text.startsWith("/")) {
    await loadCommandContext()
  }
  if (text.includes("@action:") || text.includes("@memory:")) {
    await loadReferenceContext()
  }
  syncInputHeight()
  scroll()
}

const scroll = async () => {
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
}

const syncInputHeight = () => {
  const el = inputEl.value
  if (!el) return
  el.style.height = "44px"
  el.style.height = `${Math.min(el.scrollHeight, 180)}px`
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.isComposing || composing.value) {
    return
  }
  if (slashSuggestions.value.length) {
    if (e.key === "ArrowDown") {
      e.preventDefault()
      slashIndex.value = (slashIndex.value + 1) % slashSuggestions.value.length
      return
    }
    if (e.key === "ArrowUp") {
      e.preventDefault()
      slashIndex.value = (slashIndex.value - 1 + slashSuggestions.value.length) % slashSuggestions.value.length
      return
    }
    if (e.key === "Tab") {
      e.preventDefault()
      applySlashSuggestion(slashSuggestions.value[slashIndex.value]?.insert)
      return
    }
    if (e.key === "Escape") {
      slashIndex.value = 0
    }
  }
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}

const handleInput = () => {
  if (slashIndex.value >= slashSuggestions.value.length) {
    slashIndex.value = 0
  }
  syncInputHeight()
}

const handleCompositionStart = () => {
  composing.value = true
}

const handleCompositionEnd = () => {
  composing.value = false
  syncInputHeight()
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

const toggleDeleteCategory = (name: string) => {
  confirmingDeleteCategory.value = confirmingDeleteCategory.value === name ? null : name
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

const confirmDeleteCurrentSession = async () => {
  if (!confirmingDeleteId.value) return
  await confirmDeleteSession(confirmingDeleteId.value)
}

const confirmDeleteCurrentCategory = async () => {
  if (!confirmingDeleteCategory.value) return
  const name = confirmingDeleteCategory.value
  categoryBusyName.value = name
  sessionActionMsg.value = ""
  try {
    await store.removeCustomCategory(name)
    confirmingDeleteCategory.value = null
    sessionActionMsg.value = `分类「${name}」已删除`
  } catch (error) {
    sessionActionMsg.value = String(error)
  } finally {
    categoryBusyName.value = null
  }
}

const loadReferenceContext = async () => {
  try {
    const [actions, notes] = await Promise.all([
      invoke<ActionReferenceOption[]>("get_actions"),
      invoke<MemoryReferenceOption[]>("wiki_list_notes"),
    ])
    actionReferences.value = actions || []
    memoryReferences.value = notes || []
  } catch (error) {
    console.error("Load reference context:", error)
  }
}

const applySlashSuggestion = (value?: string) => {
  if (!value) return
  input.value = value
  syncInputHeight()
  nextTick(() => inputEl.value?.focus())
}

const loadCommandContext = async () => {
  try {
    const [config, providers] = await Promise.all([
      invoke<{
        llm_profiles?: Array<{
          id: string
          name: string
          provider_type: string
          model: string
        }>
      }>("get_config"),
      invoke<Array<{ id: string; name: string }>>("get_llm_providers"),
    ])
    llmProfiles.value = config.llm_profiles || []
    llmProviders.value = providers || []
  } catch (error) {
    console.error("Load command context:", error)
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
watch(input, () => {
  syncInputHeight()
})

onMounted(async () => {
  await agentStore.init()
  await store.init()
  await loadCommandContext()
  await loadReferenceContext()
  await nextTick()
  syncInputHeight()
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
            <div
              v-for="tab in categoryTabs"
              :key="tab.id"
              class="category-row"
            >
              <button
                :class="['category-item', { active: store.selectedCategory === tab.id }]"
                @click="store.setSelectedCategory(tab.id)"
              >
                <div>
                  <div class="category-label">{{ tab.label }}</div>
                  <div class="category-note">{{ tab.note }}</div>
                </div>
                <span class="category-count">{{ tab.count }}</span>
              </button>
              <button
                v-if="tab.deletable"
                class="category-delete"
                :disabled="categoryBusyName === tab.label"
                @click="toggleDeleteCategory(tab.label)"
              >
                删除
              </button>
            </div>
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
              当前分类下共 {{ store.filteredSessions.length }} 个对话。
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

                <button
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
            当前分类暂无对话。
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
                <div class="empty-title">开始新对话</div>
                <div class="empty-desc">选择 Agent 后直接发送消息，工具调用和运行状态会同步显示在对话里。</div>
                <div class="empty-meta">
                  <span class="meta-pill">Agent</span>
                  <span class="meta-pill">Runtime</span>
                  <span class="meta-pill">Audit</span>
                </div>
              </div>
            </section>

            <section class="empty-side-card">
              <div class="side-title">快速开始</div>
              <button class="hint" @click="input = '帮我看看当前目录有什么文件'">帮我看看当前目录有什么文件</button>
              <button class="hint" @click="input = '读取 README.md 并总结项目结构'">读取 README.md 并总结项目结构</button>
              <button class="hint" @click="input = '使用 @action:system.skill_to_action 帮我把一个 skill 转成 action'">引用一个 action 模板</button>
              <button class="hint" @click="input = '结合 @memory:ohmywu-guide 帮我总结当前产品定位和使用路径'">引用一条知识记忆</button>
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
          :memory-collapsed="msg.turnId ? !!store.memoryCollapsed[msg.turnId] : false"
          :style="{ animationDelay: `${i * 15}ms` }"
          class="msg-animate"
          @show-task="(taskId: string) => emit('show-task', taskId)"
          @generate-memory="(turnId: string) => store.generateMemoryCandidate(turnId)"
          @save-memory="(turnId: string) => store.saveMemoryCandidate(turnId)"
          @clear-memory="(turnId: string) => store.clearMemoryCandidate(turnId)"
          @reopen-memory="(turnId: string) => store.reopenMemoryCandidate(turnId)"
          @update-memory-candidate="({ turnId, patch }) => store.updateMemoryCandidate(turnId, patch)"
        />
      </div>

      <div class="input-bar">
        <div class="composer-toolbar">
          <div class="composer-meta">
            <div class="composer-cover">✦</div>
            <div class="composer-copy">
              <div class="composer-title">{{ effectiveAgent?.name || "OhMyWu" }}</div>
              <div class="composer-subtitle">
                {{
                  routeSubtitle
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
          <div class="input-wrapper" :class="{ focused: inputFocused }">
            <textarea
              ref="inputEl"
              v-model="input"
              class="chat-input"
              rows="1"
              placeholder="输入消息，可用 / 命令、@action:action_id 或 @memory:slug 引用上下文"
              :disabled="store.pending"
              @input="handleInput"
              @focus="inputFocused = true"
              @blur="inputFocused = false"
              @compositionstart="handleCompositionStart"
              @compositionend="handleCompositionEnd"
              @keydown="handleKeydown"
            />
            <div v-if="slashSuggestions.length" class="slash-panel">
              <button
                v-for="(item, index) in slashSuggestions"
                :key="item.label"
                type="button"
                :class="['slash-item', { active: index === slashIndex }]"
                @mousedown.prevent="applySlashSuggestion(item.insert)"
              >
                <span class="slash-label">{{ item.label }}</span>
                <span class="slash-desc">{{ item.description }}</span>
              </button>
            </div>
          </div>

          <div class="composer-actions">
            <button
              v-if="!store.pending"
              class="send-btn"
              :disabled="!input.trim()"
              @click="send"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M2 8L14 2L8 14L6 9L2 8Z" fill="currentColor" />
              </svg>
            </button>

            <button
              v-else
              class="send-btn stop-btn"
              :class="{ pending: store.pending, stopping: store.cancelling }"
              :disabled="store.cancelling"
              @click="store.cancelAgent()"
            >
              <span v-if="store.cancelling" class="spinner" />
              <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3.5" y="3.5" width="9" height="9" rx="2" fill="currentColor" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </section>

    <ConfirmDialog
      :open="!!confirmingDeleteId"
      title="删除对话"
      :message="deleteSessionTarget ? `确定删除「${deleteSessionTarget.name}」吗？删除后将移除当前对话及其消息记录。` : '删除后将移除当前对话及其消息记录。'"
      :loading="!!(confirmingDeleteId && sessionBusyId === confirmingDeleteId)"
      @cancel="confirmingDeleteId = null"
      @confirm="confirmDeleteCurrentSession"
    />

    <ConfirmDialog
      :open="!!confirmingDeleteCategory"
      title="删除对话分类"
      :message="confirmingDeleteCategory ? `确定删除分类「${confirmingDeleteCategory}」吗？${deleteCategorySessionCount ? `该分类下的 ${deleteCategorySessionCount} 个对话会被移动到未分类。` : '该分类下当前没有对话。'}` : '删除后该分类下的对话会被移动到未分类。'"
      :loading="!!(confirmingDeleteCategory && categoryBusyName === confirmingDeleteCategory)"
      @cancel="confirmingDeleteCategory = null"
      @confirm="confirmDeleteCurrentCategory"
    />
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

.category-row {
  display: flex;
  align-items: stretch;
  gap: 8px;
}

.category-item {
  flex: 1;
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

.category-delete {
  flex-shrink: 0;
  align-self: center;
  padding: 9px 12px;
  border-radius: 14px;
  border: 1px solid rgba(255, 96, 96, 0.16);
  background: rgba(255, 96, 96, 0.08);
  color: #ffc7c7;
  font-size: 12px;
  font-family: var(--font);
  cursor: pointer;
  transition: background var(--duration-fast), border-color var(--duration-fast), color var(--duration-fast);
}

.category-delete:hover:not(:disabled) {
  background: rgba(255, 96, 96, 0.12);
  border-color: rgba(255, 96, 96, 0.24);
}

.category-delete:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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
  position: relative;
  overflow: hidden;
}

.empty-cover::after {
  content: "";
  position: absolute;
  inset: -40%;
  background: linear-gradient(115deg, transparent 25%, rgba(255, 255, 255, 0.06) 50%, transparent 75%);
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
  transform: translateX(2px);
}

.input-bar {
  padding: 14px 20px 18px;
  border-top: 1px solid var(--border-color);
  background: var(--shell-bg-soft);
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

.route-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.route-switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
}

.route-chip {
  height: 32px;
  padding: 0 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  font-family: var(--font-mono);
  transition: all var(--duration-fast) var(--ease-out);
}

.route-chip:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--surface-2);
}

.route-chip.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--active-bg);
}

.route-summary {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.route-label {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
}

.delegate-select {
  width: 220px;
}

.route-advice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.route-hint {
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.55;
}

.route-recommend {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.route-recommend-label {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.route-recommend-chip {
  height: 28px;
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 11px;
  font-family: var(--font-mono);
  transition: all var(--duration-fast) var(--ease-out);
}

.route-recommend-chip:hover {
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.28);
  background: rgba(var(--accent-rgb), 0.14);
}

.input-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}

.input-wrapper {
  position: relative;
  flex: 1;
  min-width: 0;
  padding: 10px 12px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
}

.input-wrapper.focused {
  border-color: rgba(var(--accent-rgb), 0.34);
  background: rgba(var(--accent-rgb), 0.05);
  box-shadow:
    var(--shadow-surface),
    0 0 0 1px rgba(var(--accent-rgb), 0.16),
    0 0 0 4px rgba(var(--accent-rgb), 0.08);
}

.input-wrapper::after {
  content: "";
  position: absolute;
  inset: -1px;
  border-radius: 22px;
  pointer-events: none;
  opacity: 0;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.18), transparent 45%, rgba(var(--accent-rgb), 0.12));
  transition: opacity 180ms ease;
}

.input-wrapper.focused::after {
  opacity: 1;
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
  caret-color: var(--accent);
  -webkit-user-modify: read-write-plaintext-only;
}

.chat-input::placeholder {
  color: var(--text-disabled);
}

.slash-panel {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 10px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.slash-item {
  width: 100%;
  border: 1px solid transparent;
  border-radius: 14px;
  background: var(--surface-2);
  padding: 10px 12px;
  text-align: left;
  color: inherit;
  cursor: pointer;
  transition: border-color 140ms ease, background 140ms ease;
}

.slash-item:hover,
.slash-item.active {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.08);
}

.slash-label {
  display: block;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-primary);
}

.slash-desc {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-secondary);
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
  background: rgba(var(--accent-rgb), 0.18);
  border-color: rgba(var(--accent-rgb), 0.28);
  box-shadow: var(--shadow-surface);
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

.stop-btn {
  background: rgba(248, 113, 113, 0.12);
  border-color: rgba(248, 113, 113, 0.24);
  color: #fca5a5;
}

.stop-btn:hover:not(:disabled) {
  background: rgba(248, 113, 113, 0.18);
  border-color: rgba(248, 113, 113, 0.32);
}

.stop-btn.stopping {
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

  .agent-select,
  .delegate-select {
    width: 100%;
  }

  .route-bar,
  .route-summary,
  .route-advice {
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
