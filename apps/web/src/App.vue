<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink, RouterView } from 'vue-router'

const mode = ref<'user' | 'agent'>('user')

function toggleMode() {
  if (mode.value === 'user') {
    mode.value = 'agent'
  } else {
    mode.value = 'user'
  }
}
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
      <RouterView />
    </main>
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
