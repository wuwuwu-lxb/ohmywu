<script setup lang="ts">
defineProps<{
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <Teleport to="body">
    <div class="overlay" @click.self="emit('cancel')">
      <div class="dialog">
        <div class="dialog-header">
          <h4 class="dialog-title">{{ title }}</h4>
          <button class="close-btn" @click="emit('cancel')">&#10005;</button>
        </div>
        <div class="dialog-body">
          <p class="dialog-message">{{ message }}</p>
        </div>
        <div class="dialog-footer">
          <button class="btn btn-cancel" @click="emit('cancel')">
            {{ cancelText || '取消' }}
          </button>
          <button
            class="btn"
            :class="danger ? 'btn-danger' : 'btn-confirm'"
            @click="emit('confirm')"
          >
            {{ confirmText || '确认' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  backdrop-filter: blur(2px);
}

.dialog {
  background: #fff;
  border-radius: 14px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  min-width: 320px;
  max-width: 420px;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #f0ebe3;
}

.dialog-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #1a1612;
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 16px;
  color: #888;
  padding: 4px;
}

.close-btn:hover {
  color: #1a1612;
}

.dialog-body {
  padding: 20px;
}

.dialog-message {
  margin: 0;
  font-size: 14px;
  color: #555;
  line-height: 1.5;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 20px;
  background: #faf8f5;
}

.btn {
  padding: 8px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}

.btn-cancel {
  background: #fff;
  border-color: #ddd;
  color: #555;
}

.btn-cancel:hover {
  background: #f5f0ea;
  color: #1a1612;
}

.btn-confirm {
  background: var(--accent, #c9a96e);
  color: #fff;
}

.btn-confirm:hover {
  background: #b8954f;
}

.btn-danger {
  background: #d32f2f;
  color: #fff;
}

.btn-danger:hover {
  background: #b71c1c;
}
</style>
