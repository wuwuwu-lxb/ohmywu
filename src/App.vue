<script setup lang="ts">
import { ref, onMounted, markRaw } from "vue"
import { invoke } from "@tauri-apps/api/core"
import Sidebar from "./components/Sidebar.vue"
import RightPanel from "./components/RightPanel.vue"
import { useSidebarNav } from "./composables/useNav"
import { useTheme } from "./composables/useTheme"
import ChatView from "./views/ChatView.vue"
import ActionsView from "./views/ActionsView.vue"
import AuditView from "./views/AuditView.vue"
import SettingsView from "./views/SettingsView.vue"
import type { Component } from "vue"

const { register } = useSidebarNav()
const { initFromConfig, setBackgroundImage, setBackgroundVideo, backgroundMode, backgroundVideoUrl } = useTheme()

const activeView = ref<string>("chat")
const rightPanelOpen = ref(false)
const rightPanelTaskId = ref<string | null>(null)

const viewMap: Record<string, Component> = {
  chat: markRaw(ChatView),
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

onMounted(async () => {
  try {
    const cfg = await invoke<{
      theme: string; accent: string; background_mode: string;
      surface_opacity: number; background_scale: number;
      background_blur: number; background_mask_opacity: number;
    }>("get_config")
    initFromConfig(cfg)

    // Load custom background image if in image/video mode
    if (cfg.background_mode === "image" || cfg.background_mode === "video") {
      try {
        const bgPath = await invoke<string | null>("get_background_path")
        if (bgPath) {
          const url = `asset://localhost/${bgPath}`
          if (cfg.background_mode === "video") {
            setBackgroundVideo(url)
          } else {
            setBackgroundImage(url)
          }
        }
      } catch { /* no saved bg */ }
    }
  } catch (e) {
    console.error("Init config:", e)
  }
  register({ id: "chat", label: "对话", icon: "💬" })
  register({ id: "actions", label: "Actions", icon: "⚡" })
  register({ id: "audit", label: "审计日志", icon: "📋" })
})
</script>

<template>
  <div class="app-shell">
    <!-- SPlayer AppLayout pattern: background container behind everything -->
    <div
      v-if="backgroundMode === 'image' || backgroundMode === 'video'"
      class="background-container"
    >
      <div
        v-if="backgroundMode === 'image'"
        class="background-media"
        :style="{
          backgroundImage: `var(--bg-image-url)`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          transform: `scale(var(--bg-scale))`,
          filter: `blur(var(--bg-blur))`,
        }"
      />
      <video
        v-else
        class="background-media"
        :src="backgroundVideoUrl"
        autoplay
        loop
        muted
        :style="{
          objectFit: 'cover',
          transform: `scale(var(--bg-scale))`,
          filter: `blur(var(--bg-blur))`,
        }"
      />
      <div class="background-mask" :style="{ background: `rgba(0,0,0, var(--bg-mask))` }" />
    </div>

    <div class="app-container">
      <Sidebar :active-id="activeView" @select="onNavSelect" />

      <main class="main-area">
        <ChatView v-if="activeView === 'chat'" @show-task="handleShowTask" />
        <component v-else :is="viewMap[activeView] || viewMap['chat']" />
      </main>

      <RightPanel :open="rightPanelOpen" title="执行链路" @close="rightPanelOpen = false">
        <div v-if="rightPanelTaskId">
          <p>Task ID: {{ rightPanelTaskId }}</p>
          <p class="panel-hint">完整执行链路将在后续版本中展示。</p>
        </div>
        <p v-else class="panel-placeholder">选中一条消息后，这里会显示执行链路详情。</p>
      </RightPanel>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  width: 100vw;
  position: relative;
  overflow: hidden;
  background: var(--bg-gradient);
}

/* SPlayer background-container pattern */
.background-container {
  position: fixed;
  inset: 0;
  z-index: 0;
  overflow: hidden;
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
  height: 100vh;
  background: transparent;
  overflow: hidden;
}

.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: fadeIn 0.5s 0.1s var(--ease-out) both;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}

.panel-hint { margin-top: 8px; font-size: 12px; color: var(--text-tertiary); }
.panel-placeholder { color: var(--text-tertiary); }
</style>
