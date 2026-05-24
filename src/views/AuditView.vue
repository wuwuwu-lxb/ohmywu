<script setup lang="ts">
import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"
import ConfirmDialog from "../components/ConfirmDialog.vue"
import ThemeSelect from "../components/ThemeSelect.vue"

interface AuditEvent {
  session_id?: string | null
  turn_id?: string | null
  actor: string
  action: string
  target: string
  risk_level: string
  status: string
  detail?: string
  timestamp: string
}

interface SessionSummary {
  id: string
  name: string
  category: string
}

interface AuditGroup {
  id: string
  label: string
  note: string
  count: number
  events: AuditEvent[]
}

interface AuditExportResult {
  path: string
  count: number
}

const events = ref<AuditEvent[]>([])
const sessions = ref<SessionSummary[]>([])
const expandedGroups = ref<string[]>([])
const expandedRows = ref<string[]>([])
const riskFilter = ref("all")
const confirmClear = ref(false)
const clearing = ref(false)
const exportBusyId = ref<string | null>(null)
const exportMessage = ref("")

const riskOptions = [
  { label: "全部风险", value: "all" },
  { label: "只读", value: "readonly" },
  { label: "受控写入", value: "controlledwrite" },
  { label: "高风险", value: "highrisk" },
]

function groupKey(groupId: string) {
  return `group:${groupId}`
}

function rowKey(event: AuditEvent, index: number) {
  return `${event.session_id || "system"}-${event.timestamp}-${event.action}-${index}`
}

function isGroupExpanded(groupId: string) {
  return expandedGroups.value.includes(groupKey(groupId))
}

function toggleGroup(groupId: string) {
  const key = groupKey(groupId)
  expandedGroups.value = isGroupExpanded(groupId)
    ? expandedGroups.value.filter((item) => item !== key)
    : [...expandedGroups.value, key]
}

function isRowExpanded(key: string) {
  return expandedRows.value.includes(key)
}

function toggleRow(key: string) {
  expandedRows.value = isRowExpanded(key)
    ? expandedRows.value.filter((item) => item !== key)
    : [...expandedRows.value, key]
}

const filteredEvents = computed(() =>
  riskFilter.value === "all"
    ? events.value
    : events.value.filter((event) => event.risk_level.toLowerCase() === riskFilter.value)
)

const groupedEvents = computed<AuditGroup[]>(() => {
  const sessionMap = new Map(sessions.value.map((session) => [session.id, session]))
  const groups = new Map<string, AuditGroup>()

  for (const event of filteredEvents.value) {
    const sessionId = event.session_id || "__system__"
    const session = event.session_id ? sessionMap.get(event.session_id) : null
    const label = session?.name || "全局 / 系统"
    const note = session?.category?.trim()
      ? `分类：${session.category}`
      : event.session_id
        ? "未分类对话"
        : "无对话归属"

    if (!groups.has(sessionId)) {
      groups.set(sessionId, {
        id: sessionId,
        label,
        note,
        count: 0,
        events: [],
      })
    }

    const group = groups.get(sessionId)!
    group.events.push(event)
    group.count += 1
  }

  return [...groups.values()].sort((a, b) => {
    const aTs = a.events[0]?.timestamp || ""
    const bTs = b.events[0]?.timestamp || ""
    return bTs.localeCompare(aTs)
  })
})

const hasEvents = computed(() => groupedEvents.value.length > 0)

async function loadAuditData() {
  try {
    const [auditList, sessionList] = await Promise.all([
      invoke<AuditEvent[]>("get_audits"),
      invoke<SessionSummary[]>("list_sessions"),
    ])
    events.value = auditList
    sessions.value = sessionList
    expandedGroups.value = groupedEvents.value.slice(0, 3).map((group) => groupKey(group.id))
  } catch {
    // Tauri not ready in dev
  }
}

async function clearAllAudits() {
  try {
    clearing.value = true
    await invoke("clear_audits")
    confirmClear.value = false
    exportMessage.value = "审计日志已清空。"
    await loadAuditData()
  } finally {
    clearing.value = false
  }
}

