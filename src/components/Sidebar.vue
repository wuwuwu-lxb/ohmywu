<script setup lang="ts">
import { useSidebarNav } from "../composables/useNav"
import { useSidebar } from "../composables/useTheme"

defineProps<{ activeId?: string }>()
const { items } = useSidebarNav()
const { collapsed, toggle } = useSidebar()
const emit = defineEmits<{ select: [id: string] }>()
</script>

<template>
  <aside :class="['sidebar', { collapsed }]">
    <div class="sidebar-header">
      <div v-if="!collapsed" class="brand">
        <span class="brand-mark">✦</span>
        <span class="brand-text">OhMyWu</span>
      </div>
      <button class="toggle-btn" @click="toggle" :title="collapsed ? '展开' : '折叠'">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <rect x="1" y="2" width="12" height="1.5" rx="0.75" fill="currentColor"/>
          <rect x="1" y="6" width="12" height="1.5" rx="0.75" fill="currentColor"/>
          <rect x="1" y="10" width="12" height="1.5" rx="0.75" fill="currentColor"/>
        </svg>
      </button>
    </div>

    <nav v-if="!collapsed" class="sidebar-nav">
      <button
        v-for="item in items"
        :key="item.id"
        :class="['nav-item', { active: item.id === activeId }]"
        @click="emit('select', item.id)"
      >
        <span class="nav-icon">{{ item.icon }}</span>
        <span class="nav-label truncate">{{ item.label }}</span>
        <span v-if="item.badge" class="nav-badge">{{ item.badge }}</span>
      </button>
    </nav>

    <div v-if="!collapsed" class="sidebar-footer">
      <div class="footer-divider" />
      <button
        :class="['nav-item', { active: activeId === '__settings__' }]"
        @click="emit('select', '__settings__')"
      >
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
  background: var(--surface-2);
  border-right: 1px solid var(--border-color);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transition: width var(--duration-normal) var(--ease-in-out), background 0.3s ease;
  overflow: hidden;
  animation: slideIn 0.4s var(--ease-out);
}

@keyframes slideIn {
  from { transform: translateX(-20px); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.sidebar.collapsed {
  width: 40px;
}

/* header */
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 12px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-mark {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-xs);
  background: linear-gradient(135deg, var(--accent-soft), color-mix(in srgb, var(--accent) 20%, transparent));
  color: var(--accent);
  font-size: 12px;
}

.brand-text {
  font-weight: 700;
  font-size: 13px;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.toggle-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-radius: var(--radius-xs);
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--duration-fast) var(--ease-out);
}

.toggle-btn:hover {
  background: var(--hover-bg);
  color: var(--text-secondary);
}

/* nav */
.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-family: var(--font);
  font-weight: 500;
  cursor: pointer;
  text-align: left;
  transition: all var(--duration-fast) var(--ease-out);
}

.nav-item:hover {
  background: var(--hover-bg);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--active-bg);
  color: var(--text-primary);
}

.nav-icon {
  font-size: 15px;
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
  line-height: 1.4;
}

/* footer */
.sidebar-footer {
  padding: 6px;
}

.footer-divider {
  height: 1px;
  background: var(--border-color);
  margin-bottom: 6px;
}
</style>
