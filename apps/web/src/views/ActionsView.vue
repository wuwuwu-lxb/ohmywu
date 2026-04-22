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
      <p class="eyebrow">action surface</p>
      <h3>稳定能力资产页</h3>
      <p class="muted">
        这里保留 Action 的独立导航位，强调它是用户模式和未来 Agent 模式共享的能力表面。当前阶段先保留结构位置，后续再承接动态增长的稳定能力池。
      </p>
      <p v-if="error" class="muted">{{ error }}</p>
    </article>

    <article class="panel full action-placeholder">
      <div>
        <p class="eyebrow">coming next</p>
        <h3>Action 先占位，不抢本轮主路径</h3>
      </div>
      <p class="muted">
        这一轮先保证系统能力中心与任务追踪闭环稳定。Action 继续作为长期重要资产保留独立页面，避免后续再拆导航。
      </p>
      <div class="action-grid compact">
        <article v-for="action in actions.slice(0, 6)" :key="action.name" class="action-card muted-card">
          <div>
            <p class="eyebrow">{{ action.domain }}</p>
            <h3>{{ action.title }}</h3>
          </div>
          <p class="muted">{{ action.summary }}</p>
          <p><code>/{{ action.name }}</code></p>
        </article>
      </div>
    </article>
  </section>
</template>

<style scoped>
.action-placeholder {
  display: grid;
  gap: 18px;
}

.compact {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.muted-card {
  opacity: 0.82;
}

@media (max-width: 960px) {
  .compact {
    grid-template-columns: 1fr;
  }
}
</style>
