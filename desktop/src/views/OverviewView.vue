<template>
  <div class="overview">
    <!-- Page header -->
    <header class="page-header">
      <div class="header-text">
        <h1 class="page-title">系统总览</h1>
        <p class="page-subtitle">{{ greeting }} · {{ currentTime }}</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="refresh">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M12.5 7a5.5 5.5 0 11-1.65-3.96" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            <path d="M10.5 3l.4.6-1 .7-.7.1.4-.6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
          刷新
        </button>
      </div>
    </header>

    <!-- Metric cards -->
    <section class="metrics">
      <div
        v-for="(metric, i) in metrics"
        :key="metric.label"
        class="metric-card"
        :style="{ animationDelay: `${i * 0.08}s` }"
      >
        <div class="metric-header">
          <span class="metric-icon" v-html="metric.icon"></span>
          <span class="metric-label">{{ metric.label }}</span>
        </div>
        <div class="metric-value-row">
          <span class="metric-value">{{ metric.value }}</span>
          <span v-if="metric.unit" class="metric-unit">{{ metric.unit }}</span>
        </div>
        <div class="metric-bar">
          <div class="metric-bar-fill" :style="{ width: metric.percent + '%', background: metric.color }"></div>
        </div>
        <div class="metric-detail">{{ metric.detail }}</div>
      </div>
    </section>

    <!-- Two column layout -->
    <div class="content-grid">
      <!-- Actions -->
      <section class="panel">
        <div class="panel-header">
          <h2 class="panel-title">快速动作</h2>
          <span class="panel-count">{{ actions.length }} 项</span>
        </div>
        <div class="action-list">
          <button
            v-for="(action, i) in actions"
            :key="action.name"
            class="action-card"
            :style="{ animationDelay: `${0.3 + i * 0.06}s` }"
            @click="runAction(action)"
          >
            <div class="action-card-inner">
              <div class="action-icon" :class="action.risk_level">
                <svg v-if="action.risk_level === 'read_only'" width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M4.5 7l2 2 3-3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                <svg v-else width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
              </div>
              <div class="action-info">
                <span class="action-name">{{ action.name }}</span>
                <span class="action-title">{{ action.title }}</span>
              </div>
              <span class="action-risk" :class="action.risk_level">{{ riskLabel(action.risk_level) }}</span>
            </div>
            <div class="action-card-arrow">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M4.5 2.5l3 3.5-3 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
          </button>
        </div>
      </section>

      <!-- System status -->
      <section class="panel">
        <div class="panel-header">
          <h2 class="panel-title">系统状态</h2>
          <span class="status-badge running">
            <span class="status-dot"></span>
            正常
          </span>
        </div>
        <div class="status-list">
          <div v-for="item in systemStatus" :key="item.label" class="status-row">
            <span class="status-row-label">{{ item.label }}</span>
            <span class="status-row-value" :class="item.state">{{ item.value }}</span>
          </div>
        </div>
        <div class="panel-footer">
          <span class="uptime">运行时长：{{ uptime }}</span>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import axios from 'axios'

const actions = ref<any[]>([])
const currentTime = ref('')
const uptime = ref('--')

const greeting = computed(() => {
  const h = new Date().getHours()
  if (h < 6) return '夜深了'
  if (h < 9) return '早上好'
  if (h < 12) return '上午好'
  if (h < 14) return '中午好'
  if (h < 18) return '下午好'
  if (h < 22) return '晚上好'
  return '夜深了'
})

const metrics = ref([
  {
    label: 'CPU',
    value: '--',
    unit: '%',
    percent: 0,
    detail: '计算负载',
    color: '#C9A96E',
    icon: `<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><rect x="2" y="2" width="10" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/><rect x="4.5" y="4.5" width="5" height="5" rx="1" fill="currentColor" opacity="0.4"/><path d="M5 1v2M9 1v2M5 11v2M9 11v2M1 5h2M1 9h2M11 5h2M11 9h2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>`,
  },
  {
    label: '内存',
    value: '--',
    unit: 'GB',
    percent: 0,
    detail: '已用 / 总计',
    color: '#7EACC4',
    icon: `<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><rect x="1" y="3" width="12" height="8" rx="1.5" stroke="currentColor" stroke-width="1.2"/><path d="M4 3V1.5M7 3V1.5M10 3V1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>`,
  },
  {
    label: '磁盘',
    value: '--',
    unit: 'GB',
    percent: 0,
    detail: '已用 / 总计',
    color: '#A47EBC',
    icon: `<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><ellipse cx="7" cy="7" rx="5.5" ry="5.5" stroke="currentColor" stroke-width="1.2"/><path d="M7 1.5V7l4 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>`,
  },
  {
    label: '负载',
    value: '--',
    unit: '',
    percent: 0,
    detail: '1 / 5 / 15 分钟',
    color: '#7EC8A0',
    icon: `<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 10l3-3 3 3 4-5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
  },
])

const systemStatus = ref([
  { label: 'Daemon', value: '已连接', state: 'good' },
  { label: 'Web 服务', value: '运行中', state: 'good' },
  { label: 'Action 引擎', value: '就绪', state: 'good' },
  { label: '审计系统', value: '记录中', state: 'good' },
])

async function loadActions() {
  try {
    const res = await axios.get('http://127.0.0.1:3000/api/actions')
    actions.value = res.data
  } catch {
    actions.value = []
  }
}

async function loadSystemOverview() {
  try {
    const res = await axios.get('http://127.0.0.1:3000/api/system/overview')
    const d = res.data
    if (d.cpu) {
      metrics.value[0].value = d.cpu
      metrics.value[0].percent = parseFloat(d.cpu)
    }
    if (d.memory) {
      metrics.value[1].value = d.memory
      const match = d.memory_detail?.match(/(\d+)\/(\d+)/)
      if (match) {
        metrics.value[1].percent = Math.round((parseInt(match[1]) / parseInt(match[2])) * 100)
        metrics.value[1].detail = `${match[1]} / ${match[2]} GB`
      }
    }
    if (d.disk) {
      metrics.value[2].value = d.disk
    }
    if (d.loadavg) {
      metrics.value[3].value = d.loadavg
    }
    if (d.uptime) {
      uptime.value = d.uptime
    }
  } catch {
    // fallback to mock
  }
}

async function runAction(action: any) {
  try {
    await axios.post(`http://127.0.0.1:3000/api/actions/${action.name}/run`)
  } catch {}
}

function refresh() {
  loadSystemOverview()
}

function riskLabel(level: string) {
  const map: Record<string, string> = {
    read_only: '只读',
    controlled_write: '可控',
    high_risk: '危险',
  }
  return map[level] || level
}

let timer: ReturnType<typeof setInterval>

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  loadActions()
  loadSystemOverview()
})

