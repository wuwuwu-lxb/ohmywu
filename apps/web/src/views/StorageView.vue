<script setup lang="ts">
import { ref, watch } from 'vue'
import { scanStorage, type StorageScanResult } from '@/lib/api'
import * as echarts from 'echarts'

const result = ref<StorageScanResult | null>(null)
const loading = ref(false)
const path = ref('/home')
const depth = ref(2)
const error = ref<string | null>(null)

const barRef = ref<HTMLDivElement | null>(null)
let barChart: echarts.ECharts | null = null

async function run() {
  loading.value = true
  error.value = null
  result.value = null
  barChart?.dispose()
  barChart = null
  try {
    result.value = await scanStorage({ path: path.value, depth: depth.value })
  } catch (e) {
    error.value = e instanceof Error ? e.message : '扫描失败'
  } finally {
    loading.value = false
  }
}

watch(result, (val) => {
  if (val) {
    setTimeout(renderChart, 50)
  }
}, { deep: true })

function renderChart() {
  if (!barRef.value || !result.value) return
  if (!barChart) barChart = echarts.init(barRef.value)

  const top = [...result.value.entries]
    .sort((a, b) => b.size_bytes - a.size_bytes)
    .slice(0, 10)

  const palette = ['#c85a2e', '#c9a96e', '#8b6b4a', '#b8956e', '#d4a574', '#a0785a', '#c4996e', '#9e7b5a', '#bf8f6e', '#dab892']

  barChart.setOption({
    title: { text: '存储占用 TOP 10', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { top: 36, left: 0, right: 16, bottom: 8, containLabel: true },
    xAxis: { type: 'value', axisLabel: { fontSize: 11 } },
    yAxis: { type: 'category', data: top.map(p => p.path.split('/').pop() || p.path), axisLabel: { fontSize: 10 }, inverse: true },
    series: [{
      type: 'bar',
      data: top.map((p, i) => ({ value: p.size_bytes, itemStyle: { color: palette[i % palette.length], borderRadius: [0, 4, 4, 0] } })),
      barMaxWidth: 20,
      label: { formatter: (p: any) => top[p.dataIndex].human_size, fontSize: 10, color: '#888', position: 'right' },
    }],
  })
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">存储扫描</p>
      <h3>目录占用分析</h3>
      <p class="muted">扫描指定目录，查看各子目录占用大小，找出占用空间最多的位置。</p>
    </article>

    <article class="panel full">
      <div class="scan-form">
        <label>
          路径
          <input v-model="path" type="text" placeholder="/home" class="form-input" />
        </label>
        <label>
          深度
          <input v-model.number="depth" type="number" min="1" max="5" class="form-input short" />
        </label>
        <button class="btn-primary" :disabled="loading" @click="run">
          {{ loading ? '扫描中…' : '开始扫描' }}
        </button>
      </div>
      <p v-if="error" class="muted">{{ error }}</p>
    </article>

    <div class="charts-row" v-if="result">
      <div class="chart-card" ref="barRef" style="height:260px;"></div>
    </div>

    <article class="panel full" v-if="result">
      <p class="eyebrow">扫描结果</p>
      <p class="summary">总占用：<strong>{{ result.entries.length }}</strong> 个条目，总计 <strong>{{ result.root_path }}</strong></p>
      <div class="storage-table">
        <div class="table-head">
          <span>路径</span>
          <span>大小</span>
          <span>字节</span>
        </div>
        <div v-for="e in result.entries" :key="e.path" class="table-row">
          <span class="path" :title="e.path">{{ e.path }}</span>
          <span class="size">{{ e.human_size }}</span>
          <span class="bytes">{{ e.size_bytes.toLocaleString() }}</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.scan-form { display: flex; gap: 12px; align-items: flex-end; flex-wrap: wrap; margin-bottom: 16px; }
label { display: flex; flex-direction: column; gap: 4px; font-size: 13px; color: #666; }
.form-input { padding: 8px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
.form-input.short { width: 70px; }
.btn-primary { padding: 8px 20px; background: var(--accent, #c9a96e); color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; }
.btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }
.summary { font-size: 14px; margin-bottom: 12px; }
.storage-table { font-size: 13px; }
.table-head, .table-row { display: grid; grid-template-columns: 1fr 100px 120px; gap: 8px; padding: 6px 8px; align-items: center; }
.table-head { font-size: 11px; text-transform: uppercase; color: #888; border-bottom: 1px solid #eee; font-weight: 600; }
.table-row { border-bottom: 1px solid #f5f0ea; }
.table-row:hover { background: rgba(201,169,110,0.08); }
.path { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 400px; }
.size { font-weight: 600; color: #1a1612; }
.bytes { font-family: monospace; color: #888; font-size: 12px; text-align: right; }
.charts-row { display: grid; grid-template-columns: 1fr; }
.chart-card { background: var(--panel-strong, #fffaf2); border: 1px solid rgba(74,55,39,0.14); border-radius: 16px; padding: 12px; }
</style>
