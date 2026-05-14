<script setup lang="ts">
import { ref, onMounted } from "vue"
import { invoke } from "@tauri-apps/api/core"

interface AuditEvent {
  actor: string
  action: string
  target: string
  risk_level: string
  status: string
  detail?: string
  timestamp: string
}

const events = ref<AuditEvent[]>([])

onMounted(async () => {
  try {
    events.value = await invoke("get_audits")
  } catch {
    // Tauri not ready in dev
  }
})
</script>

<template>
  <div class="audit-view">
    <h2 class="view-title">审计日志</h2>
    <p class="view-subtitle">所有关键操作的追踪记录</p>

    <div class="audit-list">
      <div v-for="(e, i) in events" :key="i" class="audit-row">
        <span :class="['risk-badge', e.risk_level.toLowerCase()]">{{ e.risk_level }}</span>
        <span class="audit-action">{{ e.action }}</span>
        <span class="audit-target truncate">{{ e.target }}</span>
        <span class="audit-status">{{ e.status }}</span>
        <span class="audit-time">{{ e.timestamp }}</span>
      </div>
    </div>

    <div v-if="!events.length" class="empty-state">
      <p>暂无审计记录</p>
    </div>
  </div>
</template>

<style scoped>
.audit-view {
  padding: 24px 32px;
  max-width: 720px;
}

.view-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.view-subtitle {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-bottom: 20px;
}

.audit-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.audit-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-surface);
  font-size: 12px;
  font-family: var(--font-mono);
  transition: background 0.15s ease, border-color 0.15s ease;
}

.audit-row:hover {
  border-color: var(--border-hover);
  background: var(--surface-2);
}

.risk-badge {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  padding: 2px 6px;
  border-radius: 4px;
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
  min-width: 80px;
}

.audit-target {
  flex: 1;
  color: var(--text-secondary);
}

.audit-status {
  color: var(--text-tertiary);
}

.audit-time {
  color: var(--text-tertiary);
  font-size: 11px;
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 14px;
}
</style>
