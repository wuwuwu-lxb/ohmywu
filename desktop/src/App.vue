<template>
  <div class="app-shell">
    <TitleBar />
    <div class="app-content">
      <!-- Sidebar -->
      <aside class="sidebar">
        <div class="sidebar-inner">
          <!-- Brand -->
          <div class="brand">
            <div class="brand-mark">
              <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
                <circle cx="14" cy="14" r="13" stroke="#C9A96E" stroke-width="1.5"/>
                <path d="M9 14 C9 10.5, 12 8, 14 8 C16 8, 19 10.5, 19 14" stroke="#C9A96E" stroke-width="1.5" stroke-linecap="round" fill="none"/>
                <circle cx="14" cy="17" r="2" fill="#C9A96E" opacity="0.6"/>
              </svg>
            </div>
            <div class="brand-text">
              <span class="brand-name">OhMyWu</span>
              <span class="brand-status">
                <span class="status-dot"></span>
                运行中
              </span>
            </div>
          </div>

          <!-- Navigation -->
          <nav class="nav">
            <span class="nav-label">导航</span>
            <router-link
              v-for="(item, i) in navItems"
              :key="item.path"
              :to="item.path"
              class="nav-item"
              :style="{ animationDelay: `${i * 0.06}s` }"
            >
              <span class="nav-icon" v-html="item.icon"></span>
              <span class="nav-text">{{ item.label }}</span>
              <span v-if="item.badge" class="nav-badge">{{ item.badge }}</span>
            </router-link>
          </nav>

          <!-- Bottom section -->
          <div class="sidebar-bottom">
            <div class="system-indicator">
              <div class="indicator-bar">
                <div class="indicator-fill" style="width: 32%"></div>
              </div>
              <span class="indicator-label">负载 0.32</span>
            </div>
          </div>
        </div>
      </aside>

      <!-- Main -->
      <main class="main">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import TitleBar from './components/TitleBar.vue'

const navItems = [
  {
    path: '/overview',
    label: '总览',
    icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><rect x="1" y="1" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2"/><rect x="9" y="1" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2"/><rect x="1" y="9" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2"/><rect x="9" y="9" width="6" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2"/></svg>`,
  },
  {
    path: '/system',
    label: '系统',
    icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.2"/><path d="M8 4.5V8L10.5 10.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>`,
  },
  {
    path: '/actions',
    label: '动作',
    icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3 8h10M8 3v10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>`,
  },
  {
    path: '/conversation',
    label: '对话',
    icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2 3.5A1.5 1.5 0 013.5 2h9A1.5 1.5 0 0114 3.5v7A1.5 1.5 0 0112.5 12H9l-3 2.5V12H3.5A1.5 1.5 0 012 10.5v-7z" stroke="currentColor" stroke-width="1.2"/></svg>`,
  },
]
</script>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-primary);
}

.app-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* ── Sidebar ── */
.sidebar {
  width: 220px;
  flex-shrink: 0;
  background: var(--bg-dark);
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

.sidebar::before {
  content: '';
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse at 50% 0%, rgba(201,169,110,0.06) 0%, transparent 60%);
  pointer-events: none;
}

.sidebar-inner {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 28px 20px;
}

/* Brand */
.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 32px;
  border-bottom: 1px solid rgba(232,226,217,0.08);
  margin-bottom: 28px;
}

.brand-mark {
  width: 40px;
  height: 40px;
  background: rgba(201,169,110,0.08);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(201,169,110,0.15);
}

.brand-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand-name {
  font-family: var(--font-display);
  font-size: 18px;
  font-weight: 600;
  color: var(--text-on-dark);
  letter-spacing: 0.02em;
}

.brand-status {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: var(--text-on-dark-muted);
}

.status-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #7EC8A0;
  box-shadow: 0 0 6px rgba(126,200,160,0.6);
  animation: pulse 3s ease-in-out infinite;
}

/* Nav */
.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
}

.nav-label {
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-on-dark-muted);
  padding: 0 10px;
  margin-bottom: 8px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  color: var(--text-on-dark-muted);
  text-decoration: none;
  font-size: 13.5px;
  font-weight: 400;
  transition:
    background var(--transition-fast),
    color var(--transition-fast),
    transform var(--transition-fast);
  animation: fadeSlideIn 0.4s var(--transition-slow) both;
  position: relative;
}

.nav-item:hover {
  background: rgba(232,226,217,0.06);
  color: var(--text-on-dark);
}

.nav-item.router-link-active {
  background: rgba(201,169,110,0.1);
  color: var(--accent);
}

.nav-item.router-link-active .nav-icon {
  color: var(--accent);
}

.nav-icon {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.nav-text {
  flex: 1;
}

.nav-badge {
  background: rgba(201,169,110,0.15);
  color: var(--accent);
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 4px;
}

/* Bottom */
.sidebar-bottom {
  padding-top: 20px;
  border-top: 1px solid rgba(232,226,217,0.08);
}

.system-indicator {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.indicator-bar {
  height: 2px;
  background: rgba(232,226,217,0.1);
  border-radius: 1px;
  overflow: hidden;
}

.indicator-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), rgba(201,169,110,0.4));
  border-radius: 1px;
  transition: width 1s ease;
}

.indicator-label {
  font-size: 10px;
  color: var(--text-on-dark-muted);
}

/* ── Main ── */
.main {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  background: var(--bg-primary);
}

/* Page transition */
.page-enter-active,
.page-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.page-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
