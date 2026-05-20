import { invoke } from "@tauri-apps/api/core"
import { defineStore } from "pinia"
import { computed, ref, watch } from "vue"

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
  delegateTags: string[]
  delegateNote: string
  delegatable: boolean
  delegatePriority: number
  primary: boolean
  editable: boolean
  deletable: boolean
  persistedId?: string
}

interface BackendAgentView {
  id: string
  name: string
  role: string
  persona: string
  memoryScope: string
  tools: string[]
  delegateTags: string[]
  delegateNote: string
  delegatable: boolean
  delegatePriority: number
  primary: boolean
  editable: boolean
  deletable: boolean
}

interface AgentUpsertInput {
  existingId: string | null
  id: string
  name: string
  role: string
  persona: string
  memoryScope: string
  tools: string[]
  delegateTags: string[]
  delegateNote: string
  delegatable: boolean
  delegatePriority: number
}

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
    delegateTags: normalizeTags(overrides.delegateTags),
    delegateNote: overrides.delegateNote?.trim() || "",
    delegatable: overrides.delegatable ?? false,
    delegatePriority: clampDelegatePriority(overrides.delegatePriority),
    primary: overrides.primary ?? false,
    editable: overrides.editable ?? true,
    deletable: overrides.deletable ?? !overrides.primary,
    persistedId: overrides.persistedId,
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
      delegateTags: ["通用", "调度", "拆解", "执行"],
      delegateNote: "默认入口。适合先理解需求、拆任务、串联其他 agent，不适合大量长期记忆召回。",
      delegatable: false,
      delegatePriority: 0,
      primary: true,
      editable: true,
      deletable: false,
      persistedId: "core",
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
      delegateTags: ["记忆", "知识库", "归档", "总结", "复盘"],
      delegateNote: "适合总结、长期知识沉淀、个人偏好整理、记忆候选和复盘归档。",
      delegatable: true,
      delegatePriority: 70,
      editable: true,
      deletable: true,
      persistedId: "memory",
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
      delegateTags: ["代码", "修复", "构建", "测试", "前端", "后端"],
      delegateNote: "适合读代码、改代码、构建检查、测试失败排查和工程实现。",
      delegatable: true,
      delegatePriority: 90,
      editable: true,
      deletable: true,
      persistedId: "coder",
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
  try {
    const parsed = JSON.parse(scope)
    if (parsed && typeof parsed === "object") {
      return normalizeMemoryScope(parsed)
    }
  } catch {
    // fall through to legacy text parsing
  }

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

function normalizeTags(input: unknown): string[] {
  if (!Array.isArray(input)) return []
  return [...new Set(input.filter((item): item is string => typeof item === "string" && item.trim().length > 0))]
}

function clampDelegatePriority(input: unknown): number {
  const value = typeof input === "number" ? input : Number(input)
  if (!Number.isFinite(value)) return 50
  return Math.max(0, Math.min(100, Math.round(value)))
}

