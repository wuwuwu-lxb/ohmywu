<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { fetchTasks, type Task } from '@/lib/api'

const tasks = ref<Task[]>([])
const error = ref<string | null>(null)
let timer: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await load()
  timer = setInterval(load, 10000)
})

onUnmounted(() => { if (timer) clearInterval(timer) })

async function load() {
  try {
    tasks.value = await fetchTasks()
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
  }
}

function statusClass(s: string) {
  if (s === 'completed') return 'status-done'
  if (s === 'running') return 'status-run'
  if (s === 'failed') return 'status-fail'
  return 'status-pending'
}

function statusLabel(s: string) {
  const map: Record<string, string> = { completed: '完成', running: '运行中', failed: '失败', pending: '等待' }
  return map[s] ?? s
}

function formatTime(t: string | null) {
  if (!t) return '—'
  return new Date(t).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' })
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">任务记录</p>
      <h3>任务列表</h3>
      <p class="muted">所有扫描与执行操作的任务记录，每 10 秒自动刷新。</p>
    </article>
    <article class="panel full">
      <p v-if="error" class="muted">{{ error }}</p>
      <p v-if="tasks.length === 0 && !error" class="muted">暂无任务记录</p>
      <div class="task-list" v-if="tasks.length > 0">
        <div class="table-head">
          <span>操作</span><span>目标</span><span>创建时间</span><span>完成时间</span><span>状态</span>
        </div>
        <div v-for="t in tasks" :key="t.id" class="task-row">
          <span class="task-action">{{ t.action }}</span>
          <span class="task-target" :title="t.target">{{ t.target }}</span>
          <span class="task-ts">{{ formatTime(t.created_at) }}</span>
          <span class="task-ts">{{ formatTime(t.completed_at) }}</span>
          <span class="task-status" :class="statusClass(t.status)">{{ statusLabel(t.status) }}</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.task-list { font-size: 13px; }
.table-head { display: grid; grid-template-columns: 140px 1fr 140px 140px 70px; gap: 8px; padding: 6px 8px; font-size: 11px; text-transform: uppercase; color: #888; border-bottom: 1px solid #eee; font-weight: 600; }
.task-row { display: grid; grid-template-columns: 140px 1fr 140px 140px 70px; gap: 8px; padding: 8px; border-bottom: 1px solid #f0ebe3; align-items: center; }
.task-row:hover { background: rgba(201,169,110,0.08); }
.task-action { font-weight: 500; font-family: monospace; font-size: 12px; }
.task-target { font-size: 12px; color: #444; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.task-ts { font-size: 11px; color: #aaa; font-family: monospace; }
.task-status { font-size: 11px; padding: 2px 8px; border-radius: 4px; text-align: center; }
.status-done { background: #e8f5e9; color: #2e7d32; }
.status-run { background: #e3f2fd; color: #1565c0; }
.status-fail { background: #ffebee; color: #c62828; }
.status-pending { background: #f5f5f5; color: #757575; }
</style>
