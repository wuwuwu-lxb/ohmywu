<template>
  <div class="actions-page">
    <header class="page-header">
      <div>
        <h1 class="page-title">动作中心</h1>
        <p class="page-subtitle">所有可用 Action，统一调用入口</p>
      </div>
      <div class="filter-tabs">
        <button
          v-for="f in filters"
          :key="f"
          class="filter-tab"
          :class="{ active: activeFilter === f }"
          @click="activeFilter = f"
        >
          {{ f }}
        </button>
      </div>
    </header>

    <div class="action-grid">
      <div
        v-for="(action, i) in filteredActions"
        :key="action.name"
        class="action-panel-card"
        :style="{ animationDelay: `${i * 0.05}s` }"
      >
        <div class="apc-header">
          <div class="apc-icon" :class="action.risk_level">
            <svg v-if="action.risk_level === 'read_only'" width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.3"/>
              <path d="M5 8l2.5 2.5 3.5-3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.3"/>
              <path d="M8 5v3.5l2.5 2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            </svg>
          </div>
          <div class="apc-domain">{{ action.domain }}</div>
        </div>

        <div class="apc-body">
          <h3 class="apc-title">{{ action.title }}</h3>
          <p class="apc-summary">{{ action.summary }}</p>
        </div>

        <div class="apc-footer">
          <code class="apc-name">{{ action.name }}</code>
          <span class="apc-risk" :class="action.risk_level">{{ riskLabel(action.risk_level) }}</span>
        </div>

        <button class="apc-run" @click="runAction(action)">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M3 2l7 4-7 4V2z" fill="currentColor"/>
          </svg>
          执行
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import axios from 'axios'

const actions = ref<any[]>([])
const activeFilter = ref('全部')
const filters = ['全部', '只读', '可控', '危险']

const filteredActions = computed(() => {
  if (activeFilter.value === '全部') return actions.value
  const map: Record<string, string> = { '只读': 'read_only', '可控': 'controlled_write', '危险': 'high_risk' }
  return actions.value.filter(a => a.risk_level === map[activeFilter.value])
})

function riskLabel(level: string) {
  const map: Record<string, string> = { read_only: '只读', controlled_write: '可控', high_risk: '危险' }
  return map[level] || level
}

async function loadActions() {
  try {
    const res = await axios.get('http://127.0.0.1:3000/api/actions')
    actions.value = res.data
  } catch {}
}

async function runAction(action: any) {
  try {
    await axios.post(`http://127.0.0.1:3000/api/actions/${action.name}/run`)
  } catch {}
}

onMounted(loadActions)
</script>

<style scoped>
.actions-page {
  padding: 40px 48px;
  max-width: 1100px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 36px;
  animation: fadeSlideIn 0.5s ease both;
}

.page-title {
  font-size: 32px;
  font-weight: 600;
  letter-spacing: -0.02em;
  font-family: var(--font-display);
}

.page-subtitle {
  font-size: 13px;
  color: var(--text-muted);
  margin-top: 5px;
}

.filter-tabs {
  display: flex;
  gap: 4px;
  background: var(--bg-secondary);
  padding: 4px;
  border-radius: var(--radius-md);
}

.filter-tab {
  padding: 6px 14px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-family: var(--font-body);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.filter-tab.active {
  background: var(--bg-card);
  color: var(--text-primary);
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
}

.action-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.action-panel-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 22px;
  box-shadow: var(--shadow-sm);
  animation: fadeSlideIn 0.4s ease both;
  display: flex;
  flex-direction: column;
  gap: 14px;
  transition: box-shadow var(--transition-medium), transform var(--transition-medium);
  position: relative;
  overflow: hidden;
}

.action-panel-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, var(--accent), transparent);
  opacity: 0;
  transition: opacity var(--transition-medium);
}

.action-panel-card:hover {
  box-shadow: var(--shadow-md);
  transform: translateY(-2px);
}

.action-panel-card:hover::before {
  opacity: 1;
}

.apc-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.apc-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.apc-icon.read_only {
  background: rgba(126,200,160,0.12);
  color: #5AAD7E;
}

.apc-icon.controlled_write {
  background: rgba(201,169,110,0.12);
  color: var(--accent);
}

.apc-icon.high_risk {
  background: rgba(200,100,100,0.12);
  color: #C86060;
}

.apc-domain {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-secondary);
  padding: 2px 8px;
  border-radius: 4px;
}

.apc-body {
  flex: 1;
}

.apc-title {
  font-size: 16px;
  font-weight: 600;
  font-family: var(--font-display);
  color: var(--text-primary);
  margin-bottom: 6px;
}

.apc-summary {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
}

.apc-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.apc-name {
  font-size: 11px;
  font-family: monospace;
  color: var(--text-muted);
  background: var(--bg-secondary);
  padding: 2px 8px;
  border-radius: 4px;
}

.apc-risk {
  font-size: 11px;
  font-weight: 500;
  padding: 2px 8px;
  border-radius: 4px;
}

.apc-risk.read_only { background: rgba(126,200,160,0.12); color: #5AAD7E; }
.apc-risk.controlled_write { background: rgba(201,169,110,0.12); color: var(--accent); }
.apc-risk.high_risk { background: rgba(200,100,100,0.12); color: #C86060; }

.apc-run {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px;
  background: var(--bg-dark);
  color: var(--text-on-dark);
  border: none;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  font-family: var(--font-body);
  cursor: pointer;
  transition: all var(--transition-fast);
  width: 100%;
}

.apc-run:hover {
  background: var(--accent);
  color: #fff;
}
</style>
