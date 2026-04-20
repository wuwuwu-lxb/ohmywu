<script setup lang="ts">
import { ref } from 'vue'

const tasks = ref([
  { id: '1', title: '系统概览扫描', status: 'completed', created_at: '2026-04-19T10:00:00Z' },
  { id: '2', title: '进程列表读取', status: 'completed', created_at: '2026-04-19T10:01:00Z' },
  { id: '3', title: '存储扫描 /tmp', status: 'running', created_at: '2026-04-19T10:02:00Z' },
])

function statusClass(s: string) {
  if (s === 'completed') return 'status-done'
  if (s === 'running') return 'status-run'
  return 'status-pending'
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">任务记录</p>
      <h3>任务列表</h3>
      <p class="muted">所有扫描与执行操作的任务记录。M3 完成后每个操作都会真实写入任务系统。</p>
    </article>
    <article class="panel full">
      <div class="task-list">
        <div v-for="t in tasks" :key="t.id" class="task-row">
          <span class="task-title">{{ t.title }}</span>
          <span class="task-ts">{{ t.created_at }}</span>
          <span class="task-status" :class="statusClass(t.status)">{{ t.status }}</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.task-list { font-size: 14px; }
.task-row { display: flex; gap: 12px; align-items: center; padding: 10px 0; border-bottom: 1px solid #f0ebe3; }
.task-title { flex: 1; font-weight: 500; }
.task-ts { font-size: 12px; color: #888; font-family: monospace; }
.task-status { font-size: 11px; padding: 2px 8px; border-radius: 4px; }
.status-done { background: #e8f5e9; color: #2e7d32; }
.status-run { background: #e3f2fd; color: #1565c0; }
.status-pending { background: #f5f5f5; color: #757575; }
</style>
