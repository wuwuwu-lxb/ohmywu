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
    <h2 class="view-title">Actions</h2>
    <p class="view-subtitle">所有可调用的稳定能力</p>

    <div class="action-list">
      <div v-for="a in actions" :key="a.id" class="action-card">
        <span class="action-id">{{ a.id }}</span>
        <span class="action-desc">{{ a.description }}</span>
      </div>
    </div>

    <div v-if="!actions.length" class="empty-state">
      <p>暂无注册的 Action</p>
    </div>
  </div>
</template>

<style scoped>
.actions-view {
  padding: 24px 32px;
  max-width: 640px;
}

.view-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.view-subtitle {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-bottom: 20px;
}

.action-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.action-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: var(--surface-1);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-surface);
  transition: background 0.15s ease, border-color 0.15s ease;
}

.action-card:hover {
  border-color: var(--border-hover);
  background: var(--surface-2);
}

.action-id {
  font-family: var(--font-mono);
  font-size: 13px;
  font-weight: 600;
  color: var(--accent);
  min-width: 100px;
}

.action-desc {
  font-size: 13px;
  color: var(--text-secondary);
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 14px;
}
</style>
