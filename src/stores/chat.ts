import { defineStore } from "pinia"
import { ref, computed } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

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

export type AgentMode = "plan" | "agent" | "auto"

export interface ExecutionInfo {
  action: string
  status: "running" | "success" | "failed"
  input?: string
  output?: string
  error?: string
  duration?: string
}

export interface SessionSummary {
  id: string
  name: string
  message_count: number
  created_at: string
  updated_at: string
}

interface BackendMessage {
  role: string
  content: string
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
  threadId?: string
  turnId?: string | null
  kind: string
  summary: string
  timestamp?: string | null
  sessionId?: string
  status?: string
}

interface RuntimeThreadView {
  turns: RuntimeTurn[]
  events: RuntimeEvent[]
}

let _msgId = 0
function nextId() {
  return `msg-${++_msgId}`
}

function backendMsgToChatMsg(msg: BackendMessage): ChatMsg {
  return {
    id: nextId(),
    role: msg.role as "user" | "agent",
    content: msg.content,
    agentName: msg.role === "agent" ? "OhMyWu" : undefined,
    agentIcon: msg.role === "agent" ? "🤖" : undefined,
    execs: msg.executions?.map((e: BackendExec) => ({
      action: e.capability,
      status: e.status as "success" | "failed",
      input: e.input,
      output: e.output,
      error: e.error,
      duration: `${(e.duration_ms / 1000).toFixed(1)}s`,
    })),
    taskId: msg.task_id,
    timestamp: msg.timestamp ? new Date(msg.timestamp).getTime() : Date.now(),
  }
}

export const useChatStore = defineStore("chat", () => {
  const messages = ref<ChatMsg[]>([])
  const sessions = ref<SessionSummary[]>([])
  const currentSessionId = ref<string | null>(null)
  const pending = ref(false)
  const error = ref<string | null>(null)
  const streamingContent = ref("")
  const agentMode = ref<AgentMode>("agent")
  const runtimeTurns = ref<RuntimeTurn[]>([])
  const runtimeEvents = ref<RuntimeEvent[]>([])
  const runtimeStatus = ref("Ready")

  let unlistenStream: UnlistenFn | null = null
  let unlistenRuntime: UnlistenFn | null = null

  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value)
  )
  const latestRuntimeEvent = computed(() =>
    runtimeEvents.value.length ? runtimeEvents.value[runtimeEvents.value.length - 1] : null
  )

  async function init() {
    try {
      agentMode.value = await invoke<AgentMode>("get_agent_mode")
      if (!unlistenRuntime) {
        unlistenRuntime = await listen<RuntimeEvent>("runtime-event", (event) => {
          const payload = event.payload
          if (payload.sessionId && payload.sessionId !== currentSessionId.value) return
          runtimeEvents.value.push(payload)
          runtimeStatus.value = payload.summary || payload.kind
        })
      }
      const list = await invoke<SessionSummary[]>("list_sessions")
      sessions.value = list
      if (list.length > 0) {
        currentSessionId.value = list[0].id
        await loadSession(list[0].id)
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
      const summary = await invoke<SessionSummary>("create_session", { name })
      sessions.value.unshift(summary)
      currentSessionId.value = summary.id
      messages.value = []
      runtimeTurns.value = []
      runtimeEvents.value = []
      runtimeStatus.value = "Ready"
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
      runtimeStatus.value = view?.events?.length
        ? view.events[view.events.length - 1].summary
        : "Ready"
    } catch (e) {
      console.error("Load runtime thread:", e)
      runtimeTurns.value = []
      runtimeEvents.value = []
      runtimeStatus.value = "Runtime unavailable"
    }
  }

  async function refreshSessions() {
    try {
      sessions.value = await invoke<SessionSummary[]>("list_sessions")
    } catch (e) {
      console.error("List sessions:", e)
    }
  }

  async function sendMessage(content: string) {
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
          agentName: "OhMyWu",
          agentIcon: "🤖",
          timestamp: Date.now(),
        })
      } else {
        messages.value.push({
          id: nextId(),
          role: "agent",
          content: `出错了：${e}`,
          agentName: "OhMyWu",
          agentIcon: "🤖",
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
    latestRuntimeEvent,
    currentSession,
    init,
    createSession,
    loadSession,
    loadRuntimeThread,
    refreshSessions,
    sendMessage,
    setAgentMode,
    cancelAgent,
  }
})
