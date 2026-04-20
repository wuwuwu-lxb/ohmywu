<script setup lang="ts">
import { ref } from 'vue'

const audits = ref([
  { id: '1', actor: 'user', summary: 'GET /api/system/overview', created_at: '2026-04-19T10:00:00Z', risk_level: 'read_only' },
  { id: '2', actor: 'user', summary: 'GET /api/processes', created_at: '2026-04-19T10:01:00Z', risk_level: 'read_only' },
  { id: '3', actor: 'user', summary: 'GET /api/services', created_at: '2026-04-19T10:01:05Z', risk_level: 'read_only' },
  { id: '4', actor: 'user', summary: 'POST /api/storage/scans', created_at: '2026-04-19T10:02:00Z', risk_level: 'read_only' },
])
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <p class="eyebrow">审计记录</p>
      <h3>行为记录</h3>
      <p class="muted">所有操作的行为记录与风险级别。M3 完成后每次写操作都会生成真实审计条目。</p>
    </article>
    <article class="panel full">
      <div class="audit-list">
        <div v-for="a in audits" :key="a.id" class="audit-row">
          <span class="audit-actor">{{ a.actor }}</span>
          <span class="audit-summary">{{ a.summary }}</span>
          <span class="audit-ts">{{ a.created_at }}</span>
          <span class="audit-risk" :class="`risk-${a.risk_level}`">{{ a.risk_level }}</span>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.audit-list { font-size: 13px; }
.audit-row { display: grid; grid-template-columns: 60px 1fr 140px 80px; gap: 10px; padding: 8px 0; border-bottom: 1px solid #f0ebe3; align-items: center; }
.audit-actor { font-weight: 600; color: #1a1612; }
.audit-summary { font-family: monospace; font-size: 12px; color: #444; }
.audit-ts { font-size: 11px; color: #aaa; font-family: monospace; }
.audit-risk { font-size: 10px; padding: 2px 6px; border-radius: 3px; text-align: center; }
.risk-read_only { background: #e8f5e9; color: #2e7d32; }
.risk-controlled_write { background: #fff8e1; color: #f57f17; }
.risk-high_risk { background: #ffebee; color: #c62828; }
</style>
