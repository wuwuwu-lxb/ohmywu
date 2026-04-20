<script setup lang="ts">
import { onActivated, onDeactivated, onUnmounted, ref, computed, watch, inject, nextTick } from 'vue'
import { fetchProcesses, killProcess, type ProcessInfo } from '@/lib/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import * as echarts from 'echarts'

const processes = ref<ProcessInfo[]>([])
const error = ref<string | null>(null)
const search = ref('')
const loading = ref(true)
const hasLoadedOnce = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

const cpuRef = ref<HTMLDivElement | null>(null)
const memRef = ref<HTMLDivElement | null>(null)
const pieRef = ref<HTMLDivElement | null>(null)
let cpuChart: echarts.ECharts | null = null
let memChart: echarts.ECharts | null = null
let pieChart: echarts.ECharts | null = null

const toast = inject<(message: string, type: 'success' | 'error' | 'info') => void>('toast')

const confirmTarget = ref<{ pid: number; name: string } | null>(null)

function resizeCharts() {
  cpuChart?.resize()
  memChart?.resize()
  pieChart?.resize()
}

function startPolling() {
  if (!timer) {
    timer = setInterval(() => {
      void load(false)
    }, 5000)
  }
}

function stopPolling() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
}

onActivated(async () => {
  window.removeEventListener('chart-resize', resizeCharts)
  window.addEventListener('chart-resize', resizeCharts)
  startPolling()

  if (hasLoadedOnce.value) {
    await nextTick()
    renderCharts()
    resizeCharts()
    void load(false)
    return
  }

  await load(true)
})

onDeactivated(() => {
  stopPolling()
  window.removeEventListener('chart-resize', resizeCharts)
})

onUnmounted(() => {
  stopPolling()
  window.removeEventListener('chart-resize', resizeCharts)
  cpuChart?.dispose()
  memChart?.dispose()
  pieChart?.dispose()
})

async function load(showLoading = false) {
  if (showLoading && !hasLoadedOnce.value) {
    loading.value = true
  }

  try {
    processes.value = await fetchProcesses()
    error.value = null
    hasLoadedOnce.value = true
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
  } finally {
    loading.value = false
  }
}

watch(processes, async (val) => {
  if (!val.length || loading.value) return
  await nextTick()
  renderCharts()
  resizeCharts()
})

