<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchActions, type ActionSpec } from '@/lib/api'

const actions = ref<ActionSpec[]>([])
const error = ref<string | null>(null)

onMounted(async () => {
  try {
    actions.value = await fetchActions()
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : '加载 Action 列表失败'
  }
})
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">动作中心</p>
      <h3>共享 Action 列表</h3>
      <p class="muted">
        这里展示的是用户和 AI 共用的调用表面。后续无论从工具栏还是对话层进入，都会走同一套 Action 定义。
      </p>
      <p v-if="error" class="muted">{{ error }}</p>
    </article>

    <div class="action-grid">
      <article v-for="action in actions" :key="action.name" class="action-card">
        <div>
          <p class="eyebrow">{{ action.domain }}</p>
          <h3>{{ action.title }}</h3>
        </div>
        <p>{{ action.summary }}</p>
        <p><code>/{{ action.name }}</code></p>
        <p class="muted">
          映射到 {{ action.target.kind }}：<code>{{ action.target.name }}</code>
        </p>
      </article>
    </div>
  </section>
</template>