function backendToAgentProfile(input: BackendAgentView, index: number): AgentProfile {
  return createAgentProfile({
    id: typeof input.id === "string" && input.id.trim() ? input.id : `agent-${index + 1}`,
    name: typeof input.name === "string" && input.name.trim() ? input.name : `Agent ${index + 1}`,
    role: typeof input.role === "string" && input.role.trim() ? input.role : "自定义角色",
    persona: typeof input.persona === "string" && input.persona.trim()
      ? input.persona
      : "按当前角色稳定执行，优先清晰、可审计和低噪音。",
    memoryScope: normalizeMemoryScope(input.memoryScope),
    tools: normalizeTools(input.tools),
    delegateTags: normalizeTags(input.delegateTags),
    delegateNote: typeof input.delegateNote === "string" ? input.delegateNote : "",
    delegatable: input.delegatable === true,
    delegatePriority: clampDelegatePriority(input.delegatePriority),
    primary: input.primary === true,
    editable: input.editable !== false,
    deletable: input.deletable === true,
    persistedId: input.id,
  })
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

function persistActiveAgent(id: string) {
  if (typeof window === "undefined") return
  window.localStorage.setItem(ACTIVE_AGENT_KEY, id)
}

function loadActiveAgent(): string {
  if (typeof window === "undefined") return "core"
  return window.localStorage.getItem(ACTIVE_AGENT_KEY) || "core"
}

function snapshotAgent(agent: AgentProfile): string {
  return JSON.stringify({
    id: agent.id,
    name: agent.name,
    role: agent.role,
    persona: agent.persona,
    memoryScope: serializeMemoryScope(agent.memoryScope),
    tools: [...agent.tools].sort(),
    delegateTags: [...agent.delegateTags].sort(),
    delegateNote: agent.delegateNote,
    delegatable: agent.delegatable,
    delegatePriority: agent.delegatePriority,
    primary: agent.primary,
  })
}

function upsertPayload(agent: AgentProfile): AgentUpsertInput {
  return {
    existingId: agent.persistedId || null,
    id: agent.id.trim(),
    name: agent.name.trim(),
    role: agent.role.trim(),
    persona: agent.persona.trim(),
    memoryScope: serializeMemoryScope(agent.memoryScope),
    tools: normalizeTools(agent.tools),
    delegateTags: normalizeTags(agent.delegateTags),
    delegateNote: agent.delegateNote.trim(),
    delegatable: agent.delegatable,
    delegatePriority: clampDelegatePriority(agent.delegatePriority),
  }
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
  const agents = ref<AgentProfile[]>([])
  const activeAgentId = ref<string>(loadActiveAgent())
  const loading = ref(false)
  const loaded = ref(false)
  const syncError = ref("")
  const saving = ref<Record<string, boolean>>({})

  let initPromise: Promise<void> | null = null
  let hydrating = false
  const snapshots = new Map<string, string>()
  const saveTimers = new Map<string, number>()

  const availableAgents = computed(() => agents.value)

  function rememberSnapshots(items: AgentProfile[]) {
    snapshots.clear()
    for (const agent of items) {
      snapshots.set(agent.persistedId || agent.id, snapshotAgent(agent))
    }
  }

  function applyAgents(items: AgentProfile[]) {
    hydrating = true
    agents.value = items
    if (!items.some((agent) => agent.id === activeAgentId.value)) {
      activeAgentId.value = items[0]?.id || "core"
    }
    persistActiveAgent(activeAgentId.value)
    rememberSnapshots(items)
    hydrating = false
  }

  async function refresh() {
    loading.value = true
    syncError.value = ""
    try {
      const list = await invoke<BackendAgentView[]>("get_agents")
      applyAgents(list.map(backendToAgentProfile))
      loaded.value = true
    } catch (error) {
      console.error("load agents:", error)
      syncError.value = String(error)
      if (!loaded.value) {
        applyAgents(defaultAgents())
        loaded.value = true
      }
    } finally {
      loading.value = false
    }
  }

  async function init(force = false) {
    if (loaded.value && !force) return
    if (!force && initPromise) {
      await initPromise
      return
    }
    initPromise = refresh().finally(() => {
      initPromise = null
    })
    await initPromise
  }

  async function persistAgent(agent: AgentProfile) {
    const agentKey = agent.persistedId || agent.id
    const payload = upsertPayload(agent)
    saving.value = { ...saving.value, [agentKey]: true }
    try {
      const list = await invoke<BackendAgentView[]>("upsert_agent", { input: payload })
      const updated = list.map(backendToAgentProfile)
      const next = updated.find((item) => item.id === payload.id)
      if (next) {
        agent.persistedId = next.id
        agent.primary = next.primary
        agent.editable = next.editable
        agent.deletable = next.deletable
      } else {
        agent.persistedId = payload.id
      }
      snapshots.delete(agentKey)
      snapshots.set(agent.persistedId || agent.id, snapshotAgent(agent))
      syncError.value = ""
    } catch (error) {
      console.error("persist agent:", error)
      syncError.value = String(error)
    } finally {
      const nextSaving = { ...saving.value }
      delete nextSaving[agentKey]
      delete nextSaving[agent.persistedId || agent.id]
      saving.value = nextSaving
    }
  }

  function schedulePersist(agent: AgentProfile) {
    if (!loaded.value || hydrating) return
    const key = agent.persistedId || agent.id
    const current = snapshotAgent(agent)
    if (snapshots.get(key) === current) return

    const existing = saveTimers.get(key)
    if (existing) {
      window.clearTimeout(existing)
    }

    const timer = window.setTimeout(() => {
      saveTimers.delete(key)
      persistAgent(agent)
    }, 320)
    saveTimers.set(key, timer)
  }

  watch(activeAgentId, (value) => {
    persistActiveAgent(value)
  })

  watch(
    agents,
    (items) => {
      if (hydrating) return
      for (const agent of items) {
        schedulePersist(agent)
      }
    },
    { deep: true }
  )

  function setActiveAgent(id: string) {
    if (!availableAgents.value.some((agent) => agent.id === id)) return
    activeAgentId.value = id
  }

  async function addAgent() {
    await init()
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
      delegateTags: ["自定义"],
      delegateNote: "",
      delegatable: true,
      delegatePriority: 50,
      editable: true,
      deletable: true,
    })
    agents.value = [...agents.value, agent]
    activeAgentId.value = agent.id
    await persistAgent(agent)
  }

  async function duplicateAgent(id: string) {
    await init()
    const source = agents.value.find((agent) => agent.id === id)
    if (!source) return
    const existingIds = new Set(agents.value.map((agent) => agent.id))
    const nextId = uniqueAgentId(`${source.id}-copy`, existingIds)
    const clone = createAgentProfile({
      ...source,
      id: nextId,
      name: `${source.name} 副本`,
      primary: false,
      deletable: true,
      memoryScope: createMemoryScope(source.memoryScope),
      tools: [...source.tools],
      persistedId: undefined,
    })
    agents.value = [...agents.value, clone]
    activeAgentId.value = clone.id
    await persistAgent(clone)
  }

  async function removeAgent(id: string) {
    await init()
    const target = agents.value.find((agent) => agent.id === id)
    if (!target || !target.deletable) return

    try {
      const list = await invoke<BackendAgentView[]>("delete_agent", { id })
      applyAgents(list.map(backendToAgentProfile))
      syncError.value = ""
    } catch (error) {
      console.error("delete agent:", error)
      syncError.value = String(error)
    }
  }

  return {
    agents,
    availableAgents,
    activeAgentId,
    loading,
    loaded,
    syncError,
    saving,
    init,
    refresh,
    setActiveAgent,
    addAgent,
    duplicateAgent,
    removeAgent,
  }
})
