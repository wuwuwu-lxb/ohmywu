<script setup lang="ts">
import { ref, onMounted, markRaw, computed } from "vue"
import { convertFileSrc, invoke } from "@tauri-apps/api/core"
import Sidebar from "./components/Sidebar.vue"
import RightPanel from "./components/RightPanel.vue"
import { useSidebarNav } from "./composables/useNav"
import { useTheme } from "./composables/useTheme"
import { useAgentStore } from "./stores/agents"
import { useChatStore } from "./stores/chat"
import ChatView from "./views/ChatView.vue"
import AgentManagementView from "./views/AgentManagementView.vue"
import ActionsView from "./views/ActionsView.vue"
import AtomicCapabilitiesView from "./views/AtomicCapabilitiesView.vue"
import AuditView from "./views/AuditView.vue"
import ModelSettingsView from "./views/ModelSettingsView.vue"
import SettingsView from "./views/SettingsView.vue"
import WikiView from "./views/WikiView.vue"
import type { Component } from "vue"

const { items, register } = useSidebarNav()
const chatStore = useChatStore()
const agentStore = useAgentStore()
const {
  initFromConfig,
  backgroundImageUrl,
  setBackgroundImage,
  backgroundMode,
} = useTheme()

const activeView = ref<string>("chat")
const rightPanelOpen = ref(false)
const rightPanelTaskId = ref<string | null>(null)

const viewMap: Record<string, Component> = {
  chat: markRaw(ChatView),
  agents: markRaw(AgentManagementView),
  wiki: markRaw(WikiView),
  models: markRaw(ModelSettingsView),
  atomic: markRaw(AtomicCapabilitiesView),
  actions: markRaw(ActionsView),
  audit: markRaw(AuditView),
  __settings__: markRaw(SettingsView),
}

const onNavSelect = (id: string) => {
  activeView.value = id
}

const handleShowTask = (taskId: string) => {
  rightPanelTaskId.value = taskId
  rightPanelOpen.value = true
}

const currentViewLabel = computed(() => {
  if (activeView.value === "chat") {
    return chatStore.panel === "manager" ? "对话管理" : "对话"
  }
  if (activeView.value === "__settings__") return "设置"
  return items.value.find((item) => item.id === activeView.value)?.label || "OhMyWu"
})

const showChatManagerArrow = computed(
  () => activeView.value === "chat" && chatStore.panel === "conversation"
)

const showChatConversationArrow = computed(
  () => activeView.value === "chat" && chatStore.panel === "manager"
)

onMounted(async () => {
  await agentStore.init()
  try {
    const cfg = await invoke<{
      theme: string; accent: string; background_mode: string;
      background_solid_color: string;
      background_preset: string;
      surface_opacity: number; background_scale: number;
      background_blur: number; background_mask_opacity: number;
      background_auto_theme: boolean;
      background_theme_color?: string | null;
    }>("get_config")
    initFromConfig(cfg)

    // Load custom background image in image mode
    if (cfg.background_mode === "image") {
      try {
        const bgPath = await invoke<string | null>("get_background_path")
        if (bgPath) {
          const url = convertFileSrc(bgPath)
          await setBackgroundImage(url, { syncTheme: !cfg.background_theme_color && cfg.background_auto_theme })
        }
      } catch { /* no saved bg */ }
    }
  } catch (e) {
    console.error("Init config:", e)
  }
  register({ id: "chat", label: "对话", icon: "💬" })
  register({ id: "agents", label: "Agent 管理", icon: "🧠" })
  register({ id: "wiki", label: "知识库", icon: "📖" })
  register({ id: "models", label: "模型设置", icon: "◌" })
  register({ id: "atomic", label: "原子化能力", icon: "⚙" })
  register({ id: "actions", label: "Action 注册", icon: "⚡" })
  register({ id: "audit", label: "审计日志", icon: "📋" })
})
</script>

<template>
  <div class="app-shell">
    <!-- SPlayer AppLayout pattern: background container behind everything -->
    <div
      v-if="backgroundMode === 'image'"
      class="background-container"
    >
      <div class="background-ambient" />
      <div
        class="background-media"
        :style="{
          backgroundImage: backgroundImageUrl ? `url(${backgroundImageUrl})` : 'none',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          transform: `scale(var(--bg-scale))`,
          filter: `blur(var(--bg-blur))`,
        }"
      />
      <div class="background-mask" :style="{ background: `rgba(0,0,0, var(--bg-mask))` }" />
    </div>

    <div class="app-container">
      <Sidebar :active-id="activeView" @select="onNavSelect" />

      <main class="main-area">
        <header class="topbar">
          <div class="topbar-left">
            <button
              v-if="showChatManagerArrow"
              class="topbar-btn"
              type="button"
              aria-label="open manager"
              @click="chatStore.setPanel('manager')"
            >
              <span>‹</span>
            </button>
            <button
              v-else-if="showChatConversationArrow"
              class="topbar-btn"
              type="button"
              aria-label="open conversation"
              @click="chatStore.setPanel('conversation')"
            >
              <span>›</span>
            </button>
            <div class="topbar-title" data-tauri-drag-region>
              <h1>{{ currentViewLabel }}</h1>
            </div>
          </div>
        </header>

        <section class="content-frame">
          <ChatView v-if="activeView === 'chat'" @show-task="handleShowTask" />
          <component v-else :is="viewMap[activeView] || viewMap['chat']" />
        </section>
      </main>

      <RightPanel :open="rightPanelOpen" title="执行链路" @close="rightPanelOpen = false">
        <div v-if="rightPanelTaskId">
          <p>Task ID: {{ rightPanelTaskId }}</p>
          <p class="panel-hint">执行详情会在这里展开。</p>
        </div>
        <p v-else class="panel-placeholder">选中一条消息查看执行详情。</p>
      </RightPanel>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100%;
  width: 100%;
  position: relative;
  overflow: hidden;
  background: var(--bg-gradient);
}

.background-container {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
}

.background-ambient {
  position: absolute;
  inset: 0;
  background: rgba(6, 8, 12, 0.18);
}

.background-media {
  position: absolute;
  inset: 0;
  background-size: cover;
  background-position: center;
  transform-origin: center;
}

.background-mask {
  position: absolute;
  inset: 0;
}

.app-container {
  position: relative;
  z-index: 1;
  display: flex;
  height: 100%;
  background: transparent;
  overflow: hidden;
}

.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--shell-bg);
  animation: fadeIn 0.5s 0.1s var(--ease-out) both;
}

.topbar {
  height: 70px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 24px;
  border-bottom: 1px solid var(--border-color);
  background: var(--shell-bg-soft);
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.topbar-btn,
.topbar-chip {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.topbar-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  font-size: 24px;
  line-height: 1;
}

.topbar-btn:hover,
.topbar-chip:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.topbar-title h1 {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
}

.topbar-chip {
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
  font-size: 12px;
  font-family: var(--font-mono);
}

.content-frame {
  flex: 1;
  min-height: 0;
  padding: 0 24px 0;
  overflow: hidden;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.panel-hint { margin-top: 8px; font-size: 12px; color: var(--text-tertiary); }
.panel-placeholder { color: var(--text-tertiary); }

@media (max-width: 960px) {
  .topbar {
    padding: 0 16px;
  }

  .content-frame {
    padding: 0 16px;
  }
}
</style>
