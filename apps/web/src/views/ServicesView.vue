<script setup lang="ts">
import { onMounted, ref, watch, computed } from 'vue'
import { fetchServices, type ServiceInfo } from '@/lib/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import * as echarts from 'echarts'

const services = ref<ServiceInfo[]>([])
const error = ref<string | null>(null)
const search = ref('')
const loading = ref(true)
const pieRef = ref<HTMLDivElement | null>(null)
let pieChart: echarts.ECharts | null = null

onMounted(async () => {
  await load()
})

async function load() {
  try {
    services.value = await fetchServices()
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
  } finally {
    loading.value = false
  }
}

watch(services, (val) => {
  if (val.length && !loading.value) renderPie()
})

function renderPie() {
  if (!pieRef.value) return
  if (!pieChart) pieChart = echarts.init(pieRef.value)

  const activeCount = services.value.filter(s => s.active_state === 'active').length
  const inactiveCount = services.value.filter(s => s.active_state === 'inactive').length
  const failedCount = services.value.filter(s => s.active_state === 'failed').length
  const otherCount = services.value.length - activeCount - inactiveCount - failedCount

  pieChart.setOption({
    title: { text: '服务状态分布', textStyle: { fontSize: 13, fontWeight: '600', color: '#1a1612' }, left: 0, top: 4 },
    tooltip: { trigger: 'item' },
    legend: { bottom: 0, left: 'center', textStyle: { fontSize: 11 } },
    series: [{
      type: 'pie',
      radius: ['40%', '68%'],
      center: ['50%', '50%'],
      data: [
        { name: 'active', value: activeCount, itemStyle: { color: '#e8f5e9' } },
        { name: 'inactive', value: inactiveCount, itemStyle: { color: '#f5f5f5' } },
        { name: 'failed', value: failedCount, itemStyle: { color: '#ffebee' } },
        { name: 'other', value: otherCount, itemStyle: { color: '#eceff1' } },
      ],
      label: { formatter: '{b}: {c}', fontSize: 11 },
      itemStyle: { borderRadius: 6 },
    }],
  })
}

const filtered = computed(() => {
  if (!search.value) return services.value
  const q = search.value.toLowerCase()
  return services.value.filter((s) => s.name.toLowerCase().includes(q))
})
</script>

<template>
  <LoadingOverlay :visible="loading" message="加载服务列表…" />

  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">服务管理</p>
      <h3>systemd 服务列表</h3>
      <p class="muted">查看所有服务状态，进行启动、停止、重启操作（需确认）。</p>
    </article>

    <div class="charts-row">
      <div class="chart-card" ref="pieRef" style="height:240px;"></div>
    </div>

    <article class="panel full">
      <div class="search-bar">
        <input v-model="search" type="text" placeholder="搜索服务名…" class="search-input" />
        <span class="muted count">{{ filtered.length }} 个服务</span>
      </div>

      <p v-if="error" class="muted">{{ error }}</p>

      <div class="service-table" v-if="!error">
        <div class="table-head">
          <span>服务名</span><span>加载状态</span><span>活动状态</span><span>子状态</span>
        </div>
        <template v-if="loading">
          <div v-for="i in 12" :key="`sk-${i}`" class="table-row skeleton-row">
            <span v-for="w in [200,70,70,70]" :key="w" class="skeleton" :style="`width:${w}px;height:14px`"></span>
          </div>
        </template>
        <div v-else v-for="s in filtered" :key="s.name" class="table-row">
          <span class="name">{{ s.name }}</span>
          <span class="badge" :class="`load-${s.load_state}`">{{ s.load_state }}</span>
          <span class="badge" :class="`active-${s.active_state}`">{{ s.active_state }}</span>
          <span class="sub">{{ s.sub_state }}</span>
        </div>
        <p v-if="!loading && filtered.length === 0" class="muted">无匹配服务</p>
      </div>
    </article>
  </section>
</template>

<style scoped>
.charts-row { display: grid; grid-template-columns: 1fr; }
.chart-card { background: var(--panel-strong, #fffaf2); border: 1px solid var(--line, rgba(74,55,39,0.14)); border-radius: 16px; padding: 12px; }
.search-bar { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
.search-input { flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
.count { font-size: 13px; white-space: nowrap; }
.service-table { font-size: 13px; max-height: calc(100vh - 540px); overflow-y: auto; border-radius: 8px; border: 1px solid #eee; }
.table-head, .table-row { display: grid; grid-template-columns: 1fr 100px 100px 100px; gap: 8px; padding: 6px 8px; align-items: center; }
.table-head { font-size: 11px; text-transform: uppercase; color: #888; border-bottom: 1px solid #eee; font-weight: 600; position: sticky; top: 0; background: #fffaf2; z-index: 1; }
.table-row { border-bottom: 1px solid #f5f0ea; }
.table-row:hover { background: rgba(201,169,110,0.08); }
.skeleton-row { pointer-events: none; }
.skeleton { background: linear-gradient(90deg, #f0e7d9 25%, #e8dfd0 50%, #f0e7d9 75%); background-size: 200% 100%; animation: shimmer 1.4s infinite; border-radius: 4px; display: inline-block; }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
.name { font-weight: 500; font-size: 13px; }
.badge { font-size: 11px; padding: 2px 6px; border-radius: 4px; text-align: center; }
.load-loaded { background: #e8f5e9; color: #2e7d32; }
.load-not-found { background: #ffebee; color: #c62828; }
.active-active { background: #e8f5e9; color: #2e7d32; }
.active-inactive { background: #f5f5f5; color: #757575; }
.active-failed { background: #ffebee; color: #c62828; }
.sub { color: #666; font-size: 12px; }
</style>
