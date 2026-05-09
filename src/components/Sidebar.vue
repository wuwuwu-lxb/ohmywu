<script setup lang="ts">
import { useSidebarNav } from "../composables/useNav"
import { useSidebar } from "../composables/useTheme"

const { items } = useSidebarNav()
const { collapsed, toggle } = useSidebar()
const emit = defineEmits<{ select: [id: string] }>()
</script>

<template>
  <aside :class="['sidebar', { collapsed }]">
    <div class="sidebar-header">
      <span v-if="!collapsed" class="sidebar-brand">OhMyWu</span>
      <button class="sidebar-toggle" @click="toggle" :title="collapsed ? '展开侧栏' : '折叠侧栏'">
        <span v-if="collapsed">☰</span>
        <span v-else>✕</span>
      </button>
    </div>

    <nav v-if="!collapsed" class="sidebar-nav">
      <button
        v-for="item in items"
        :key="item.id"
        class="nav-item"
        @click="emit('select', item.id)"
      >
        <span class="nav-icon">{{ item.icon }}</span>
        <span class="nav-label truncate">{{ item.label }}</span>
        <span v-if="item.badge" class="nav-badge">{{ item.badge }}</span>
      </button>
    </nav>

    <div v-if="!collapsed" class="sidebar-footer">
      <button class="nav-item" @click="emit('select', '__settings__')">
        <span class="nav-icon">⚙</span>
        <span class="nav-label">设置</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: var(--sidebar-w);
  min-width: 0;
  background: var(--bg-surface);
  border-right: 1px solid var(--border-subtle);
  transition: width 0.2s ease, opacity 0.2s ease;
  overflow: hidden;
}

.sidebar.collapsed {
  width: var(--sidebar-collapsed-w);
  opacity: 0;
  pointer-events: none;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.sidebar-brand {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}

.sidebar-toggle {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 14px;
  padding: 4px 6px;
  border-radius: var(--radius-sm);
}

.sidebar-toggle:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.nav-label {
  flex: 1;
}

.nav-badge {
  background: var(--accent);
  color: var(--text-on-accent);
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 10px;
  min-width: 18px;
  text-align: center;
}

.sidebar-footer {
  padding: 8px;
  border-top: 1px solid var(--border-subtle);
}
</style>
