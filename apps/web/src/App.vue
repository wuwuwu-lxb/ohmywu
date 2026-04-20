<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { RouterLink, RouterView } from 'vue-router'
import Toast from '@/components/Toast.vue'

const mode = ref<'user' | 'agent'>('user')
const toasts = ref<Array<{ id: number; message: string; type: 'success' | 'error' | 'info' }>>([])
let toastId = 0

function toggleMode() {
  if (mode.value === 'user') {
    mode.value = 'agent'
  } else {
    mode.value = 'user'
  }
}

function showToast(message: string, type: 'success' | 'error' | 'info' = 'info') {
  const id = ++toastId
  toasts.value.push({ id, message, type })
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }, 3500)
}

provide('toast', showToast)

const dispatchChartResize = () => {
  window.dispatchEvent(new CustomEvent('chart-resize'))
}

onMounted(() => {
  window.addEventListener('resize', dispatchChartResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', dispatchChartResize)
})
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <p class="eyebrow">OhMyWu</p>
        <h1>智能电脑管家</h1>
      </div>

      <!-- 模式切换 -->
      <div class="mode-switcher">
        <button
          class="mode-btn"
          :class="{ active: mode === 'user' }"
          @click="mode = 'user'"
        >
          用户模式
        </button>
        <button
          class="mode-btn"
          :class="{ active: mode === 'agent', disabled: true }"
          @click="mode = 'agent'"
          disabled
          title="Agent 模式暂未开放"
        >
          Agent 模式
        </button>
      </div>

      <nav class="nav" v-if="mode === 'user'">
        <RouterLink to="/">概览</RouterLink>
        <RouterLink to="/tools/processes">进程管理</RouterLink>
        <RouterLink to="/tools/services">服务管理</RouterLink>
        <RouterLink to="/tools/storage">存储扫描</RouterLink>
        <RouterLink to="/tools/logs">日志查看</RouterLink>
        <RouterLink to="/tasks">任务记录</RouterLink>
        <RouterLink to="/audits">审计记录</RouterLink>
      </nav>

      <nav class="nav" v-else>
        <RouterLink to="/conversation">对话</RouterLink>
        <RouterLink to="/actions">Actions</RouterLink>
      </nav>

      <div class="status-card">
        <span class="pill">v0.1 · system-management</span>
        <p>默认沙箱模式，写操作需确认。</p>
      </div>
    </aside>

    <main class="content">
      <RouterView v-slot="{ Component }">
        <KeepAlive>
          <component :is="Component" />
        </KeepAlive>
      </RouterView>
    </main>

    <!-- Toast container -->
    <div class="toast-container">
      <Toast
        v-for="toast in toasts"
        :key="toast.id"
        :message="toast.message"
        :type="toast.type"
      />
    </div>
  </div>
</template>

<style scoped>
.mode-switcher {
  display: flex;
  gap: 4px;
  padding: 8px 12px;
  background: rgba(255,255,255,0.05);
  border-radius: 8px;
  margin: 0 12px 12px;
}
.mode-btn {
  flex: 1;
  padding: 6px 8px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  background: transparent;
  color: #888;
  transition: all 0.2s;
}
.mode-btn.active {
  background: var(--accent, #c9a96e);
  color: #fff;
}
.mode-btn.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>

<style>
.toast-container {
  position: fixed;
  bottom: 24px;
  right: 24px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  z-index: 99999;
}
</style>
