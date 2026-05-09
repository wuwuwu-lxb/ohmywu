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
  background: var(--bg-surface);
  border-left: 1px solid var(--border-subtle);
  overflow: hidden;
  transition: width 0.2s ease;
  display: flex;
  flex-direction: column;
}

.right-panel.open {
  width: var(--right-panel-w);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 12px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 13px;
  padding: 4px 6px;
  border-radius: var(--radius-sm);
}

.panel-close:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}
</style>
