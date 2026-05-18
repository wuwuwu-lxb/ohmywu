<script setup lang="ts">
import { ref, onMounted } from "vue"
import { invoke } from "@tauri-apps/api/core"

interface Action {
  id: string
  description: string
}

const actions = ref<Action[]>([])

onMounted(async () => {
  try {
    actions.value = await invoke("get_actions")
  } catch {
    // Tauri not ready in dev
  }
})
</script>

<template>
  <div class="actions-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">Actions</h2>
        <p class="view-subtitle">稳定能力入口。这里展示当前已经注册给桌面 Agent 的动作能力。</p>
      </div>
      <span class="section-count">{{ actions.length }}</span>
    </header>

    <div class="action-list">
      <div v-for="a in actions" :key="a.id" class="action-card">
        <div class="action-main">
          <span class="action-id">{{ a.id }}</span>
          <span class="action-desc">{{ a.description }}</span>
        </div>
        <span class="action-pill">stable</span>
      </div>
    </div>

    <div v-if="!actions.length" class="empty-state">
      <p>暂无注册的 Action</p>
    </div>
  </div>
</template>

<style scoped>
.actions-view {
  padding: 28px 32px 32px;
  max-width: 760px;
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.view-subtitle {
  max-width: 520px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.section-count {
  flex-shrink: 0;
  min-width: 40px;
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
  text-align: center;
}

.action-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.action-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 18px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  border-radius: 18px;
  box-shadow: var(--shadow-surface);
  transition: background 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
}

.action-card:hover {
  border-color: rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.06);
  transform: translateY(-1px);
}

.action-main {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.action-id {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
}

.action-desc {
  font-size: 13px;
  line-height: 1.55;
  color: var(--text-primary);
}

.action-pill {
  flex-shrink: 0;
  padding: 5px 10px;
  border-radius: 999px;
  background: var(--surface-2);
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
  text-transform: uppercase;
}

.empty-state {
  margin-top: 14px;
  padding: 34px;
  text-align: center;
  border: 1px dashed var(--border-color);
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.018);
  color: var(--text-tertiary);
  font-size: 14px;
}
</style>
