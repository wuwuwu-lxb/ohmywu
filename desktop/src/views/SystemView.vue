<template>
  <div class="system-view">
    <header class="page-header">
      <div>
        <h1 class="page-title">系统管理</h1>
        <p class="page-subtitle">进程 · 服务 · 存储 · 日志</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="loadOverview">
          <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
            <path d="M11.5 6.5a4.5 4.5 0 11-1.35-3.24" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            <path d="M9.5 2.5l.35.5-.85.65-.65.1.35-.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
          刷新
        </button>
      </div>
    </header>

    <div v-if="overview" class="system-grid">
      <div
        v-for="(value, key, i) in formattedOverview"
        :key="key"
        class="sys-card"
        :style="{ animationDelay: `${i * 0.06}s` }"
      >
        <span class="sys-label">{{ key }}</span>
        <span class="sys-value">{{ value }}</span>
      </div>
    </div>

    <div v-else class="loading-state">
      <div class="loading-shimmer"></div>
      <span>正在获取系统信息...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import axios from 'axios'

const overview = ref<Record<string, string> | null>(null)

const formattedOverview = computed(() => {
  if (!overview.value) return {}
  const labels: Record<string, string> = {
    hostname: '主机名',
    os: '操作系统',
    kernel: '内核版本',
    uptime: '运行时长',
    cpu_model: 'CPU 型号',
    cpu_cores: 'CPU 核心数',
    memory_total: '内存总量',
    memory: '内存使用',
    disk: '磁盘使用',
    loadavg: '系统负载',
  }
  const result: Record<string, string> = {}
  for (const [k, v] of Object.entries(overview.value)) {
    if (v && v !== 'null' && v !== 'N/A') {
      result[labels[k] || k] = String(v)
    }
  }
  return result
})

async function loadOverview() {
  try {
    const res = await axios.get('http://127.0.0.1:3000/api/system/overview')
    overview.value = res.data
  } catch {
    overview.value = { error: '无法连接到 Daemon' }
  }
}

onMounted(loadOverview)
</script>

<style scoped>
.system-view {
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
  color: var(--text-primary);
}

.system-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
}

.sys-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  box-shadow: var(--shadow-sm);
  animation: fadeSlideIn 0.4s ease both;
  transition: box-shadow var(--transition-medium);
}

.sys-card:hover {
  box-shadow: var(--shadow-md);
}

.sys-label {
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--text-muted);
}

.sys-value {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-primary);
  word-break: break-all;
  font-family: var(--font-display);
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 80px;
  color: var(--text-muted);
  font-size: 14px;
}

.loading-shimmer {
  width: 200px;
  height: 12px;
  border-radius: 6px;
  background: linear-gradient(90deg, var(--bg-secondary) 25%, var(--border) 50%, var(--bg-secondary) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}
</style>