onUnmounted(() => clearInterval(timer))
</script>

<style scoped>
.overview {
  padding: 40px 48px;
  max-width: 1100px;
}

/* Header */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 40px;
  animation: fadeSlideIn 0.5s ease both;
}

.page-title {
  font-size: 32px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: -0.02em;
  line-height: 1.1;
}

.page-subtitle {
  font-size: 13px;
  color: var(--text-muted);
  margin-top: 6px;
  font-weight: 400;
}

.btn-ghost {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  font-family: var(--font-body);
  transition: all var(--transition-fast);
}

.btn-ghost:hover {
  background: var(--bg-secondary);
  border-color: var(--border-strong);
  color: var(--text-primary);
}

/* Metrics */
.metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 32px;
}

.metric-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 20px;
  box-shadow: var(--shadow-sm);
  animation: fadeSlideIn 0.5s ease both;
  transition: box-shadow var(--transition-medium), transform var(--transition-medium);
}

.metric-card:hover {
  box-shadow: var(--shadow-md);
  transform: translateY(-1px);
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 12px;
}

.metric-icon {
  display: flex;
  color: var(--text-muted);
}

.metric-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.metric-value-row {
  display: flex;
  align-items: baseline;
  gap: 3px;
  margin-bottom: 10px;
}

.metric-value {
  font-family: var(--font-display);
  font-size: 30px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1;
  letter-spacing: -0.02em;
}

.metric-unit {
  font-size: 13px;
  color: var(--text-muted);
}

.metric-bar {
  height: 3px;
  background: var(--bg-secondary);
  border-radius: 2px;
  margin-bottom: 8px;
  overflow: hidden;
}

.metric-bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 1s ease;
}

.metric-detail {
  font-size: 11px;
  color: var(--text-muted);
}

/* Content grid */
.content-grid {
  display: grid;
  grid-template-columns: 1fr 340px;
  gap: 20px;
}

/* Panel */
.panel {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  box-shadow: var(--shadow-sm);
  animation: fadeSlideIn 0.5s ease both;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.panel-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  font-family: var(--font-display);
}

.panel-count {
  font-size: 12px;
  color: var(--text-muted);
}

/* Actions */
.action-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.action-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  animation: fadeSlideIn 0.4s ease both;
  font-family: var(--font-body);
  text-align: left;
  width: 100%;
}

.action-card:hover {
  background: var(--bg-secondary);
  border-color: var(--accent);
  transform: translateX(2px);
}

.action-card-inner {
  display: flex;
  align-items: center;
  gap: 12px;
}

.action-icon {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.action-icon.read_only {
  background: rgba(126,200,160,0.12);
  color: #5AAD7E;
}

.action-icon.controlled_write {
  background: rgba(201,169,110,0.12);
  color: var(--accent);
}

.action-icon.high_risk {
  background: rgba(200,100,100,0.12);
  color: #C86060;
}

.action-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.action-name {
  font-size: 11px;
  color: var(--text-muted);
  font-family: monospace;
}

.action-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.action-risk {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: 500;
}

.action-risk.read_only {
  background: rgba(126,200,160,0.12);
  color: #5AAD7E;
}

.action-risk.controlled_write {
  background: rgba(201,169,110,0.12);
  color: var(--accent);
}

.action-risk.high_risk {
  background: rgba(200,100,100,0.12);
  color: #C86060;
}

.action-card-arrow {
  color: var(--text-muted);
  opacity: 0;
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}

.action-card:hover .action-card-arrow {
  opacity: 1;
  transform: translateX(3px);
}

/* Status */
.status-badge {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-weight: 500;
  padding: 3px 10px;
  border-radius: 20px;
}

.status-badge.running {
  background: rgba(126,200,160,0.12);
  color: #5AAD7E;
}

.status-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #7EC8A0;
  animation: pulse 3s ease-in-out infinite;
}

.status-list {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.status-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}

.status-row:last-child {
  border-bottom: none;
}

.status-row-label {
  font-size: 13px;
  color: var(--text-secondary);
}

.status-row-value {
  font-size: 13px;
  font-weight: 500;
}

.status-row-value.good { color: #5AAD7E; }
.status-row-value.warn { color: var(--accent); }
.status-row-value.error { color: #C86060; }

.panel-footer {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.uptime {
  font-size: 12px;
  color: var(--text-muted);
  font-family: monospace;
}
</style>
