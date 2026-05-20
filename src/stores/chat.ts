import { defineStore } from "pinia"
import { ref, computed } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { serializeMemoryScope, type AgentProfile } from "./agents"

export interface ChatMsg {
  id: string
  role: "user" | "agent"
  content: string
  turnId?: string
  agentName?: string
  agentIcon?: string
  execs?: ExecutionInfo[]
  taskId?: string
  timestamp: number
}

export type AgentMode = "plan" | "agent" | "auto"

export interface ExecutionInfo {
  action: string
  status: "running" | "success" | "failed" | "denied" | "needs_confirm"
  input?: string
  output?: string
  error?: string
  duration?: string
}

export interface MemoryCandidate {
  title: string
  folder: string
  tags: string[]
  body: string
  shouldSave: boolean
  reason: string
}

export interface SavedMemoryNote {
  slug: string
  title: string
  folder: string
}

export interface SessionSummary {
  id: string
  name: string
  category: string
  message_count: number
  created_at: string
  updated_at: string
}

export type ChatPanel = "conversation" | "manager"

interface BackendMessage {
  role: string
  content: string
  agent_id?: string | null
  agent_name?: string | null
  turn_id?: string | null
  executions?: BackendExec[]
  task_id?: string
  timestamp: string
}

interface BackendExec {
  capability: string
  input: string
  output?: string
  error?: string
  status: string
  duration_ms: number
}

interface StreamChunk {
  content_delta: string | null
  done: boolean
}

interface RuntimeTurn {
  id: string
  threadId: string
  sessionId: string
  status: string
  agentMode: AgentMode
  userContent: string
  assistantContent?: string | null
  executionCount: number
  checklistCount: number
  startedAt: string
  finishedAt?: string | null
}

interface RuntimeEvent {
  id?: string
  sessionId?: string
  threadId?: string
  turnId?: string | null
  kind: string
  summary: string
  timestamp?: string | null
  status?: string
  payload?: Record<string, unknown>
}

interface RuntimeThreadView {
  turns: RuntimeTurn[]
  events: RuntimeEvent[]
}

export interface RuntimeTurnView {
  turn: RuntimeTurn
  events: RuntimeEvent[]
  tools: ExecutionInfo[]
}

interface RuntimeToolState extends ExecutionInfo {
  toolCallId?: string
}

let _msgId = 0
function nextId() {
  return `msg-${++_msgId}`
}

function normalizeExecStatus(status: string): ExecutionInfo["status"] {
  if (
    status === "running" ||
    status === "success" ||
    status === "denied" ||
    status === "needs_confirm"
  ) {
    return status
  }
  return "failed"
}

function formatDuration(durationMs: number): string {
  if (durationMs < 1000) return `${durationMs}ms`
  return `${(durationMs / 1000).toFixed(1)}s`
}

function agentIconFor(agentId?: string | null): string {
  switch (agentId) {
    case "memory":
      return "◎"
    case "coder":
      return "</>"
    default:
      return "✦"
  }
}

function backendMsgToChatMsg(msg: BackendMessage): ChatMsg {
  return {
    id: nextId(),
    role: msg.role as "user" | "agent",
    content: msg.content,
    turnId: msg.turn_id ?? undefined,
    agentName: msg.role === "agent" ? msg.agent_name || "OhMyWu" : undefined,
    agentIcon: msg.role === "agent" ? agentIconFor(msg.agent_id) : undefined,
    execs: msg.executions?.map((e: BackendExec) => ({
      action: e.capability,
      status: normalizeExecStatus(e.status),
      input: e.input,
      output: e.output,
      error: e.error,
      duration: `${(e.duration_ms / 1000).toFixed(1)}s`,
    })),
    taskId: msg.task_id,
    timestamp: msg.timestamp ? new Date(msg.timestamp).getTime() : Date.now(),
  }
}

const CHAT_PANEL_KEY = "ohmywu.chat.panel"
const CHAT_CATEGORY_KEY = "ohmywu.chat.category"
const CHAT_CUSTOM_CATEGORIES_KEY = "ohmywu.chat.custom-categories"
const CHAT_CURRENT_SESSION_KEY = "ohmywu.chat.current-session"

