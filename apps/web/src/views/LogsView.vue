<script setup lang="ts">
import { onActivated, onDeactivated, onUnmounted, ref, watch, nextTick } from 'vue'
import { fetchJournal, type JournalEntry } from '@/lib/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import * as echarts from 'echarts'

const entries = ref<JournalEntry[]>([])
const error = ref<string | null>(null)
const lines = ref(50)
const loading = ref(true)
const hasLoadedOnce = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

const pieRef = ref<HTMLDivElement | null>(null)
const barRef = ref<HTMLDivElement | null>(null)
let pieChart: echarts.ECharts | null = null
let barChart: echarts.ECharts | null = null

function resizeCharts() {
  pieChart?.resize()
  barChart?.resize()
}

function startPolling() {
  if (!timer) {
    timer = setInterval(() => {
      void load(false)
    }, 30000)
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
  pieChart?.dispose()
  barChart?.dispose()
})

async function load(showLoading = false) {
  if (showLoading && !hasLoadedOnce.value) {
    loading.value = true
  }

  try {
    entries.value = await fetchJournal({ lines: lines.value })
    error.value = null
    hasLoadedOnce.value = true
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
  } finally {
    loading.value = false
  }
}

watch(entries, async (val) => {
  if (!val.length || loading.value) return
  await nextTick()
  renderCharts()
  resizeCharts()
})

function renderCharts() {
  if (pieRef.value && !pieChart) pieChart = echarts.init(pieRef.value)
  if (barRef.value && !barChart) barChart = echarts.init(barRef.value)

  const priCounts: Record<number, number> = {}
  entries.value.forEach(e => { priCounts[e.priority] = (priCounts[e.priority] || 0) + 1 })
  const priPalette: Record<number, string> = { 0: '#c62828', 1: '#e53935', 2: '#f57f17', 3: '#d32f2f', 4: '#f9a825', 5: '#388e3c', 6: '#1976d2', 7: '#757575' }
  const priLabels: Record<number, string> = { 0: 'EMERG', 1: 'ALERT', 2: 'CRIT', 3: 'ERR', 4: 'WARN', 5: 'NOTICE', 6: 'INFO', 7: 'DEBUG' }

  if (pieChart) pieChart.setOption({
    title: { text: '优先级分布', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'item' },
    legend: { bottom: 0, left: 'center', textStyle: { fontSize: 10 } },
    series: [{
      type: 'pie', radius: ['40%', '68%'], center: ['50%', '50%'],
      data: Object.entries(priCounts).map(([p, c]) => ({ name: priLabels[Number(p)] || String(p), value: c, itemStyle: { color: priPalette[Number(p)] || '#eceff1' } })),
      label: { formatter: '{b}: {c}', fontSize: 10 },
      itemStyle: { borderRadius: 5 },
    }],
  })

  const unitCounts: Record<string, number> = {}
  entries.value.forEach(e => { unitCounts[e.unit] = (unitCounts[e.unit] || 0) + 1 })
  const topUnits = Object.entries(unitCounts).sort((a, b) => b[1] - a[1]).slice(0, 10)

  if (barChart) barChart.setOption({
    title: { text: '日志数 TOP 10 单位', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { top: 36, left: 16, right: 16, bottom: 56, containLabel: true },
    xAxis: { type: 'category', data: topUnits.map(([u]) => u.slice(0, 12)), axisLabel: { fontSize: 10, interval: 0, rotate: 20 } },
    yAxis: { type: 'value', axisLabel: { fontSize: 11 } },
    series: [{ type: 'bar', data: topUnits.map(([, c]) => c), itemStyle: { color: '#5c6bc0', borderRadius: [4, 4, 0, 0] }, barMaxWidth: 24 }],
  })
}

function priorityLabel(p: number) {
  const map: Record<number, string> = { 0: 'EMERG', 1: 'ALERT', 2: 'CRIT', 3: 'ERR', 4: 'WARN', 5: 'NOTICE', 6: 'INFO', 7: 'DEBUG' }
  return map[p] ?? String(p)
}
function priorityClass(p: number) {
  if (p <= 3) return 'priority-err'
  if (p <= 4) return 'priority-warn'
  return 'priority-info'
}
</script>

<template>
  <LoadingOverlay :visible="loading" message="加载日志…" />

  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">日志查看</p>
      <h3>systemd Journal</h3>
      <p class="muted">查看最近的 systemd 日志条目，帮助诊断系统异常。</p>
    </article>

    <div class="charts-row">
      <div class="chart-card" ref="pieRef" style="height:220px;"></div>
      <div class="chart-card" ref="barRef" style="height:220px;"></div>
    </div>

    <article class="panel full">
      <div class="controls">
        <label>
          显示行数
          <input v-model.number="lines" type="number" min="5" max="200" class="form-input short" @change="() => load(false)" />
        </label>
        <button class="btn-secondary" @click="() => load(false)">刷新</button>
      </div>
      <p v-if="error" class="muted">{{ error }}</p>

      <div class="log-list" v-if="!error">
        <div v-for="(e, i) in entries" :key="i" class="log-entry">
          <div class="log-meta">
            <span class="ts">{{ e.timestamp }}</span>
            <span class="unit">{{ e.unit }}</span>
            <span class="pri" :class="priorityClass(e.priority)">{{ priorityLabel(e.priority) }}</span>
          </div>
          <p class="msg">{{ e.message }}</p>
        </div>
        <p v-if="entries.length === 0" class="muted">无日志条目</p>
      </div>
    </article>
  </section>
</template>

<style scoped>
.charts-row { flex: 1 1 100%; width: 100%; min-width: 0; display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.chart-card { flex: 1 1 100%; width: 100%; min-width: 0; background: var(--panel-strong, #fffaf2); border: 1px solid rgba(74,55,39,0.14); border-radius: 16px; padding: 12px; }
.controls { display: flex; gap: 12px; align-items: flex-end; margin-bottom: 12px; }
label { display: flex; flex-direction: column; gap: 4px; font-size: 13px; color: #666; }
.form-input { padding: 8px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
.form-input.short { width: 70px; }
.btn-secondary { padding: 8px 16px; border: 1px solid #ddd; border-radius: 8px; cursor: pointer; font-size: 14px; background: #fff; }
.log-list { font-size: 13px; }
.log-entry { padding: 8px 0; border-bottom: 1px solid #f0ebe3; }
.log-meta { display: flex; gap: 10px; align-items: center; margin-bottom: 4px; }
.ts { font-family: monospace; font-size: 12px; color: #888; }
.unit { font-size: 12px; color: #666; font-weight: 500; }
.pri { font-size: 10px; padding: 1px 5px; border-radius: 3px; font-weight: 700; }
.priority-err { background: #ffebee; color: #c62828; }
.priority-warn { background: #fff8e1; color: #f57f17; }
.priority-info { background: #e8f5e9; color: #2e7d32; }
.msg { color: #1a1612; line-height: 1.5; word-break: break-all; }
</style>
