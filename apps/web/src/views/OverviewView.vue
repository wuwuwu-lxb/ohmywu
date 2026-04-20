<script setup lang="ts">
import { onActivated, onDeactivated, onUnmounted, ref, nextTick } from 'vue'
import { fetchActions, fetchHealth, fetchProcesses, fetchSystemOverview, type HealthResponse, type ProcessInfo } from '@/lib/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import * as echarts from 'echarts'

const health = ref<HealthResponse | null>(null)
const actionCount = ref<number>(0)
const systemOverview = ref<Record<string, string>>({})
const error = ref<string | null>(null)
const loading = ref(true)
const hasLoadedOnce = ref(false)

const processes = ref<ProcessInfo[]>([])
const cpuRef = ref<HTMLDivElement | null>(null)
const memRef = ref<HTMLDivElement | null>(null)
let cpuChart: echarts.ECharts | null = null
let memChart: echarts.ECharts | null = null
let timer: ReturnType<typeof setInterval> | null = null

function resizeCharts() {
  cpuChart?.resize()
  memChart?.resize()
}

function startPolling() {
  if (!timer) {
    timer = setInterval(() => {
      void refreshDashboard()
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
    void refreshDashboard()
    return
  }

  await refreshDashboard()
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
})

async function loadOverviewData() {
  try {
    const [healthResponse, actions, overview] = await Promise.all([
      fetchHealth(),
      fetchActions(),
      fetchSystemOverview(),
    ])
    health.value = healthResponse
    actionCount.value = actions.length
    systemOverview.value = overview
    error.value = null
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : '连接 daemon 失败'
  }
}

async function loadProcesses() {
  try {
    processes.value = await fetchProcesses()
  } catch {}
}

async function refreshDashboard() {
  await Promise.all([loadOverviewData(), loadProcesses()])
  loading.value = false
  hasLoadedOnce.value = true
  await nextTick()
  renderCharts()
  resizeCharts()
}

function renderCharts() {
  if (cpuRef.value && !cpuChart) cpuChart = echarts.init(cpuRef.value)
  if (memRef.value && !memChart) memChart = echarts.init(memRef.value)

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
}

const labelMap: Record<string, string> = {
  hostname: '主机名',
  os: '操作系统',
  kernel: '内核版本',
  uptime: '运行时长',
  cpu_model: 'CPU 型号',
  cpu_cores: 'CPU 核心数',
  loadavg: '系统负载',
  memory: '已用内存',
  memory_total: '总内存',
  memory_detail: '内存详情',
  disk: '磁盘',
}
</script>

<template>
  <LoadingOverlay :visible="loading" message="加载系统信息…" />

  <section class="page-grid">
    <article class="panel metric-card">
      <p class="eyebrow">运行时状态</p>
      <strong>{{ health?.status ?? '...' }}</strong>
      <p class="muted">
        {{ health ? `${health.app} v${health.version}` : '等待健康检查…' }}
      </p>
    </article>

    <article class="panel metric-card">
      <p class="eyebrow">共享 Actions</p>
      <strong>{{ actionCount }}</strong>
      <p class="muted">用户模式和未来 Agent 模式共用同一套 Action 表面。</p>
    </article>

    <article class="panel metric-card">
      <p class="eyebrow">能力域</p>
      <strong>system-management</strong>
      <p class="muted">首发只做 Linux 系统管理能力域。</p>
    </article>

    <div class="charts-row">
      <div class="chart-card" ref="cpuRef" style="height:220px;"></div>
      <div class="chart-card" ref="memRef" style="height:220px;"></div>
    </div>

    <article class="panel full">
      <p class="eyebrow">系统总览</p>
      <h3>当前 Linux 系统状态</h3>
      <div class="overview-grid">
        <template v-if="loading">
          <div v-for="i in 8" :key="`sk-${i}`" class="overview-item">
            <span class="skeleton" style="width:80px;height:11px;margin-bottom:4px;"></span>
            <span class="skeleton" style="width:160px;height:14px;"></span>
          </div>
        </template>
        <div v-else v-for="(value, key) in systemOverview" :key="key" class="overview-item">
          <span class="overview-key">{{ labelMap[key] ?? key }}</span>
          <span class="overview-value">{{ value }}</span>
        </div>
      </div>
      <p v-if="error" class="muted">{{ error }}</p>
    </article>
  </section>
</template>

<style scoped>
.charts-row { flex: 1 1 100%; width: 100%; min-width: 0; display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.chart-card { flex: 1 1 100%; width: 100%; min-width: 0; background: var(--panel-strong, #fffaf2); border: 1px solid rgba(74,55,39,0.14); border-radius: 16px; padding: 12px; }
.overview-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px; margin-top: 12px; }
.overview-item { display: flex; flex-direction: column; padding: 8px 12px; background: rgba(255,255,255,0.4); border-radius: 6px; }
.overview-key { font-size: 11px; color: #888; text-transform: uppercase; letter-spacing: 0.05em; }
.overview-value { font-size: 14px; font-weight: 500; color: #1a1612; margin-top: 2px; word-break: break-all; }
.skeleton { background: linear-gradient(90deg, #f0e7d9 25%, #e8dfd0 50%, #f0e7d9 75%); background-size: 200% 100%; animation: shimmer 1.4s infinite; border-radius: 4px; display: inline-block; }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
</style>
