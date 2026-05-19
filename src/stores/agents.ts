import { defineStore } from "pinia"
import { ref, watch } from "vue"

export const MEMORY_SCOPE_FOLDERS = ["concepts", "notes", "daily", "profile"] as const

export type MemoryScopeFolder = (typeof MEMORY_SCOPE_FOLDERS)[number]
export type MemoryScopeMode = "none" | "all" | "focused"

export const MEMORY_SCOPE_FOLDER_LABELS: Record<MemoryScopeFolder, string> = {
  concepts: "概念",
  notes: "笔记",
  daily: "每日",
  profile: "画像",
}

export interface AgentMemoryScope {
  version: 1
  label: string
  mode: MemoryScopeMode
  folders: MemoryScopeFolder[]
  recallLimit: number
  notes: string
}

export interface AgentProfile {
  id: string
  name: string
  role: string
  persona: string
  memoryScope: AgentMemoryScope
  tools: string[]
  primary: boolean
}

interface LegacyAgentProfile {
  id?: string
  name?: string
  role?: string
  persona?: string
  memoryScope?: unknown
  tools?: unknown
  primary?: boolean
}

const STORAGE_KEY = "ohmywu.agent-profiles.v2"
const LEGACY_STORAGE_KEY = "ohmywu.agent-profiles.v1"
const ACTIVE_AGENT_KEY = "ohmywu.active-agent.v1"

export function createMemoryScope(overrides: Partial<AgentMemoryScope> = {}): AgentMemoryScope {
  const mode = normalizeMemoryScopeMode(overrides.mode)
  const folders = normalizeMemoryScopeFolders(overrides.folders)
  const finalFolders = mode === "all"
    ? [...MEMORY_SCOPE_FOLDERS]
    : mode === "none"
      ? []
      : folders

  return {
    version: 1,
    label: overrides.label?.trim() || defaultMemoryScopeLabel(mode, finalFolders),
    mode,
    folders: finalFolders,
    recallLimit: clampRecallLimit(overrides.recallLimit),
    notes: overrides.notes?.trim() || "",
  }
}

function createAgentProfile(overrides: Partial<AgentProfile> = {}): AgentProfile {
  return {
    id: overrides.id || `agent-${Date.now()}`,
    name: overrides.name || "新 Agent",
    role: overrides.role || "自定义角色",
    persona: overrides.persona || "按当前角色稳定执行，优先清晰、可审计和低噪音。",
    memoryScope: normalizeMemoryScope(overrides.memoryScope),
    tools: normalizeTools(overrides.tools),
    primary: overrides.primary ?? false,
  }
}

function defaultAgents(): AgentProfile[] {
  return [
    createAgentProfile({
      id: "core",
      name: "主 Agent（无记忆）",
      role: "通用执行 / 零记忆",
      persona: "稳健、可审计、优先把任务拆清楚再执行，不主动携带长期记忆，适合做默认入口和调度。",
      memoryScope: createMemoryScope({
        mode: "none",
        label: "禁用长期记忆",
        notes: "适合纯即时任务，不注入历史知识。",
      }),
      tools: ["read", "grep", "glob", "bash", "wiki_read", "wiki_search"],
      primary: true,
    }),
    createAgentProfile({
      id: "memory",
      name: "记忆 Agent（大量记忆）",
      role: "知识整理 / 长期记忆",
      persona: "更偏总结、归档、提炼长期上下文，减少噪音，强调结构化知识沉淀与记忆召回。",
      memoryScope: createMemoryScope({
        mode: "focused",
        label: "长期偏好与知识沉淀",
        folders: ["concepts", "profile", "daily"],
        recallLimit: 6,
        notes: "优先召回长期偏好、项目决策和近期沉淀。",
      }),
      tools: ["read", "wiki_read", "wiki_search", "wiki_write"],
    }),
    createAgentProfile({
      id: "coder",
      name: "编码 Agent（纯编码）",
      role: "纯编码 / 工程实现",
      persona: "偏工程实现，优先读代码、改代码、跑构建，避免无关发散。",
      memoryScope: createMemoryScope({
        mode: "focused",
        label: "工程上下文",
        folders: ["notes", "concepts"],
        recallLimit: 4,
        notes: "优先使用项目笔记、技术概念和实现约定。",
      }),
      tools: ["read", "grep", "glob", "edit", "write", "bash"],
    }),
  ]
}

