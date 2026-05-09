<script setup lang="ts">
import { ref } from "vue"

const showPet = ref(false)

const togglePet = () => {
  showPet.value = !showPet.value
}
</script>

<template>
  <div class="app-shell">
    <header class="titlebar">
      <span class="titlebar-brand">OhMyWu</span>
      <nav class="titlebar-nav">
        <router-link to="/">对话</router-link>
      </nav>
      <button class="pet-toggle" @click="togglePet">
        {{ showPet ? "隐藏桌宠" : "显示桌宠" }}
      </button>
    </header>
    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #0f0f0f;
  color: #e0e0e0;
}

.titlebar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 16px;
  height: 40px;
  background: #1a1a1a;
  border-bottom: 1px solid #2a2a2a;
  user-select: none;
}

.titlebar-brand {
  font-weight: 700;
  font-size: 14px;
  color: #fff;
}

.titlebar-nav a {
  color: #888;
  text-decoration: none;
  font-size: 13px;
  padding: 4px 8px;
  border-radius: 4px;
}

.titlebar-nav a:hover,
.titlebar-nav a.router-link-active {
  color: #fff;
  background: #2a2a2a;
}

.pet-toggle {
  margin-left: auto;
  background: #2a2a2a;
  border: 1px solid #3a3a3a;
  color: #ccc;
  font-size: 12px;
  padding: 4px 12px;
  border-radius: 4px;
  cursor: pointer;
}

.pet-toggle:hover {
  background: #3a3a3a;
}

.main-content {
  flex: 1;
  overflow: hidden;
}
</style>
