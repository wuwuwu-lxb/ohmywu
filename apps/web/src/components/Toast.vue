<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  message: string
  type?: 'success' | 'error' | 'info'
}>()

const visible = ref(true)

onMounted(() => {
  setTimeout(() => {
    visible.value = false
  }, 3000)
})

function close() {
  visible.value = false
}
</script>

<template>
  <Transition name="toast-slide">
    <div v-if="visible" class="toast" :class="`toast-${type || 'info'}`">
      <span class="toast-icon">
        <template v-if="type === 'success'">&#10003;</template>
        <template v-else-if="type === 'error'">&#10007;</template>
        <template v-else>&#9432;</template>
      </span>
      <span class="toast-message">{{ message }}</span>
      <button class="toast-close" @click="close">&#10005;</button>
    </div>
  </Transition>
</template>

<style scoped>
.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 10px;
  font-size: 14px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  min-width: 220px;
  max-width: 360px;
}

.toast-success {
  background: #e8f5e9;
  border: 1px solid #a5d6a7;
  color: #2e7d32;
}

.toast-error {
  background: #ffebee;
  border: 1px solid #ef9a9a;
  color: #c62828;
}

.toast-info {
  background: #e3f2fd;
  border: 1px solid #90caf9;
  color: #1565c0;
}

.toast-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.toast-message {
  flex: 1;
}

.toast-close {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 0;
  opacity: 0.6;
  flex-shrink: 0;
}

.toast-close:hover {
  opacity: 1;
}

.toast-slide-enter-active,
.toast-slide-leave-active {
  transition: all 0.3s ease;
}

.toast-slide-enter-from,
.toast-slide-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
