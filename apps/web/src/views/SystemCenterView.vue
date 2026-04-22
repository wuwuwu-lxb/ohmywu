<script setup lang="ts">
import { KeepAlive } from 'vue'
import { RouterLink, RouterView } from 'vue-router'

const systemItems = [
  { to: '/system/processes', label: '进程' },
  { to: '/system/services', label: '服务' },
  { to: '/system/storage', label: '存储' },
  { to: '/system/logs', label: '日志' },
]

const summaryCards = [
  {
    title: '进程',
    summary: '查看实时进程列表、CPU/内存占用分布，并执行受控结束进程。',
    to: '/system/processes',
  },
  {
    title: '服务',
    summary: '统一查看 systemd 服务状态，并执行启动、停止、重启。',
    to: '/system/services',
  },
  {
    title: '存储',
    summary: '分析目录占用与清理候选，先预览再执行。',
    to: '/system/storage',
  },
  {
    title: '日志',
    summary: '查看 journal 日志、异常优先级分布和关键单位。',
    to: '/system/logs',
  },
]
</script>

<template>
  <section class="page-grid">
    <article class="panel full system-summary-panel">
      <div class="system-summary-head">
        <div>
          <p class="eyebrow">system capability center</p>
          <h3>系统能力中心</h3>
          <p class="muted">
            当前把 system-management 的固定工作台统一收口到这里。主导航保持精简，二级能力导航直接写死，避免用户心智被噪音打散。
          </p>
        </div>
        <span class="pill pill-warm">固定二级导航</span>
      </div>

      <div class="system-card-grid">
        <RouterLink v-for="card in summaryCards" :key="card.to" :to="card.to" class="system-card">
          <div>
            <p class="eyebrow">workspace</p>
            <h4>{{ card.title }}</h4>
          </div>
          <p class="muted">{{ card.summary }}</p>
        </RouterLink>
      </div>
    </article>

    <article class="panel full system-workspace-panel">
      <div class="system-subnav">
        <RouterLink v-for="item in systemItems" :key="item.to" :to="item.to">
          {{ item.label }}
        </RouterLink>
      </div>

      <div class="system-workspace-body">
        <RouterView v-slot="{ Component }">
          <KeepAlive>
            <component :is="Component" />
          </KeepAlive>
        </RouterView>
      </div>
    </article>
  </section>
</template>

<style scoped>
.system-summary-panel,
.system-workspace-panel {
  display: grid;
  gap: 18px;
}

.system-summary-head {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}

.system-card-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.system-card {
  display: grid;
  gap: 12px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid var(--line);
  background: var(--panel-strong);
  transition: transform 160ms ease, border-color 160ms ease;
}

.system-card:hover {
  transform: translateY(-2px);
  border-color: rgba(200, 90, 46, 0.28);
}

.system-card h4 {
  margin: 0;
  font-size: 1.1rem;
}

.system-subnav {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--line);
}

.system-subnav a {
  padding: 10px 14px;
  border-radius: 14px;
  border: 1px solid transparent;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.38);
}

.system-subnav a.router-link-active {
  color: var(--ink);
  border-color: var(--line);
  background: var(--panel-strong);
}

.system-workspace-body {
  display: grid;
}

@media (max-width: 960px) {
  .system-summary-head {
    flex-direction: column;
  }

  .system-card-grid {
    grid-template-columns: 1fr;
  }
}
</style>
