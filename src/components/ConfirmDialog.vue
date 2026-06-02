<script setup lang="ts">
defineProps<{
  open: boolean
  title: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
  loading?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="confirm-overlay" @click.self="emit('cancel')">
      <div class="confirm-dialog">
        <div class="confirm-title">{{ title }}</div>
        <div class="confirm-message">{{ message }}</div>
        <div class="confirm-actions">
          <button class="ghost-btn" type="button" @click="emit('cancel')">
            {{ cancelLabel || "取消" }}
          </button>
          <button class="danger-btn" type="button" :disabled="loading" @click="emit('confirm')">
            {{ loading ? "处理中..." : (confirmLabel || "确认删除") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 1200;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(3, 6, 10, 0.42);
}

.confirm-dialog {
  width: min(100%, 420px);
  padding: 22px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: var(--surface-bg);
  box-shadow: var(--shadow-float);
}

.confirm-title {
  color: var(--text-primary);
  font-size: 17px;
  font-weight: 700;
}

.confirm-message {
  margin-top: 10px;
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.7;
}

.confirm-actions {
  margin-top: 18px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.ghost-btn,
.danger-btn {
  padding: 9px 14px;
  border-radius: 12px;
  font: inherit;
  transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.ghost-btn {
  border: 1px solid var(--border-color);
  background: var(--control-bg);
  color: var(--text-secondary);
}

.ghost-btn:hover {
  border-color: var(--border-hover);
  background: var(--control-bg-focus);
  color: var(--text-primary);
}

.danger-btn {
  border: 1px solid rgba(248, 113, 113, 0.2);
  background: rgba(248, 113, 113, 0.12);
  color: #fecaca;
}

.danger-btn:hover {
  background: rgba(248, 113, 113, 0.18);
}

.danger-btn:disabled,
.ghost-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
