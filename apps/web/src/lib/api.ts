export type HealthResponse = {
  app: string
  status: string
  version: string
}

export type ActionSpec = {
  name: string
  title: string
  summary: string
  domain: string
  risk_level: 'read_only' | 'controlled_write' | 'high_risk'
  target: {
    kind: 'tool' | 'workflow' | 'skill' | 'agent_template'
    name: string
  }
}

export type ResolvedReference = {
  raw: string
  kind: 'action' | 'agent'
  name: string
  exists: boolean
}

export type ReferenceResolutionResponse = {
  input: string
  references: ResolvedReference[]
}

const daemonOrigin = import.meta.env.VITE_DAEMON_ORIGIN ?? 'http://127.0.0.1:3000'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${daemonOrigin}${path}`, {
    headers: {
      'Content-Type': 'application/json',
    },
    ...init,
  })

  if (!response.ok) {
    throw new Error(`request failed: ${response.status}`)
  }

  return response.json() as Promise<T>
}

export function fetchHealth() {
  return request<HealthResponse>('/api/health')
}

export function fetchActions() {
  return request<ActionSpec[]>('/api/actions')
}

export function resolveReferences(input: string) {
  return request<ReferenceResolutionResponse>('/api/references/resolve', {
    method: 'POST',
    body: JSON.stringify({ input }),
  })
}

export function fetchSystemOverview() {
  return request<Record<string, string>>('/api/system/overview')
}

export type ProcessInfo = {
  pid: number
  name: string
  state: string
  cpu_percent: number
  memory_kb: number
  user: string
  command: string
}

export function fetchProcesses() {
  return request<ProcessInfo[]>('/api/processes')
}

export type ServiceInfo = {
  name: string
  load_state: string
  active_state: string
  sub_state: string
  description: string
  enabled: boolean
}

export function fetchServices() {
  return request<ServiceInfo[]>('/api/services')
}

export type StorageEntry = {
  path: string
  size_bytes: number
  human_size: string
}

export type StorageScanResult = {
  root_path: string
  total_bytes: number
  entries: StorageEntry[]
  scanned_at: string
}

export type StorageScanQuery = {
  path?: string
  depth?: number
}

export function scanStorage(query: StorageScanQuery) {
  return request<StorageScanResult>('/api/storage/scans', {
    method: 'POST',
    body: JSON.stringify(query),
  })
}

export type JournalEntry = {
  timestamp: string
  unit: string
  priority: number
  message: string
}

export type JournalQuery = {
  unit?: string
  priority?: number
  lines?: number
}

export function fetchJournal(query: JournalQuery) {
  return request<JournalEntry[]>('/api/logs/journal', {
    method: 'POST',
    body: JSON.stringify(query),
  })
}