async function exportAudits(sessionId?: string | null) {
  try {
    exportBusyId.value = sessionId || "__all__"
    const result = await invoke<AuditExportResult>("export_audits", {
      sessionId: sessionId || null,
    })
    exportMessage.value = `已导出 ${result.count} 条审计到 ${result.path}`
  } finally {
    exportBusyId.value = null
  }
}

onMounted(async () => {
  await loadAuditData()
})
</script>

<template>
  <div class="audit-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">审计日志</h2>
        <p class="view-subtitle">按对话查看工具执行、风险等级和最终状态，避免所有日志堆在一起。</p>
      </div>
      <div class="section-controls">
        <ThemeSelect
          class="risk-select"
          :model-value="riskFilter"
          :options="riskOptions"
          @update:model-value="(value) => riskFilter = String(value)"
        />
        <button class="toolbar-btn" type="button" @click="exportAudits(null)">
          {{ exportBusyId === "__all__" ? "导出中..." : "导出全部" }}
        </button>
        <button class="toolbar-btn danger" type="button" @click="confirmClear = true">清空</button>
        <span class="section-count">{{ filteredEvents.length }}</span>
      </div>
    </header>

    <div v-if="exportMessage" class="export-banner">{{ exportMessage }}</div>

    <div v-if="hasEvents" class="audit-groups">
      <section
        v-for="group in groupedEvents"
        :key="group.id"
        class="audit-group"
        :class="{ expanded: isGroupExpanded(group.id) }"
      >
        <button type="button" class="audit-group-head" @click="toggleGroup(group.id)">
          <div class="audit-group-copy">
            <span class="audit-group-title">{{ group.label }}</span>
            <span class="audit-group-note">{{ group.note }}</span>
          </div>
          <div class="audit-group-meta">
            <button
              type="button"
              class="group-export-btn"
              @click.stop="exportAudits(group.id)"
            >
              {{
                exportBusyId === group.id
                  ? "导出中..."
                  : "导出"
              }}
            </button>
            <span class="audit-group-count">{{ group.count }}</span>
            <span class="audit-group-chevron">{{ isGroupExpanded(group.id) ? "▾" : "▸" }}</span>
          </div>
        </button>

        <div v-if="isGroupExpanded(group.id)" class="audit-list">
          <button
            v-for="(event, index) in group.events"
            :key="rowKey(event, index)"
            type="button"
            class="audit-row"
            :class="{ expanded: isRowExpanded(rowKey(event, index)) }"
            @click="toggleRow(rowKey(event, index))"
          >
            <div class="audit-primary">
              <span :class="['risk-badge', event.risk_level.toLowerCase()]">{{ event.risk_level }}</span>
              <span class="audit-action">{{ event.action }}</span>
              <span class="audit-target">{{ event.target }}</span>
            </div>
            <div class="audit-meta">
              <span class="audit-status">{{ event.status }}</span>
              <span class="audit-time">{{ event.timestamp }}</span>
            </div>

            <div v-if="isRowExpanded(rowKey(event, index))" class="audit-detail">
              <div class="audit-detail-row">
                <span class="audit-detail-label">Actor</span>
                <span class="audit-detail-value">{{ event.actor || "system" }}</span>
              </div>
              <div v-if="event.turn_id" class="audit-detail-row">
                <span class="audit-detail-label">Turn</span>
                <span class="audit-detail-value mono">{{ event.turn_id }}</span>
              </div>
              <div v-if="event.detail" class="audit-detail-row">
                <span class="audit-detail-label">Detail</span>
                <pre class="audit-detail-pre">{{ event.detail }}</pre>
              </div>
            </div>
          </button>
        </div>
      </section>
    </div>

    <div v-else class="empty-state">
      <p>暂无审计记录</p>
    </div>

    <ConfirmDialog
      :open="confirmClear"
      title="清空审计日志"
      message="确定清空全部审计日志吗？该操作会删除当前所有审计记录。"
      confirm-label="确认清空"
      :loading="clearing"
      @cancel="confirmClear = false"
      @confirm="clearAllAudits"
    />
  </div>
</template>

<style scoped>
.audit-view {
  padding: 28px 32px 32px;
  height: 100%;
  min-height: 0;
  max-width: 980px;
  display: flex;
  flex-direction: column;
}

