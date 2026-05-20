<script setup lang="ts">
interface ExecutionStep {
  action: string
  status: "running" | "success" | "failed"
  detail: string
  duration?: string
}

defineProps<{
  open: boolean
  title: string
  steps?: ExecutionStep[]
}>()

const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <aside :class="['right-panel', { open }]">
    <div class="panel-header">
      <span class="panel-title truncate">{{ title }}</span>
      <button class="panel-close" @click="emit('close')">✕</button>
    </div>

    <div class="panel-body">
      <slot />
    </div>
  </aside>
</template>

<style scoped>
.right-panel {
  width: 0;
  min-width: 0;
  background: var(--shell-bg);
  border-left: 1px solid var(--border-color);
  overflow: hidden;
  transition: width 0.3s var(--ease-out), background 0.3s ease;
  display: flex;
  flex-direction: column;
}

.right-panel.open {
  width: var(--right-panel-w);
  box-shadow: -18px 0 40px rgba(0, 0, 0, 0.16);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 14px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.panel-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.panel-close {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 14px;
  border-radius: var(--radius-xs);
  transition: all var(--duration-fast) var(--ease-out);
}

.panel-close:hover {
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.18);
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
  font-size: var(--text-sm);
  line-height: 1.7;
  color: var(--text-secondary);
}
</style>