export function normalizeMemoryScope(input: unknown): AgentMemoryScope {
  if (!input) {
    return createMemoryScope({ mode: "none" })
  }

  if (typeof input === "string") {
    return parseLegacyMemoryScope(input)
  }

  if (typeof input === "object") {
    const raw = input as Partial<AgentMemoryScope>
    return createMemoryScope({
      label: typeof raw.label === "string" ? raw.label : undefined,
      mode: raw.mode,
      folders: Array.isArray(raw.folders) ? raw.folders : undefined,
      recallLimit: typeof raw.recallLimit === "number" ? raw.recallLimit : undefined,
      notes: typeof raw.notes === "string" ? raw.notes : undefined,
    })
  }

  return createMemoryScope({ mode: "none" })
}

function parseLegacyMemoryScope(scope: string): AgentMemoryScope {
  const scopeLower = scope.toLowerCase()
  if (!scopeLower.trim() || scopeLower.includes("none") || scopeLower.includes("无记忆")) {
    return createMemoryScope({
      mode: "none",
      label: "禁用长期记忆",
    })
  }

  if (scopeLower.includes("all") || scopeLower.includes("全部")) {
    return createMemoryScope({
      mode: "all",
      label: "全部知识",
      recallLimit: 6,
    })
  }

  const folders = MEMORY_SCOPE_FOLDERS.filter((folder) => scopeLower.includes(folder))
  return createMemoryScope({
    mode: folders.length ? "focused" : "none",
    folders,
    label: folders.length ? defaultMemoryScopeLabel("focused", folders) : "禁用长期记忆",
  })
}

function normalizeMemoryScopeMode(mode: unknown): MemoryScopeMode {
  if (mode === "all" || mode === "focused" || mode === "none") {
    return mode
  }
  return "focused"
}

function normalizeMemoryScopeFolders(folders: unknown): MemoryScopeFolder[] {
  if (!Array.isArray(folders)) return []
  return [...new Set(
    folders.filter((folder): folder is MemoryScopeFolder =>
      typeof folder === "string" && MEMORY_SCOPE_FOLDERS.includes(folder as MemoryScopeFolder)
    )
  )]
}

function clampRecallLimit(value: unknown): number {
  const num = typeof value === "number" ? value : Number(value)
  if (!Number.isFinite(num)) return 4
  return Math.max(1, Math.min(8, Math.round(num)))
}

export function defaultMemoryScopeLabel(
  mode: MemoryScopeMode,
  folders: readonly MemoryScopeFolder[]
): string {
  if (mode === "none") return "禁用长期记忆"
  if (mode === "all") return "全部知识"
  if (!folders.length) return "定向记忆"
  return folders.map((folder) => MEMORY_SCOPE_FOLDER_LABELS[folder]).join(" / ")
}

function normalizeTools(input: unknown): string[] {
  if (!Array.isArray(input)) return []
  return [...new Set(input.filter((item): item is string => typeof item === "string" && item.trim().length > 0))]
}

function normalizeAgentProfile(input: LegacyAgentProfile, index: number): AgentProfile {
  return createAgentProfile({
    id: typeof input.id === "string" && input.id.trim() ? input.id : `agent-${index + 1}`,
    name: typeof input.name === "string" && input.name.trim() ? input.name : `Agent ${index + 1}`,
    role: typeof input.role === "string" && input.role.trim() ? input.role : "自定义角色",
    persona: typeof input.persona === "string" && input.persona.trim()
      ? input.persona
      : "按当前角色稳定执行，优先清晰、可审计和低噪音。",
    memoryScope: normalizeMemoryScope(input.memoryScope),
    tools: normalizeTools(input.tools),
    primary: input.primary === true,
  })
}

function ensurePrimary(agents: AgentProfile[]): AgentProfile[] {
  if (!agents.length) {
    return defaultAgents()
  }
  if (agents.some((agent) => agent.primary)) {
    return agents
  }
  return agents.map((agent, index) => ({
    ...agent,
    primary: index === 0,
  }))
}

