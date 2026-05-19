<script setup lang="ts">
import { useSidebarNav } from "../composables/useNav"
import { useSidebar } from "../composables/useTheme"
import { computed } from "vue"

defineProps<{ activeId?: string }>()
const { items } = useSidebarNav()
const { collapsed, toggle } = useSidebar()
const emit = defineEmits<{ select: [id: string] }>()

const iconMap = computed<Record<string, string>>(() => ({
  chat: "M4 5.5h8M4 8.5h5m-4.5 4 1.8-2.4a1 1 0 0 1 .8-.4H12a2 2 0 0 0 2-2v-6A2 2 0 0 0 12 2H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h.6a1 1 0 0 1 .8.4Z",
  agents: "M5 5.5a2 2 0 1 1 0-4 2 2 0 0 1 0 4Zm6 1a1.75 1.75 0 1 0 0-3.5A1.75 1.75 0 0 0 11 6.5ZM2.5 13a2.5 2.5 0 0 1 5 0M8.5 13a2.5 2.5 0 0 1 5 0",
  wiki: "M4 2.5h7.5A2.5 2.5 0 0 1 14 5v7.5A1.5 1.5 0 0 1 12.5 14H5a3 3 0 0 0-3 3V5.5A3 3 0 0 1 5 2.5Zm0 0A2.5 2.5 0 0 0 1.5 5V14",
  actions: "M8.5 1.5 3 8h3l-.5 6.5L11 8H8l.5-6.5Z",
  audit: "M4 3.5h8M4 6.5h8M4 9.5h5m3 2.5 1.2 1.2L15.5 11M13 14a2 2 0 1 1 0-4 2 2 0 0 1 0 4ZM3.5 2h9A1.5 1.5 0 0 1 14 3.5v5",
  __settings__: "M8 1.75 9.1 3.2l1.75.25.8 1.55-1.15 1.35.25 1.75-1.55.8L8 7.75l-1.2 1.15-1.55-.8.25-1.75L4.35 5l.8-1.55L6.9 3.2 8 1.75Zm0 8.5a2.25 2.25 0 1 0 0 4.5 2.25 2.25 0 0 0 0-4.5Z",
}))
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
        <span class="nav-icon">
          <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              :d="iconMap[item.id] || iconMap.chat"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </span>
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
        <span class="nav-icon">
          <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              :d="iconMap.__settings__"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </span>
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
  background: var(--shell-bg);
  border-right: 1px solid var(--border-color);
  backdrop-filter: blur(var(--shell-blur));
  -webkit-backdrop-filter: blur(var(--shell-blur));
  transition: width var(--duration-normal) var(--ease-in-out), background 0.3s ease;
  overflow: hidden;
  animation: slideIn 0.4s var(--ease-out);
  box-shadow: inset -1px 0 0 rgba(255, 255, 255, 0.02);
}

@keyframes slideIn {
  from { transform: translateX(-20px); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.sidebar.collapsed {
  width: 64px;
}

/* header */
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--titlebar-h);
  padding: 0 18px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
}

.brand-mark {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: rgba(var(--accent-rgb), 0.18);
  border: 1px solid rgba(var(--accent-rgb), 0.24);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06), var(--shadow-glow);
  color: #f4f6ff;
  font-size: 13px;
}

.brand-text {
  font-weight: 700;
  font-size: 20px;
  color: var(--text-primary);
  letter-spacing: 0;
  line-height: 1;
  transition: width 0.3s, opacity 0.3s, margin 0.3s;
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
  background: var(--surface-2);
  color: var(--text-primary);
}

/* nav */
.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 0 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 12px 14px;
  border: 1px solid transparent;
  border-radius: 12px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-family: var(--font);
  font-weight: 500;
  cursor: pointer;
  text-align: left;
  transition: all var(--duration-fast) var(--ease-out);
  position: relative;
}

.nav-item:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.nav-item.active {
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.14);
  color: var(--text-primary);
}

.nav-item.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 4px;
  border-radius: 0 4px 4px 0;
  background: rgba(var(--accent-rgb), 0.9);
}

.nav-icon {
  width: 22px;
  height: 22px;
  text-align: center;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.nav-icon svg {
  width: 18px;
  height: 18px;
}

.nav-label {
  flex: 1;
}

.nav-badge {
  background: rgba(var(--accent-rgb), 0.88);
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
  padding: 8px 10px 14px;
}

.footer-divider {
  height: 1px;
  background: var(--border-color);
  margin-bottom: 8px;
}

.sidebar.collapsed .brand-text,
.sidebar.collapsed .nav-label,
.sidebar.collapsed .nav-badge {
  width: 0;
  opacity: 0;
  margin: 0;
}
</style>
