<script setup lang="ts">
import { ref, onMounted, markRaw } from "vue"
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
const { preset } = useTheme()

const activeView = ref<string>("chat")
const rightPanelOpen = ref(false)

const viewMap: Record<string, Component> = {
  chat: markRaw(ChatView),
  actions: markRaw(ActionsView),
  audit: markRaw(AuditView),
  __settings__: markRaw(SettingsView),
}

const onNavSelect = (id: string) => {
  activeView.value = id
}

onMounted(() => {
  register({ id: "chat", label: "对话", icon: "💬" })
  register({ id: "actions", label: "Actions", icon: "⚡" })
  register({ id: "audit", label: "审计日志", icon: "📋" })
})
</script>

<template>
  <div class="app-shell" :data-theme="preset">
    <Sidebar @select="onNavSelect" />

    <main class="main-area">
      <component :is="viewMap[activeView] || viewMap['chat']" />
    </main>

    <RightPanel
      :open="rightPanelOpen"
      title="执行链路"
      @close="rightPanelOpen = false"
    >
      <p>选中一条消息后，这里会显示执行链路详情。</p>
    </RightPanel>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
  background: var(--bg-base);
}

.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
