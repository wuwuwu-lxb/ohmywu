<script setup lang="ts">
defineProps<{
  visible: boolean
  message?: string
}>()
</script>

<template>
  <transition name="fade">
    <div v-if="visible" class="loading-overlay">
      <div class="spinner">
        <div class="spinner-ring"></div>
      </div>
      <p v-if="message" class="loading-msg">{{ message }}</p>
    </div>
  </transition>
</template>

<style scoped>
.loading-overlay {
  position: fixed;
  inset: 0;
  background: rgba(245, 240, 229, 0.72);
  backdrop-filter: blur(4px);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  z-index: 9999;
}

.spinner {
  width: 40px;
  height: 40px;
}
.spinner-ring {
  width: 100%;
  height: 100%;
  border: 3px solid rgba(201, 169, 110, 0.2);
  border-top-color: #c9a96e;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-msg {
  font-size: 14px;
  color: #6e6357;
  margin: 0;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
