<script setup lang="ts">
import { computed, provide, ref, KeepAlive } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import Toast from '@/components/Toast.vue'

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

const navItems = [
  { to: '/', label: '总览' },
  { to: '/system/processes', label: '系统能力中心' },
  { to: '/actions', label: 'Action' },
  { to: '/tasks', label: '任务总览' },
]

const pageMeta = computed(() => {
  if (route.path.startsWith('/system')) {
    return {
      eyebrow: 'system-management',
      title: '系统能力中心',
      description: '统一查看进程、服务、存储和日志能力，进入后通过顶部二级导航切换当前系统工作台。',
    }
  }

  if (route.path === '/actions') {
    return {
      eyebrow: 'stable actions',
      title: 'Action',
      description: '稳定能力资产入口。当前保留独立页面，后续逐步承接沉淀后的可直接触发能力。',
    }
  }

  if (route.path === '/tasks') {
    return {
      eyebrow: 'trace center',
      title: '任务总览',
      description: '统一查看 Task 与 Audit，追踪执行结果、风险等级与命令闭环。',
    }
  }

  return {
    eyebrow: 'computer manager',
    title: '总览',
    description: '电脑管家的主入口。先判断系统状态，再进入系统能力中心、Action 和任务追踪。',
  }
})
</script>

<template>
  <div class="shell shell-layout">
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
      </section>

      <RouterView v-slot="{ Component }">
        <KeepAlive>
          <component :is="Component" />
        </KeepAlive>
      </RouterView>
    </main>

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
