<script setup lang="ts">
import { onMounted, ref, watch, computed, inject } from 'vue'
import { fetchServices, controlService, type ServiceInfo, type ServiceAction } from '@/lib/api'
import LoadingOverlay from '@/components/LoadingOverlay.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import * as echarts from 'echarts'

const services = ref<ServiceInfo[]>([])
const error = ref<string | null>(null)
const search = ref('')
const loading = ref(true)
const pieRef = ref<HTMLDivElement | null>(null)
let pieChart: echarts.ECharts | null = null

const toast = inject<(message: string, type: 'success' | 'error' | 'info') => void>('toast')

interface ConfirmState {
  name: string
  action: ServiceAction
}

const confirmTarget = ref<ConfirmState | null>(null)

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

function getAvailableActions(service: ServiceInfo): ServiceAction[] {
  const actions: ServiceAction[] = []
  if (service.active_state !== 'active') actions.push('start')
  if (service.active_state === 'active') actions.push('stop')
  actions.push('restart')
  return actions
}

function getActionLabel(action: ServiceAction): string {
  const labels: Record<ServiceAction, string> = {
    start: '启动',
    stop: '停止',
    restart: '重启',
  }
  return labels[action]
}

function requestAction(name: string, action: ServiceAction) {
  confirmTarget.value = { name, action }
}

async function confirmAction() {
  if (!confirmTarget.value) return
  const { name, action } = confirmTarget.value
  confirmTarget.value = null
  try {
    await controlService(name, action)
    toast?.(`服务 ${name} 已${action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启'}`, 'success')
    await load()
  } catch (e) {
    toast?.(`操作失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  }
}

function cancelAction() {
  confirmTarget.value = null
}

function getConfirmTitle(action: ServiceAction): string {
  const titles: Record<ServiceAction, string> = {
    start: '确认启动服务',
    stop: '确认停止服务',
    restart: '确认重启服务',
  }
  return titles[action]
}

function getConfirmMessage(name: string, action: ServiceAction): string {
  const verbs: Record<ServiceAction, string> = {
    start: '启动',
    stop: '停止',
    restart: '重启',
  }
  return `确定要${verbs[action]}服务「${name}」吗？`
}
</script>

<template>
  <LoadingOverlay :visible="loading" message="加载服务列表…" />

  <ConfirmDialog
    v-if="confirmTarget"
    :title="getConfirmTitle(confirmTarget.action)"
    :message="getConfirmMessage(confirmTarget.name, confirmTarget.action)"
    :confirmText="getActionLabel(confirmTarget.action)"
    cancelText="取消"
    @confirm="confirmAction"
    @cancel="cancelAction"
  />

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
          <span>服务名</span><span>加载状态</span><span>活动状态</span><span>子状态</span><span>操作</span>
        </div>
        <template v-if="loading">
          <div v-for="i in 12" :key="`sk-${i}`" class="table-row skeleton-row">
            <span v-for="w in [200,70,70,70,140]" :key="w" class="skeleton" :style="`width:${w}px;height:14px`"></span>
          </div>
        </template>
        <div v-else v-for="s in filtered" :key="s.name" class="table-row">
          <span class="name">{{ s.name }}</span>
          <span class="badge" :class="`load-${s.load_state}`">{{ s.load_state }}</span>
          <span class="badge" :class="`active-${s.active_state}`">{{ s.active_state }}</span>
          <span class="sub">{{ s.sub_state }}</span>
          <span class="actions">
            <button
              v-for="action in getAvailableActions(s)"
              :key="action"
              class="btn-action"
              :class="`btn-${action}`"
              @click="requestAction(s.name, action)"
            >
              {{ getActionLabel(action) }}
            </button>
          </span>
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
.table-head, .table-row { display: grid; grid-template-columns: 1fr 100px 100px 100px 140px; gap: 8px; padding: 6px 8px; align-items: center; }
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
.actions { display: flex; gap: 6px; flex-wrap: wrap; }
.btn-action {
  padding: 4px 10px;
  font-size: 11px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}
.btn-start {
  background: #e8f5e9;
  border-color: #a5d6a7;
  color: #2e7d32;
}
.btn-start:hover {
  background: #c8e6c9;
}
.btn-stop {
  background: #ffebee;
  border-color: #ef9a9a;
  color: #c62828;
}
.btn-stop:hover {
  background: #ffcdd2;
}
.btn-restart {
  background: #e3f2fd;
  border-color: #90caf9;
  color: #1565c0;
}
.btn-restart:hover {
  background: #bbdefb;
}
</style>
