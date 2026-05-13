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

  let unlistenStream: UnlistenFn | null = null

  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value)
  )

  async function init() {
    try {
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
    } catch (e) {
      console.error("Load session:", e)
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

  return {
    messages,
    sessions,
    currentSessionId,
    pending,
    error,
    streamingContent,
    currentSession,
    init,
    createSession,
    loadSession,
    refreshSessions,
    sendMessage,
  }
})
