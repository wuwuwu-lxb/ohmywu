<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { fetchAudits, type AuditEvent } from '@/lib/api'

const audits = ref<AuditEvent[]>([])
const error = ref<string | null>(null)
let timer: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await load()
  timer = setInterval(load, 10000)
})

onUnmounted(() => { if (timer) clearInterval(timer) })

async function load() {
  try {
    audits.value = await fetchAudits(200)
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
  }
}

function riskLabel(r: string) {
  const map: Record<string, string> = { read_only: '只读', controlled_write: '受控写', high_risk: '高风险' }
  return map[r] ?? r
}

function formatTime(t: string) {
  return new Date(t).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' })
}
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">审计记录</p>
      <h3>行为记录</h3>
      <p class="muted">所有操作的行为记录与风险级别，每 10 秒自动刷新。</p>
    </article>
    <article class="panel full">
      <p v-if="error" class="muted">{{ error }}</p>
      <p v-if="audits.length === 0 && !error" class="muted">暂无审计记录</p>
      <div class="audit-list" v-if="audits.length > 0">
        <div class="table-head">
          <span>时间</span><span>操作者</span><span>动作</span><span>目标</span><span>风险</span><span>结果</span>
        </div>
        <div v-for="a in audits" :key="a.id" class="audit-row">
          <span class="audit-ts">{{ formatTime(a.timestamp) }}</span>
          <span class="audit-actor">{{ a.actor }}</span>
          <span class="audit-action">{{ a.action }}</span>
          <span class="audit-target" :title="a.target">{{ a.target }}</span>
          <span class="audit-risk" :class="`risk-${a.risk_level}`">{{ riskLabel(a.risk_level) }}</span>
          <span class="audit-result" :class="a.result === 'success' ? 'res-ok' : 'res-fail'">{{ a.result }}</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.audit-list { font-size: 13px; }
.table-head { display: grid; grid-template-columns: 130px 60px 140px 1fr 70px 60px; gap: 8px; padding: 6px 8px; font-size: 11px; text-transform: uppercase; color: #888; border-bottom: 1px solid #eee; font-weight: 600; }
.audit-row { display: grid; grid-template-columns: 130px 60px 140px 1fr 70px 60px; gap: 8px; padding: 7px 8px; border-bottom: 1px solid #f0ebe3; align-items: center; }
.audit-row:hover { background: rgba(201,169,110,0.08); }
.audit-ts { font-size: 11px; color: #aaa; font-family: monospace; }
.audit-actor { font-weight: 600; color: #1a1612; font-size: 12px; }
.audit-action { font-family: monospace; font-size: 12px; color: #444; }
.audit-target { font-size: 12px; color: #666; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.audit-risk { font-size: 10px; padding: 2px 6px; border-radius: 3px; text-align: center; white-space: nowrap; }
.risk-read_only { background: #e8f5e9; color: #2e7d32; }
.risk-controlled_write { background: #fff8e1; color: #f57f17; }
.risk-high_risk { background: #ffebee; color: #c62828; }
.audit-result { font-size: 11px; font-weight: 600; }
.res-ok { color: #2e7d32; }
.res-fail { color: #c62828; }
</style>