.section-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.view-subtitle {
  max-width: 620px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.section-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-btn,
.group-export-btn {
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
  font-size: 12px;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.toolbar-btn:hover,
.group-export-btn:hover {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.12);
}

.toolbar-btn.danger {
  border-color: rgba(248, 113, 113, 0.18);
  background: rgba(248, 113, 113, 0.1);
  color: #fecaca;
}

.risk-select {
  min-width: 160px;
}

.export-banner {
  margin-bottom: 12px;
  padding: 10px 14px;
  border-radius: 14px;
  border: 1px solid rgba(var(--accent-rgb), 0.12);
  background: rgba(var(--accent-rgb), 0.05);
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.6;
}

.section-count,
.audit-group-count {
  flex-shrink: 0;
  min-width: 40px;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
  text-align: center;
}

.audit-groups {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  padding-right: 6px;
}

.audit-group {
  border: 1px solid var(--border-color);
  border-radius: 20px;
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  overflow: hidden;
}

.audit-group.expanded {
  border-color: rgba(var(--accent-rgb), 0.18);
}

.audit-group-head {
  width: 100%;
  padding: 16px 18px;
  border: none;
  background: transparent;
  color: inherit;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  cursor: pointer;
  text-align: left;
}

.audit-group-copy {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.audit-group-title {
  color: var(--text-primary);
  font-size: 15px;
  font-weight: 700;
}

.audit-group-note,
.audit-group-chevron {
  color: var(--text-tertiary);
  font-size: 12px;
}

.audit-group-meta {
  display: flex;
  align-items: center;
  gap: 10px;
}

.audit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 12px 12px;
}

.audit-row {
  width: 100%;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border-radius: 16px;
  background: rgba(var(--accent-rgb), 0.04);
  border: 1px solid rgba(var(--accent-rgb), 0.08);
  transition: background 0.15s ease, border-color 0.15s ease;
  cursor: pointer;
  text-align: left;
}

.audit-row:hover,
.audit-row.expanded {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.06);
}

.audit-primary {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
}

.audit-meta {
  display: flex;
  align-items: flex-end;
  flex-direction: column;
  gap: 6px;
  flex-shrink: 0;
}

.risk-badge {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  padding: 4px 8px;
  border-radius: 999px;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.risk-badge.readonly {
  background: color-mix(in srgb, #22c55e 15%, transparent);
  color: #22c55e;
}

.risk-badge.highrisk {
  background: color-mix(in srgb, #ef4444 15%, transparent);
  color: #ef4444;
}

.risk-badge.controlledwrite {
  background: color-mix(in srgb, #f59e0b 15%, transparent);
  color: #f59e0b;
}

.audit-action {
  color: var(--accent);
  font-weight: 600;
  min-width: 88px;
  font-size: 12px;
  font-family: var(--font-mono);
}

.audit-target {
  flex: 1;
  color: var(--text-primary);
  font-size: 13px;
  line-height: 1.55;
  word-break: break-word;
}

.audit-status {
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
  text-transform: lowercase;
}

.audit-time,
.mono {
  color: var(--text-tertiary);
  font-size: 11px;
  font-family: var(--font-mono);
}

.audit-detail {
  width: 100%;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid rgba(var(--accent-rgb), 0.1);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.audit-detail-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.audit-detail-label {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.audit-detail-value,
.audit-detail-pre {
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.6;
  word-break: break-word;
}

.audit-detail-pre {
  margin: 0;
  white-space: pre-wrap;
  font-family: var(--font-mono);
}

.empty-state {
  margin-top: 14px;
  padding: 34px;
  text-align: center;
  border: 1px dashed var(--border-color);
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.018);
  color: var(--text-tertiary);
  font-size: 14px;
}

@media (max-width: 760px) {
  .section-head,
  .section-controls,
  .audit-group-head,
  .audit-row,
  .audit-primary {
    flex-direction: column;
    align-items: flex-start;
  }

  .audit-meta {
    align-items: flex-start;
  }

  .risk-select {
    width: 100%;
  }
}
</style>