function loadStoredString(key: string, fallback: string): string {
  if (typeof window === "undefined") return fallback
  return window.localStorage.getItem(key) || fallback
}

function loadStoredArray(key: string): string[] {
  if (typeof window === "undefined") return []
  try {
    const raw = window.localStorage.getItem(key)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : []
  } catch {
    return []
  }
}

function persistString(key: string, value: string) {
  if (typeof window === "undefined") return
  window.localStorage.setItem(key, value)
}

function persistArray(key: string, value: string[]) {
  if (typeof window === "undefined") return
  window.localStorage.setItem(key, JSON.stringify(value))
}

export const useChatStore = defineStore("chat", () => {
  const messages = ref<ChatMsg[]>([])
  const sessions = ref<SessionSummary[]>([])
  const currentSessionId = ref<string | null>(
    loadStoredString(CHAT_CURRENT_SESSION_KEY, "") || null
  )
  const pending = ref(false)
  const error = ref<string | null>(null)
  const streamingContent = ref("")
  const agentMode = ref<AgentMode>("agent")
  const runtimeTurns = ref<RuntimeTurn[]>([])
  const runtimeEvents = ref<RuntimeEvent[]>([])
  const runtimeStatus = ref("Ready")
  const activeTurnId = ref<string | null>(null)
  const panel = ref<ChatPanel>(
    loadStoredString(CHAT_PANEL_KEY, "conversation") as ChatPanel
  )
  const selectedCategory = ref(loadStoredString(CHAT_CATEGORY_KEY, "all"))
  const customCategories = ref(loadStoredArray(CHAT_CUSTOM_CATEGORIES_KEY))
  const memoryCandidates = ref<Record<string, MemoryCandidate>>({})
  const memoryGenerating = ref<Record<string, boolean>>({})
  const memorySaving = ref<Record<string, boolean>>({})
  const memoryErrors = ref<Record<string, string | null>>({})
  const memorySaved = ref<Record<string, SavedMemoryNote | null>>({})

  let unlistenStream: UnlistenFn | null = null
  let unlistenRuntime: UnlistenFn | null = null

  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value)
  )
  const sessionCategories = computed(() => {
    const merged = new Set<string>()
    for (const category of customCategories.value) {
      const trimmed = category.trim()
      if (trimmed) merged.add(trimmed)
    }
    for (const session of sessions.value) {
      const trimmed = (session.category || "").trim()
      if (trimmed) merged.add(trimmed)
    }
    return [...merged].sort((a, b) => a.localeCompare(b, "zh-CN"))
  })
  const filteredSessions = computed(() => {
    if (selectedCategory.value === "all") {
      return sessions.value
    }
    if (selectedCategory.value === "__uncategorized__") {
      return sessions.value.filter((session) => !(session.category || "").trim())
    }
    return sessions.value.filter((session) => (session.category || "").trim() === selectedCategory.value)
  })
  const latestRuntimeEvent = computed(() =>
    runtimeEvents.value.length ? runtimeEvents.value[runtimeEvents.value.length - 1] : null
  )
  const runtimeByTurnId = computed<Record<string, RuntimeTurnView>>(() => {
    const map: Record<string, RuntimeTurnView> = {}

    for (const turn of runtimeTurns.value) {
      map[turn.id] = {
        turn,
        events: [],
        tools: [],
      }
    }

    for (const event of runtimeEvents.value) {
      if (!event.turnId) continue
      const existing = map[event.turnId]
      if (existing) {
        existing.events.push(event)
      }
    }

    for (const turnId of Object.keys(map)) {
      const toolsById: Record<string, RuntimeToolState> = {}
      const toolsOrdered: RuntimeToolState[] = []
      const events = map[turnId].events

      for (const event of events) {
        const payload = event.payload || {}
        if (event.kind === "tool.started") {
          const capability = typeof payload.capability === "string" ? payload.capability : "tool"
          const toolCallId = typeof payload.toolCallId === "string" ? payload.toolCallId : `${capability}-${toolsOrdered.length}`
          const existing = toolsById[toolCallId]
          if (existing) {
            existing.status = "running"
            existing.input = typeof payload.inputPreview === "string" ? payload.inputPreview : existing.input
            continue
          }
          const tool: RuntimeToolState = {
            toolCallId,
            action: capability,
            status: "running",
            input: typeof payload.inputPreview === "string" ? payload.inputPreview : undefined,
          }
          toolsById[toolCallId] = tool
          toolsOrdered.push(tool)
        }

        if (event.kind === "tool.completed") {
          const capability = typeof payload.capability === "string" ? payload.capability : "tool"
          const toolCallId = typeof payload.toolCallId === "string"
            ? payload.toolCallId
            : Object.keys(toolsById).find((id) => toolsById[id].action === capability && toolsById[id].status === "running")
              || `${capability}-${toolsOrdered.length}`
          const existing = toolsById[toolCallId]
          const status = normalizeExecStatus(typeof payload.status === "string" ? payload.status : "failed")
          const durationMs = typeof payload.durationMs === "number" ? payload.durationMs : undefined
          if (existing) {
            existing.status = status
            existing.input = typeof payload.inputPreview === "string" ? payload.inputPreview : existing.input
            existing.output = typeof payload.outputPreview === "string" ? payload.outputPreview : existing.output
            existing.error = typeof payload.errorPreview === "string" ? payload.errorPreview : existing.error
            existing.duration = durationMs != null ? formatDuration(durationMs) : existing.duration
          } else {
            const tool: RuntimeToolState = {
              toolCallId,
              action: capability,
              status,
              input: typeof payload.inputPreview === "string" ? payload.inputPreview : undefined,
              output: typeof payload.outputPreview === "string" ? payload.outputPreview : undefined,
              error: typeof payload.errorPreview === "string" ? payload.errorPreview : undefined,
              duration: durationMs != null ? formatDuration(durationMs) : undefined,
            }
            toolsById[toolCallId] = tool
            toolsOrdered.push(tool)
          }
        }
      }

      if (!toolsOrdered.length) {
        for (const msg of messages.value) {
          if (msg.role !== "agent" || msg.turnId !== turnId || !msg.execs?.length) continue
          map[turnId].tools = msg.execs
        }
      } else {
        map[turnId].tools = toolsOrdered.map(({ toolCallId: _toolCallId, ...tool }) => tool)
      }
    }

    return map
  })

  function resetSessionState() {
    messages.value = []
    runtimeTurns.value = []
    runtimeEvents.value = []
    runtimeStatus.value = "Ready"
    activeTurnId.value = null
    memoryCandidates.value = {}
    memoryGenerating.value = {}
    memorySaving.value = {}
    memoryErrors.value = {}
    memorySaved.value = {}
  }

  function upsertRuntimeTurn(turn: RuntimeTurn) {
    const existingIndex = runtimeTurns.value.findIndex((item) => item.id === turn.id)
    if (existingIndex >= 0) {
      runtimeTurns.value[existingIndex] = turn
      return
    }
    runtimeTurns.value.push(turn)
    runtimeTurns.value.sort((a, b) => a.startedAt.localeCompare(b.startedAt))
  }

  function handleRuntimeEvent(event: RuntimeEvent) {
    if (event.sessionId && event.sessionId !== currentSessionId.value) return

    const eventId = event.id
    if (eventId && runtimeEvents.value.some((item) => item.id === eventId)) {
      return
    }

    runtimeEvents.value.push(event)
    runtimeStatus.value = event.summary || event.kind

    if (event.kind === "turn.started" && event.turnId && event.sessionId) {
      activeTurnId.value = event.turnId
      upsertRuntimeTurn({
        id: event.turnId,
        sessionId: event.sessionId,
        threadId: event.threadId || `thread-${event.sessionId}`,
        status: "running",
        agentMode: (event.payload?.agentMode as AgentMode) || agentMode.value,
        userContent: (event.payload?.userContent as string) || "",
        assistantContent: null,
        executionCount: 0,
        checklistCount: 0,
        startedAt: event.timestamp || new Date().toISOString(),
        finishedAt: null,
      })
      return
    }

    if (event.turnId) {
      const existing = runtimeTurns.value.find((item) => item.id === event.turnId)
      if (existing && event.kind === "turn.completed") {
        existing.status = typeof event.status === "string" ? event.status : "completed"
        existing.finishedAt = event.timestamp || new Date().toISOString()
        const executionCount = event.payload?.executionCount
        if (typeof executionCount === "number") {
          existing.executionCount = executionCount
        }
        const assistantContent = event.payload?.assistantContent
        if (typeof assistantContent === "string") {
          existing.assistantContent = assistantContent
        }
        if (activeTurnId.value === event.turnId) {
          activeTurnId.value = null
        }
      }
    }
  }

  async function init() {
    try {
      agentMode.value = await invoke<AgentMode>("get_agent_mode")
      if (!unlistenRuntime) {
        unlistenRuntime = await listen<RuntimeEvent>("runtime-event", (event) => {
          handleRuntimeEvent(event.payload)
        })
      }
      const list = await invoke<SessionSummary[]>("list_sessions")
      sessions.value = list
      if (list.length > 0) {
        const targetId = currentSessionId.value && list.some((session) => session.id === currentSessionId.value)
          ? currentSessionId.value
          : list[0].id
        await loadSession(targetId)
      } else {
        await createSession("新对话")
      }
    } catch (e) {
      console.error("Failed to init sessions:", e)
      messages.value.push({
        id: nextId(),
        role: "agent",
        content: "你好，我是 OhMyWu。有什么可以帮你的？",
        agentName: "OhMyWu",
        agentIcon: "🤖",
        timestamp: Date.now(),
      })
    }
  }

  async function createSession(name: string) {
    try {
      const category =
        selectedCategory.value !== "all" && selectedCategory.value !== "__uncategorized__"
          ? selectedCategory.value
          : ""
      const summary = await invoke<SessionSummary>("create_session", { name, category })
      sessions.value.unshift(summary)
      currentSessionId.value = summary.id
      persistString(CHAT_CURRENT_SESSION_KEY, summary.id)
      resetSessionState()
    } catch (e) {
      console.error("Create session:", e)
      error.value = String(e)
    }
  }

  async function loadSession(id: string) {
    try {
      const msgs = await invoke<BackendMessage[]>("load_session", { sessionId: id })
      messages.value = msgs.map(backendMsgToChatMsg)
      currentSessionId.value = id
      persistString(CHAT_CURRENT_SESSION_KEY, id)
      memoryCandidates.value = {}
      memoryGenerating.value = {}
      memorySaving.value = {}
      memoryErrors.value = {}
      memorySaved.value = {}
      await loadRuntimeThread(id)
    } catch (e) {
      console.error("Load session:", e)
    }
  }

  async function loadRuntimeThread(sessionId: string) {
    try {
      const view = await invoke<RuntimeThreadView | null>("load_runtime_thread", { sessionId })
      runtimeTurns.value = view?.turns || []
      runtimeEvents.value = view?.events || []
      const runningTurns = runtimeTurns.value.filter((turn) => turn.status === "running")
      activeTurnId.value = runningTurns.length ? runningTurns[runningTurns.length - 1].id : null
      runtimeStatus.value = view?.events?.length
        ? view.events[view.events.length - 1].summary
        : "Ready"
    } catch (e) {
      console.error("Load runtime thread:", e)
      runtimeTurns.value = []
      runtimeEvents.value = []
      runtimeStatus.value = "Runtime unavailable"
      activeTurnId.value = null
    }
  }

  async function refreshSessions() {
    try {
      sessions.value = await invoke<SessionSummary[]>("list_sessions")
    } catch (e) {
      console.error("List sessions:", e)
    }
  }

  async function deleteSession(id: string) {
    if (pending.value) return

    const wasCurrent = currentSessionId.value === id

    try {
      await invoke("delete_session", { sessionId: id })
      await refreshSessions()

      if (!sessions.value.length) {
        currentSessionId.value = null
        persistString(CHAT_CURRENT_SESSION_KEY, "")
        resetSessionState()
        await createSession("新对话")
        return
      }

      if (wasCurrent) {
        await loadSession(sessions.value[0].id)
      }
    } catch (e) {
      console.error("Delete session:", e)
      error.value = String(e)
    }
  }

  async function updateSessionMeta(
    sessionId: string,
    patch: {
      name?: string
      category?: string
    }
  ) {
    try {
      const summary = await invoke<SessionSummary>("update_session_meta", {
        sessionId,
        name: patch.name ?? null,
        category: patch.category ?? null,
      })
      const index = sessions.value.findIndex((session) => session.id === sessionId)
      if (index >= 0) {
        sessions.value[index] = summary
      } else {
        await refreshSessions()
      }
      return summary
    } catch (e) {
      console.error("Update session meta:", e)
      error.value = String(e)
      throw e
    }
  }

  function setPanel(next: ChatPanel) {
    panel.value = next
    persistString(CHAT_PANEL_KEY, next)
  }

  function setSelectedCategory(category: string) {
    selectedCategory.value = category
    persistString(CHAT_CATEGORY_KEY, category)
  }

  function addCustomCategory(name: string) {
    const trimmed = name.trim()
    if (!trimmed) return
    if (!customCategories.value.includes(trimmed)) {
      customCategories.value = [...customCategories.value, trimmed].sort((a, b) =>
        a.localeCompare(b, "zh-CN")
      )
      persistArray(CHAT_CUSTOM_CATEGORIES_KEY, customCategories.value)
    }
    setSelectedCategory(trimmed)
  }

  async function sendMessage(content: string, agentProfile?: AgentProfile) {
    if (!currentSessionId.value || pending.value) return

    // user message
    messages.value.push({
      id: nextId(),
      role: "user",
      content,
      timestamp: Date.now(),
    })
    pending.value = true
    error.value = null
    streamingContent.value = ""

    // Start listening for stream events
    unlistenStream = await listen<StreamChunk>("chat-stream", (event) => {
      if (event.payload.content_delta) {
        streamingContent.value += event.payload.content_delta
      }
    })

    try {
      const response = await invoke<BackendMessage>("send_message", {
        sessionId: currentSessionId.value,
        content,
        agentProfile: agentProfile
          ? {
              id: agentProfile.id,
              name: agentProfile.name,
              role: agentProfile.role,
              persona: agentProfile.persona,
              memoryScope: serializeMemoryScope(agentProfile.memoryScope),
            }
          : null,
      })
      // If we streamed content, use that; otherwise use full response
      if (streamingContent.value) {
        const streamedMsg: ChatMsg = backendMsgToChatMsg(response)
        streamedMsg.content = streamingContent.value
        messages.value.push(streamedMsg)
      } else {
        messages.value.push(backendMsgToChatMsg(response))
      }
      await loadRuntimeThread(currentSessionId.value)
      await refreshSessions()
    } catch (e) {
      console.error("Send message:", e)
      error.value = String(e)
      // if streaming yielded content, keep it
      if (streamingContent.value) {
        messages.value.push({
          id: nextId(),
          role: "agent",
          content: streamingContent.value + `\n\n[错误: ${e}]`,
          agentName: agentProfile?.name || "OhMyWu",
          agentIcon: agentIconFor(agentProfile?.id),
          timestamp: Date.now(),
        })
      } else {
        messages.value.push({
          id: nextId(),
          role: "agent",
          content: `出错了：${e}`,
          agentName: agentProfile?.name || "OhMyWu",
          agentIcon: agentIconFor(agentProfile?.id),
          timestamp: Date.now(),
        })
      }
    } finally {
      // cleanup
      if (unlistenStream) {
        unlistenStream()
        unlistenStream = null
      }
      streamingContent.value = ""
      pending.value = false
    }
  }

  async function setAgentMode(mode: AgentMode) {
    try {
      agentMode.value = await invoke<AgentMode>("set_agent_mode", { mode })
      runtimeStatus.value = `Mode: ${agentMode.value}`
    } catch (e) {
      console.error("Set agent mode:", e)
      error.value = String(e)
    }
  }

  async function cancelAgent() {
    try {
      await invoke("cancel_agent")
    } catch (e) {
      console.error("Cancel agent:", e)
    }
  }

  function updateMemoryCandidate(turnId: string, patch: Partial<MemoryCandidate>) {
    const existing = memoryCandidates.value[turnId]
    if (!existing) return
    memoryCandidates.value = {
      ...memoryCandidates.value,
      [turnId]: {
        ...existing,
        ...patch,
      },
    }
  }

  function clearMemoryCandidate(turnId: string) {
    const nextCandidates = { ...memoryCandidates.value }
    const nextErrors = { ...memoryErrors.value }
    const nextSaved = { ...memorySaved.value }
    delete nextCandidates[turnId]
    delete nextErrors[turnId]
    delete nextSaved[turnId]
    memoryCandidates.value = nextCandidates
    memoryErrors.value = nextErrors
    memorySaved.value = nextSaved
  }

  async function generateMemoryCandidate(turnId: string) {
    if (!currentSessionId.value || memoryGenerating.value[turnId]) return
    memoryGenerating.value = { ...memoryGenerating.value, [turnId]: true }
    memoryErrors.value = { ...memoryErrors.value, [turnId]: null }

    try {
      const candidate = await invoke<MemoryCandidate>("generate_memory_candidate", {
        sessionId: currentSessionId.value,
        turnId,
      })
      memoryCandidates.value = {
        ...memoryCandidates.value,
        [turnId]: candidate,
      }
      memorySaved.value = {
        ...memorySaved.value,
        [turnId]: null,
      }
    } catch (e) {
      console.error("Generate memory candidate:", e)
      memoryErrors.value = {
        ...memoryErrors.value,
        [turnId]: String(e),
      }
    } finally {
      memoryGenerating.value = { ...memoryGenerating.value, [turnId]: false }
    }
  }

  async function saveMemoryCandidate(turnId: string) {
    if (!currentSessionId.value || memorySaving.value[turnId]) return
    const candidate = memoryCandidates.value[turnId]
    if (!candidate) return
    memorySaving.value = { ...memorySaving.value, [turnId]: true }
    memoryErrors.value = { ...memoryErrors.value, [turnId]: null }

    try {
      const note = await invoke<SavedMemoryNote>("save_memory_candidate", {
        sessionId: currentSessionId.value,
        turnId,
        candidate,
      })
      memorySaved.value = {
        ...memorySaved.value,
        [turnId]: note,
      }
    } catch (e) {
      console.error("Save memory candidate:", e)
      memoryErrors.value = {
        ...memoryErrors.value,
        [turnId]: String(e),
      }
    } finally {
      memorySaving.value = { ...memorySaving.value, [turnId]: false }
    }
  }

  return {
    messages,
    sessions,
    currentSessionId,
    pending,
    error,
    streamingContent,
    agentMode,
    runtimeTurns,
    runtimeEvents,
    runtimeStatus,
    activeTurnId,
    panel,
    selectedCategory,
    customCategories,
    memoryCandidates,
    memoryGenerating,
    memorySaving,
    memoryErrors,
    memorySaved,
    latestRuntimeEvent,
    runtimeByTurnId,
    currentSession,
    sessionCategories,
    filteredSessions,
    init,
    createSession,
    loadSession,
    loadRuntimeThread,
    refreshSessions,
    deleteSession,
    updateSessionMeta,
    setPanel,
    setSelectedCategory,
    addCustomCategory,
    sendMessage,
    setAgentMode,
    cancelAgent,
    updateMemoryCandidate,
    clearMemoryCandidate,
    generateMemoryCandidate,
    saveMemoryCandidate,
  }
})