function renderCharts() {
  if (cpuRef.value && !cpuChart) cpuChart = echarts.init(cpuRef.value)
  if (memRef.value && !memChart) memChart = echarts.init(memRef.value)
  if (pieRef.value && !pieChart) pieChart = echarts.init(pieRef.value)

  const cpuTop = [...processes.value].sort((a, b) => b.cpu_percent - a.cpu_percent).slice(0, 10)
  const memTop = [...processes.value].sort((a, b) => b.memory_kb - a.memory_kb).slice(0, 10)

  if (cpuChart) cpuChart.setOption({
    title: { text: 'CPU 占用 TOP 10', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { top: 36, left: 16, right: 16, bottom: 56, containLabel: true },
    xAxis: { type: 'category', data: cpuTop.map(p => p.name.slice(0, 12)), axisLabel: { fontSize: 11, interval: 0, rotate: 20 } },
    yAxis: { type: 'value', axisLabel: { fontSize: 11 } },
    series: [{ type: 'bar', data: cpuTop.map(p => parseFloat(p.cpu_percent.toFixed(1))), itemStyle: { color: '#c85a2e', borderRadius: [4, 4, 0, 0] }, barMaxWidth: 28 }],
  })

  if (memChart) memChart.setOption({
    title: { text: '内存占用 TOP 10', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { top: 36, left: 16, right: 16, bottom: 56, containLabel: true },
    xAxis: { type: 'category', data: memTop.map(p => p.name.slice(0, 12)), axisLabel: { fontSize: 11, interval: 0, rotate: 20 } },
    yAxis: { type: 'value', axisLabel: { fontSize: 11 } },
    series: [{ type: 'bar', data: memTop.map(p => p.memory_kb), itemStyle: { color: '#c9a96e', borderRadius: [4, 4, 0, 0] }, barMaxWidth: 28 }],
  })

  if (pieChart) {
    const counts: Record<string, number> = {}
    processes.value.forEach(p => { const s = p.state || '?'; counts[s] = (counts[s] || 0) + 1 })
    const palette: Record<string, string> = { S: '#e8f5e9', R: '#e3f2fd', Z: '#ffebee', D: '#f3e5f5', T: '#fff8e1', I: '#e0f7fa' }
    pieChart.setOption({
      title: { text: '状态分布', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
      tooltip: { trigger: 'item' },
      series: [{
        type: 'pie', radius: ['40%', '68%'], center: ['50%', '58%'],
        data: Object.entries(counts).map(([state, count]) => ({ name: state, value: count, itemStyle: { color: palette[state] || '#eceff1' } })),
        label: { formatter: '{b}: {c}', fontSize: 11 },
      }],
    })
  }
}

const filtered = computed(() => {
  if (!search.value) return processes.value
  const q = search.value.toLowerCase()
  return processes.value.filter(
    (p) => p.name.toLowerCase().includes(q) || p.command.toLowerCase().includes(q) || String(p.pid).includes(q)
  )
})

function requestKill(pid: number, name: string) {
  confirmTarget.value = { pid, name }
}

async function confirmKill() {
  if (!confirmTarget.value) return
  const { pid, name } = confirmTarget.value
  confirmTarget.value = null
  try {
    await killProcess(pid)
    toast?.(`进程 ${name} (PID: ${pid}) 已结束`, 'success')
    await load(false)
  } catch (e) {
    toast?.(`结束进程失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  }
}

function cancelKill() {
  confirmTarget.value = null
}
</script>

<template>
  <LoadingOverlay :visible="loading" message="加载进程列表…" />

  <ConfirmDialog
    v-if="confirmTarget"
    title="确认结束进程"
    :message="`确定要结束进程「${confirmTarget.name}」(PID: ${confirmTarget.pid}) 吗？该操作不可撤销。`"
    confirmText="结束进程"
    cancelText="取消"
    :danger="true"
    @confirm="confirmKill"
    @cancel="cancelKill"
  />

  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">进程管理</p>
      <h3>进程列表</h3>
      <p class="muted">显示所有运行中的进程，点击"结束"可终止目标进程（需确认）。</p>
    </article>

    <div class="charts-row">
      <div class="chart-card" ref="cpuRef" style="height:220px;"></div>
      <div class="chart-card" ref="memRef" style="height:220px;"></div>
      <div class="chart-card" ref="pieRef" style="height:220px;"></div>
    </div>

    <article class="panel full">
      <div class="search-bar">
        <input v-model="search" type="text" placeholder="搜索进程名、命令或 PID…" class="search-input" />
        <span class="muted count">{{ filtered.length }} 个进程</span>
        <button class="btn-icon" @click="() => load(false)" title="刷新">↻</button>
      </div>

      <p v-if="error" class="muted">{{ error }}</p>

      <div class="process-table" v-if="!error">
        <div class="table-head">
          <span>PID</span><span>名称</span><span>状态</span><span>CPU %</span><span>内存 KB</span><span>用户</span><span>命令</span><span>操作</span>
        </div>
        <template v-if="loading">
          <div v-for="i in 12" :key="`sk-${i}`" class="table-row skeleton-row">
            <span v-for="w in [50,100,40,50,60,60,200,60]" :key="w" class="skeleton" :style="`width:${w}px;height:14px`"></span>
          </div>
        </template>
        <div v-else v-for="p in filtered" :key="p.pid" class="table-row">
          <span class="mono">{{ p.pid }}</span>
          <span class="name">{{ p.name }}</span>
          <span class="state" :class="`state-${p.state}`">{{ p.state }}</span>
          <span class="num">{{ p.cpu_percent.toFixed(1) }}</span>
          <span class="num">{{ p.memory_kb.toLocaleString() }}</span>
          <span>{{ p.user }}</span>
          <span class="command" :title="p.command">{{ p.command }}</span>
          <span class="action">
            <button class="btn-kill" @click="requestKill(p.pid, p.name)" title="结束进程">结束</button>
          </span>
        </div>
        <p v-if="!loading && filtered.length === 0" class="muted">无匹配进程</p>
      </div>
    </article>
  </section>
</template>

<style scoped>
.charts-row { flex: 1 1 100%; width: 100%; min-width: 0; display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.chart-card { flex: 1 1 100%; width: 100%; min-width: 0; background: var(--panel-strong, #fffaf2); border: 1px solid var(--line, rgba(74,55,39,0.14)); border-radius: 16px; padding: 12px; }
.search-bar { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
.search-input { flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
.btn-icon { width: 32px; height: 32px; border: 1px solid #ddd; border-radius: 8px; background: #fff; cursor: pointer; font-size: 14px; display: flex; align-items: center; justify-content: center; }
.btn-icon:hover { background: #f5f0ea; }
.count { font-size: 13px; white-space: nowrap; }
.process-table { font-size: 13px; max-height: calc(100vh - 520px); overflow-y: auto; border-radius: 8px; border: 1px solid #eee; }
.table-head, .table-row { display: grid; grid-template-columns: 70px 140px 60px 70px 90px 80px 1fr 60px; gap: 8px; padding: 6px 8px; align-items: center; }
.table-head { font-size: 11px; text-transform: uppercase; color: #888; border-bottom: 1px solid #eee; font-weight: 600; position: sticky; top: 0; background: #fffaf2; z-index: 1; }
.table-row { border-bottom: 1px solid #f5f0ea; }
.table-row:hover { background: rgba(201,169,110,0.08); }
.skeleton-row { pointer-events: none; }
.skeleton { background: linear-gradient(90deg, #f0e7d9 25%, #e8dfd0 50%, #f0e7d9 75%); background-size: 200% 100%; animation: shimmer 1.4s infinite; border-radius: 4px; display: inline-block; }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
.mono { font-family: monospace; }
.name { font-weight: 500; color: #1a1612; }
.state { font-size: 11px; padding: 2px 6px; border-radius: 4px; }
.state-S { background: #e8f5e9; color: #2e7d32; }
.state-R { background: #e3f2fd; color: #1565c0; }
.state-Z { background: #ffebee; color: #c62828; }
.num { font-family: monospace; text-align: right; }
.command { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 12px; color: #666; max-width: 300px; }
.action { display: flex; justify-content: center; }
.btn-kill {
  padding: 4px 10px;
  font-size: 12px;
  border-radius: 6px;
  border: 1px solid #ef9a9a;
  background: #ffebee;
  color: #c62828;
  cursor: pointer;
  transition: all 0.2s;
}
.btn-kill:hover {
  background: #ffcdd2;
  border-color: #e57373;
}
</style>
