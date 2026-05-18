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
    <header class="section-head">
      <div>
        <h2 class="view-title">审计日志</h2>
        <p class="view-subtitle">所有关键读写和执行行为都会落到这里，便于回看系统做过什么。</p>
      </div>
      <span class="section-count">{{ events.length }}</span>
    </header>

    <div class="audit-list">
      <div v-for="(e, i) in events" :key="i" class="audit-row">
        <div class="audit-primary">
          <span :class="['risk-badge', e.risk_level.toLowerCase()]">{{ e.risk_level }}</span>
          <span class="audit-action">{{ e.action }}</span>
          <span class="audit-target truncate">{{ e.target }}</span>
        </div>
        <div class="audit-meta">
          <span class="audit-status">{{ e.status }}</span>
          <span class="audit-time">{{ e.timestamp }}</span>
        </div>
      </div>
    </div>

    <div v-if="!events.length" class="empty-state">
      <p>暂无审计记录</p>
    </div>
  </div>
</template>

<style scoped>
.audit-view {
  padding: 28px 32px 32px;
  max-width: 860px;
}

.section-head {
  display: flex;
  align-items: center;
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
  max-width: 560px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.section-count {
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

.audit-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.audit-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
  border-radius: 18px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-surface);
  transition: background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}

.audit-row:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.05);
  transform: translateY(-1px);
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
}

.audit-status {
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
  text-transform: lowercase;
}

.audit-time {
  color: var(--text-tertiary);
  font-size: 11px;
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
  .audit-row,
  .audit-primary {
    flex-direction: column;
    align-items: flex-start;
  }

  .audit-meta {
    align-items: flex-start;
  }
}
</style>
