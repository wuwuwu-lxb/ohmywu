<script setup lang="ts">
import { ref } from 'vue'

import { resolveReferences, type ResolvedReference } from '@/lib/api'

const input = ref('@system-ops 先执行 /system_overview，然后把结果和 /scan_storage 一起整理出来。')
const references = ref<ResolvedReference[]>([])
const error = ref<string | null>(null)
const pending = ref(false)

async function submit() {
  pending.value = true
  error.value = null

  try {
    const response = await resolveReferences(input.value)
    references.value = response.references
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : '引用解析失败'
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <div class="page-title">
        <p class="eyebrow">对话与编排</p>
        <h3>用户可以手动拆任务，再显式引用 Agent 和 Action。</h3>
      </div>
      <p class="muted">
        先实现引用解析和共享 Action 表面。后续主 Agent 编排会沿用同一套引用协议。
      </p>

      <textarea v-model="input" />

      <div class="metric-row" style="margin-top: 14px;">
        <button :disabled="pending" @click="submit">
          {{ pending ? '解析中…' : '解析引用' }}
        </button>
        <span class="pill">@agent + /action</span>
      </div>

      <p v-if="error" class="muted">{{ error }}</p>
    </article>

    <article class="panel full">
      <p class="eyebrow">已解析的引用</p>
      <div class="reference-list">
        <div
          v-for="reference in references"
          :key="`${reference.kind}-${reference.name}`"
          class="reference-chip"
          :data-exists="reference.exists"
        >
          <strong>{{ reference.raw }}</strong>
          <small>{{ reference.kind }} · {{ reference.exists ? '已注册' : '未知' }}</small>
        </div>

        <p v-if="references.length === 0" class="muted">
          从对话中提取出来的显式引用会显示在这里。
        </p>
      </div>
    </article>
  </section>
</template>
