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
    const text = await response.text()
    throw new Error(text || `request failed: ${response.status}`)
  }

  const contentType = response.headers.get('content-type')
  if (!contentType?.includes('application/json')) {
    throw new Error('Invalid response format')
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

export type ServiceAction = 'start' | 'stop' | 'restart'

async function postVoid(path: string): Promise<void> {
  const response = await fetch(`${daemonOrigin}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
  })
  if (!response.ok) {
    throw new Error(`request failed: ${response.status}`)
  }
}

export async function killProcess(pid: number): Promise<void> {
  await postVoid(`/api/processes/${pid}/kill`)
}

export async function controlService(name: string, action: ServiceAction): Promise<void> {
  await postVoid(`/api/services/${name}/${action}`)
}

// ── M3: Storage Cleanup ────────────────────────────────────────────────────────

export type CleanupItem = {
  path: string
  size_bytes: number
  human_size: string
  category: 'cache' | 'temp' | 'log' | 'package_cache' | 'toolchain' | 'large_dir' | 'other'
  reason: string
  risk: 'safe' | 'risky'
  risk_reason: string
  executable: boolean
  execution_block_reason: string
}

export type CleanupPreviewResult = {
  items: CleanupItem[]
  total_bytes: number
  human_total: string
  scanned_path: string
}

export async function fetchCleanupPreview(preset: 'common' | 'custom', path?: string): Promise<CleanupPreviewResult> {
  return request<CleanupPreviewResult>('/api/cleanup/preview', {
    method: 'POST',
    body: JSON.stringify({ preset, path }),
  })
}

export async function executeCleanup(paths: string[]): Promise<{ freed_bytes: number }> {
  return request<{ freed_bytes: number }>('/api/cleanup/execute', {
    method: 'POST',
    body: JSON.stringify({ paths }),
  })
}

export type CleanupNode = {
  path: string
  name: string
  size_bytes: number
  human_size: string
  category: string
  level: string
  reason: string
  children?: CleanupNode[]
}

export type CleanupTreeResult = {
  root: CleanupNode
  total_bytes: number
  human_total: string
}

export async function fetchCleanupTree(): Promise<CleanupTreeResult> {
  return request<CleanupTreeResult>('/api/cleanup/tree', {
    method: 'POST',
    body: JSON.stringify({}),
  })
}

export async function fetchScanPath(path: string): Promise<CleanupTreeResult> {
  return request<CleanupTreeResult>('/api/cleanup/scan-path', {
    method: 'POST',
    body: JSON.stringify({ path }),
  })
}

export type Task = {
  id: string
  action: string
  target: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  created_at: string
  completed_at: string | null
  result: string | null
}

export type AuditEvent = {
  id: string
  timestamp: string
  actor: string
  action: string
  target: string
  risk_level: 'read_only' | 'controlled_write' | 'high_risk'
  result: string
  detail: string | null
}

export function fetchTasks(): Promise<Task[]> {
  return request<Task[]>('/api/tasks')
}

export function fetchAudits(limit = 100): Promise<AuditEvent[]> {
  return request<AuditEvent[]>(`/api/audits?limit=${limit}`)
}
