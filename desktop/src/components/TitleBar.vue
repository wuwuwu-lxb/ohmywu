<template>
  <div class="title-bar" @mousedown="onDragStart">
    <div class="title-bar-drag">
      <span class="title-text">OhMyWu</span>
      <span v-if="version" class="version-tag">v{{ version }}</span>
    </div>
    <div class="window-controls">
      <button
        class="control-btn minimize"
        @click="minimize"
        title="最小化"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <rect x="1" y="5.5" width="10" height="1" fill="currentColor"/>
        </svg>
      </button>
      <button
        class="control-btn maximize"
        @click="maximize"
        :title="isMaximized ? '还原' : '最大化'"
      >
        <svg v-if="!isMaximized" width="12" height="12" viewBox="0 0 12 12">
          <rect x="1.5" y="1.5" width="9" height="9" stroke="currentColor" stroke-width="1" fill="none"/>
        </svg>
        <svg v-else width="12" height="12" viewBox="0 0 12 12">
          <rect x="3" y="0.5" width="8" height="8" stroke="currentColor" stroke-width="1" fill="none"/>
          <rect x="0.5" y="3" width="8" height="8" stroke="currentColor" stroke-width="1" fill="var(--bg-dark)"/>
        </svg>
      </button>
      <button
        class="control-btn close"
        @click="close"
        title="关闭"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M1 1L11 11M11 1L1 11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const isMaximized = ref(false)
const version = ref('')

onMounted(async () => {
  if (window.ohmywu) {
    isMaximized.value = await window.ohmywu.window.isMaximized()
    version.value = await window.ohmywu.app.getVersion()
  }
})

async function minimize() {
  await window.ohmywu?.window.minimize()
}

async function maximize() {
  await window.ohmywu?.window.maximize()
  isMaximized.value = await window.ohmywu?.window.isMaximized() ?? false
}

async function close() {
  await window.ohmywu?.window.close()
}

function onDragStart(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('.window-controls')) {
    e.preventDefault()
  }
}
</script>

<style scoped>
.title-bar {
  height: 32px;
  background: var(--bg-dark);
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 12px;
  -webkit-app-region: drag;
  user-select: none;
  position: sticky;
  top: 0;
  z-index: 100;
}

.title-bar-drag {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-text {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-on-dark);
}

.version-tag {
  font-size: 10px;
  color: var(--text-on-dark-muted);
  background: rgba(255, 255, 255, 0.05);
  padding: 1px 6px;
  border-radius: 4px;
}

.window-controls {
  display: flex;
  gap: 8px;
  -webkit-app-region: no-drag;
}

.control-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  color: var(--text-on-dark-muted);
  background: transparent;
}

.control-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-on-dark);
}

.control-btn.close:hover {
  background: #e81123;
  color: #fff;
}

.control-btn:active {
  transform: scale(0.95);
}
</style>