function loadAgents(): AgentProfile[] {
  if (typeof window === "undefined") {
    return defaultAgents()
  }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY) || window.localStorage.getItem(LEGACY_STORAGE_KEY)
    if (!raw) return defaultAgents()
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return defaultAgents()
    return ensurePrimary(parsed.map((agent, index) => normalizeAgentProfile(agent, index)))
  } catch {
    return defaultAgents()
  }
}

function uniqueAgentId(base: string, existingIds: Set<string>): string {
  let next = base
  let index = 2
  while (existingIds.has(next)) {
    next = `${base}-${index}`
    index += 1
  }
  return next
}

export function summarizeMemoryScope(scope: AgentMemoryScope): string {
  if (scope.mode === "none") {
    return "不注入长期记忆"
  }
  if (scope.mode === "all") {
    return `全部知识 · ${scope.recallLimit} 条召回`
  }
  const folderText = scope.folders.length
    ? scope.folders.map((folder) => MEMORY_SCOPE_FOLDER_LABELS[folder]).join(" / ")
    : "未选择目录"
  return `${folderText} · ${scope.recallLimit} 条召回`
}

export function serializeMemoryScope(scope: AgentMemoryScope): string {
  return JSON.stringify({
    version: 1,
    label: scope.label,
    mode: scope.mode,
    folders: scope.folders,
    recallLimit: scope.recallLimit,
    notes: scope.notes,
  })
}

export const useAgentStore = defineStore("agents", () => {
  const agents = ref<AgentProfile[]>(loadAgents())
  const activeAgentId = ref<string>(
    typeof window === "undefined"
      ? "core"
      : window.localStorage.getItem(ACTIVE_AGENT_KEY) || "core"
  )

  watch(
    agents,
    (value) => {
      if (typeof window === "undefined") return
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
      window.localStorage.removeItem(LEGACY_STORAGE_KEY)
    },
    { deep: true }
  )

  watch(activeAgentId, (value) => {
    if (typeof window === "undefined") return
    window.localStorage.setItem(ACTIVE_AGENT_KEY, value)
  })

  function setActiveAgent(id: string) {
    if (!agents.value.some((agent) => agent.id === id)) return
    activeAgentId.value = id
  }

  function addAgent() {
    const existingIds = new Set(agents.value.map((agent) => agent.id))
    const id = uniqueAgentId("custom-agent", existingIds)
    const agent = createAgentProfile({
      id,
      name: "自定义 Agent",
      role: "自定义角色",
      memoryScope: createMemoryScope({
        mode: "focused",
        label: "定向记忆",
        folders: ["notes"],
        recallLimit: 4,
      }),
      tools: ["read", "grep", "glob", "wiki_read", "wiki_search"],
    })
    agents.value = [...agents.value, agent]
    activeAgentId.value = agent.id
  }

  function duplicateAgent(id: string) {
    const source = agents.value.find((agent) => agent.id === id)
    if (!source) return
    const existingIds = new Set(agents.value.map((agent) => agent.id))
    const nextId = uniqueAgentId(`${source.id}-copy`, existingIds)
    const clone = createAgentProfile({
      ...source,
      id: nextId,
      name: `${source.name} 副本`,
      primary: false,
      memoryScope: createMemoryScope(source.memoryScope),
      tools: [...source.tools],
    })
    agents.value = [...agents.value, clone]
    activeAgentId.value = clone.id
  }

  function removeAgent(id: string) {
    const target = agents.value.find((agent) => agent.id === id)
    if (!target || target.primary) return
    const next = ensurePrimary(agents.value.filter((agent) => agent.id !== id))
    agents.value = next
    if (activeAgentId.value === id) {
      activeAgentId.value = next[0]?.id || "core"
    }
  }

  function reset() {
    agents.value = defaultAgents()
    activeAgentId.value = "core"
  }

  return {
    agents,
    activeAgentId,
    setActiveAgent,
    addAgent,
    duplicateAgent,
    removeAgent,
    reset,
  }
})
