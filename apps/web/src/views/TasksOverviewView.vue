<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchAudits, fetchTasks, type AuditEvent, type Task } from '@/lib/api'

const tasks = ref<Task[]>([])
const audits = ref<AuditEvent[]>([])
const error = ref<string | null>(null)
const loading = ref(true)

onMounted(async () => {
  try {
    const [taskData, auditData] = await Promise.all([
      fetchTasks(),
      fetchAudits(100),
    ])
    tasks.value = taskData
    audits.value = auditData
    error.value = null
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : '加载任务总览失败'
  } finally {
    loading.value = false
  }
})

function formatTime(value: string | null) {
  if (!value) return '—'
  return new Date(value).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function statusLabel(status: Task['status']) {
  const map: Record<Task['status'], string> = {
    pending: '等待',
    running: '运行中',
    completed: '完成',
    failed: '失败',
  }
  return map[status]
}

function riskLabel(risk: AuditEvent['risk_level']) {
  const map: Record<AuditEvent['risk_level'], string> = {
    read_only: '只读',
    controlled_write: '受控写',
    high_risk: '高风险',
  }
  return map[risk]
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">trace overview</p>
      <h3>任务总览</h3>
      <p class="muted">
        这里统一收口 Task 与 Audit。重点不是把两张表并排堆上去，而是让用户能追踪执行动作、风险等级和最终结果，保持命令闭环可见。
      </p>
    </article>

    <article class="panel metric-card">
      <p class="eyebrow">任务数</p>
      <strong>{{ tasks.length }}</strong>
      <p class="muted">所有扫描与执行动作统一进入任务系统。</p>
    </article>

    <article class="panel metric-card">
      <p class="eyebrow">审计数</p>
      <strong>{{ audits.length }}</strong>
      <p class="muted">所有关键读取与写操作都保留审计记录。</p>
    </article>

    <article class="panel metric-card">
      <p class="eyebrow">失败任务</p>
      <strong>{{ tasks.filter((task) => task.status === 'failed').length }}</strong>
      <p class="muted">优先关注失败动作和高风险执行结果。</p>
    </article>

    <article class="panel full">
      <p v-if="loading" class="muted">加载任务总览中…</p>
      <p v-else-if="error" class="muted">{{ error }}</p>
      <template v-else>
        <div class="trace-section">
          <div class="trace-head">
            <div>
              <p class="eyebrow">task stream</p>
              <h3>任务流</h3>
            </div>
          </div>
          <div v-if="tasks.length === 0" class="muted">暂无任务记录</div>
          <div v-else class="trace-table">
            <div class="table-head task-grid">
              <span>操作</span><span>目标</span><span>状态</span><span>创建时间</span><span>完成时间</span>
            </div>
            <div v-for="task in tasks" :key="task.id" class="table-row task-grid">
              <span class="mono">{{ task.action }}</span>
              <span class="truncate" :title="task.target">{{ task.target }}</span>
              <span class="badge" :class="`status-${task.status}`">{{ statusLabel(task.status) }}</span>
              <span class="mono muted">{{ formatTime(task.created_at) }}</span>
              <span class="mono muted">{{ formatTime(task.completed_at) }}</span>
            </div>
          </div>
        </div>

        <div class="trace-section">
          <div class="trace-head">
            <div>
              <p class="eyebrow">audit stream</p>
              <h3>审计流</h3>
            </div>
          </div>
          <div v-if="audits.length === 0" class="muted">暂无审计记录</div>
          <div v-else class="trace-table">
            <div class="table-head audit-grid">
              <span>时间</span><span>动作</span><span>目标</span><span>风险</span><span>结果</span>
            </div>
            <div v-for="audit in audits" :key="audit.id" class="table-row audit-grid">
              <span class="mono muted">{{ formatTime(audit.timestamp) }}</span>
              <span class="mono">{{ audit.action }}</span>
              <span class="truncate" :title="audit.target">{{ audit.target }}</span>
              <span class="badge" :class="`risk-${audit.risk_level}`">{{ riskLabel(audit.risk_level) }}</span>
              <span class="badge" :class="audit.result === 'success' ? 'result-success' : 'result-failure'">{{ audit.result }}</span>
            </div>
          </div>
        </div>
      </template>
    </article>
  </section>
</template>

<style scoped>
.trace-section {
  display: grid;
  gap: 12px;
}

.trace-section + .trace-section {
  margin-top: 24px;
}

.trace-table {
  overflow: hidden;
  border-radius: 14px;
  border: 1px solid #eee;
}

.table-head,
.table-row {
  display: grid;
  gap: 12px;
  padding: 10px 12px;
  align-items: center;
}

.table-head {
  font-size: 11px;
  text-transform: uppercase;
  color: #888;
  background: #fffaf2;
  border-bottom: 1px solid #eee;
  font-weight: 700;
}

.table-row {
  border-bottom: 1px solid #f2ece4;
}

.task-grid {
  grid-template-columns: 180px 1fr 100px 160px 160px;
}

.audit-grid {
  grid-template-columns: 160px 180px 1fr 100px 100px;
}

.mono {
  font-family: monospace;
}

.truncate {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 4px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 700;
}

.status-pending { background: #f5f5f5; color: #757575; }
.status-running { background: #e3f2fd; color: #1565c0; }
.status-completed { background: #e8f5e9; color: #2e7d32; }
.status-failed { background: #ffebee; color: #c62828; }
.risk-read_only { background: #e8f5e9; color: #2e7d32; }
.risk-controlled_write { background: #fff8e1; color: #f57f17; }
.risk-high_risk { background: #ffebee; color: #c62828; }
.result-success { background: #e8f5e9; color: #2e7d32; }
.result-failure { background: #ffebee; color: #c62828; }

@media (max-width: 960px) {
  .task-grid,
  .audit-grid {
    grid-template-columns: 1fr;
  }
}
</style>
