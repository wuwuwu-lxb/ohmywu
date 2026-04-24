<script setup lang="ts">
import { computed, provide, ref, KeepAlive, onMounted, onUnmounted } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import Toast from '@/components/Toast.vue'
import Live2DPet from '@/components/Live2DPet.vue'

const toasts = ref<Array<{ id: number; message: string; type: 'success' | 'error' | 'info' }>>([])
let toastId = 0

function showToast(message: string, type: 'success' | 'error' | 'info' = 'info') {
  const id = ++toastId
  toasts.value.push({ id, message, type })
  setTimeout(() => {
    toasts.value = toasts.value.filter((toast) => toast.id !== id)
  }, 3500)
}

provide('toast', showToast)

const route = useRoute()

// Auto-enter lightweight if URL has ?pet=1 (pet window mode)
// Pet window detection: only true when THIS window has ?pet=1 (pet window loads itself with this param)
const isPetWindow = window.location.search.includes('pet=1')
// Lightweight mode state: only meaningful for main window (controls shell visibility)
const isLightweightMode = ref(false)

function enterLightweight() {
  // Tell Electron main process to: hide main window + show pet window
  ;(window as any).ohmywu?.pet?.enter()
}

function exitLightweight() {
  // Tell Electron main process to: close pet window + show main window
  ;(window as any).ohmywu?.pet?.exit()
}

;(window as any).__ohmywu_enterLightweight = enterLightweight

let unsubEnter: (() => void) | null = null
let unsubExit: (() => void) | null = null

onMounted(() => {
  if (window.ohmywu) {
    unsubEnter = window.ohmywu.on('enter-lightweight', enterLightweight)
    unsubExit = window.ohmywu.on('exit-lightweight', () => {
      // Only relevant for main window
      isLightweightMode.value = false
    })
  }
})

onUnmounted(() => {
  unsubEnter?.()
  unsubExit?.()
})

const navItems = [
  { to: '/', label: '总览' },
  { to: '/system/processes', label: '系统能力中心' },
  { to: '/actions', label: 'Action' },
  { to: '/tasks', label: '任务总览' },
]

const pageMeta = computed(() => {
  if (route.path.startsWith('/system')) {
    return { eyebrow: 'system-management', title: '系统能力中心', description: '统一查看进程、服务、存储和日志能力，进入后通过顶部二级导航切换当前系统工作台。' }
  }
  if (route.path === '/actions') {
    return { eyebrow: 'stable actions', title: 'Action', description: '稳定能力资产入口。当前保留独立页面，后续逐步承接沉淀后的可直接触发能力。' }
  }
  if (route.path === '/tasks') {
    return { eyebrow: 'trace center', title: '任务总览', description: '统一查看 Task 与 Audit，追踪执行结果、风险等级与命令闭环。' }
  }
  return { eyebrow: 'computer manager', title: '总览', description: '电脑管家的主入口。先判断系统状态，再进入系统能力中心、Action 和任务追踪。' }
})
</script>

<template>
  <!-- Pet window: renders ONLY Live2DPet, no shell -->
  <Live2DPet v-if="isPetWindow" :is-lightweight-mode="true" @exit-lightweight="exitLightweight" />

  <!-- Main window: normal shell, hidden when in lightweight mode (pet window is showing) -->
  <div class="shell shell-layout" v-show="!isLightweightMode && !isPetWindow">
    <aside class="sidebar">
      <div class="brand">
        <p class="eyebrow">OhMyWu</p>
        <h1>真正的电脑管家</h1>
        <p class="muted shell-copy">本地优先、可控执行、可审计，同时为未来 Agent 模式保留统一能力底座。</p>
      </div>
      <nav class="nav sidebar-nav">
        <RouterLink v-for="item in navItems" :key="item.to" :to="item.to">{{ item.label }}</RouterLink>
      </nav>
      <div class="status-card">
        <span class="pill">v0.1 · system-management</span>
        <span class="pill pill-dark">sandbox by default</span>
      </div>
    </aside>

    <main class="content">
      <section class="topbar">
        <div class="page-title">
          <p class="eyebrow">{{ pageMeta.eyebrow }}</p>
          <h2>{{ pageMeta.title }}</h2>
          <p class="muted">{{ pageMeta.description }}</p>
        </div>
        <button class="lightweight-btn" @click="enterLightweight" title="轻量模式">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.2"/>
            <path d="M5 7 C5 5.5, 6 4.5, 7 4.5 C8 4.5, 9 5.5, 9 7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>
            <circle cx="7" cy="9" r="0.8" fill="currentColor"/>
          </svg>
          轻量模式
        </button>
      </section>

      <RouterView v-slot="{ Component }">
        <KeepAlive>
          <component :is="Component" />
        </KeepAlive>
      </RouterView>
    </main>

    <div class="toast-container">
      <Toast v-for="toast in toasts" :key="toast.id" :message="toast.message" :type="toast.type" />
    </div>
  </div>
</template>

<style scoped>
.lightweight-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  font-family: var(--font-body);
  transition: all var(--transition-fast);
  flex-shrink: 0;
}
.lightweight-btn:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-color: var(--accent);
}
.topbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}
</style>
