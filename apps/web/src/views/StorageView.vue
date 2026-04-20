<script setup lang="ts">
import { ref, inject, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import * as echarts from 'echarts'
import { fetchCleanupTree, executeCleanup, type CleanupNode, type CleanupTreeResult } from '@/lib/api'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const toast = inject<(message: string, type: 'success' | 'error' | 'info') => void>('toast')

const tab = ref<'main' | 'deep' | 'pkg'>('main')
const treeData = ref<CleanupNode | null>(null)
const loading = ref(false)
const selectedPaths = ref<Set<string>>(new Set())
const expandedPaths = ref<Set<string>>(new Set())
const showConfirm = ref(false)
const sudoCmd = ref<string | null>(null)
const focusPath = ref<string | null>(null)
const treemapRef = ref<HTMLDivElement | null>(null)

let treemapChart: echarts.ECharts | null = null

function levelColor(level: string): string {
  const colors: Record<string, string> = {
    L1: '#2e7d32',
    L2: '#f57c00',
    L3: '#c62828',
    unknown: '#9e9e9e',
    folder: '#607d8b',
    root: '#455a64',
  }
  return colors[level] || '#9e9e9e'
}

function levelFill(level: string): string {
  const colors: Record<string, string> = {
    L1: 'rgba(46,125,50,0.20)',
    L2: 'rgba(245,124,0,0.20)',
    L3: 'rgba(198,40,40,0.20)',
    unknown: 'rgba(158,158,158,0.18)',
    folder: 'rgba(96,125,139,0.18)',
    root: 'rgba(69,90,100,0.18)',
  }
  return colors[level] || 'rgba(158,158,158,0.18)'
}

function findNode(node: CleanupNode, target: string): CleanupNode | null {
  if (node.path === target) return node
  for (const child of node.children || []) {
    const found = findNode(child, target)
    if (found) return found
  }
  return null
}

function findParent(node: CleanupNode, target: string, parent: CleanupNode | null): CleanupNode | null {
  if (node.path === target) return parent
  for (const child of node.children || []) {
    const found = findParent(child, target, node)
    if (found !== null) return found
  }
  return null
}

function collectBreadcrumbs(node: CleanupNode, target: string, path: CleanupNode[] = []): CleanupNode[] | null {
  if (node.path === target) return [...path, node]
  for (const child of node.children || []) {
    const found = collectBreadcrumbs(child, target, [...path, node])
    if (found) return found
  }
  return null
}

const focusNode = computed(() => {
  if (!treeData.value) return null
  if (!focusPath.value) return treeData.value
  return findNode(treeData.value, focusPath.value)
})

const breadcrumbs = computed(() => {
  if (!treeData.value || !focusPath.value) return []
  return collectBreadcrumbs(treeData.value, focusPath.value) || []
})

const treemapSeriesData = computed(() => {
  const root = focusNode.value
  if (!root) return []
  return (root.children || []).map(node => ({
    name: node.name,
    value: Math.max(node.size_bytes, 1),
    path: node.path,
    humanSize: node.human_size,
    level: node.level,
    childCount: (node.children || []).length,
    itemStyle: {
      color: levelFill(node.level),
      borderColor: levelColor(node.level),
      borderWidth: 2,
    },
  }))}
);

const rightList = computed(() => {
  const root = focusNode.value
  if (!root) return []

  const rows: Array<{ node: CleanupNode; depth: number }> = []

  function walk(node: CleanupNode, depth: number) {
    rows.push({ node, depth })
    if (expandedPaths.value.has(node.path)) {
      for (const child of node.children || []) {
        walk(child, depth + 1)
      }
    }
  }

  if (focusPath.value) {
    for (const child of root.children || []) {
      walk(child, 0)
    }
  } else {
    for (const child of root.children || []) {
      walk(child, 0)
    }
  }

  return rows
})

const selectedList = computed(() => {
  const list: CleanupNode[] = []
  function collect(node: CleanupNode | null | undefined) {
    if (!node) return
    if (selectedPaths.value.has(node.path)) list.push(node)
    for (const child of node.children || []) collect(child)
  }
  if (treeData.value) collect(treeData.value)
  return list
})

const selectedSize = computed(() => selectedList.value.reduce((sum, node) => sum + node.size_bytes, 0))
const totalSize = computed(() => treeData.value?.size_bytes || 0)

async function loadTree() {
  loading.value = true
  try {
    const result: CleanupTreeResult = await fetchCleanupTree()
    treeData.value = result.root
    selectedPaths.value = new Set()
    expandedPaths.value = new Set()
    focusPath.value = null
  } catch (e) {
    toast?.(`扫描失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  } finally {
    loading.value = false
    await nextTick()
    renderTreemap()
  }
}

function renderTreemap() {
  if (!treemapRef.value) return

  if (treemapChart && treemapChart.getDom() !== treemapRef.value) {
    treemapChart.dispose()
    treemapChart = null
  }

  if (!treemapChart) {
    treemapChart = echarts.init(treemapRef.value)
  }

  const data = treemapSeriesData.value
  if (!data.length) {
    treemapChart.clear()
    return
  }

  treemapChart.setOption(
    {
      tooltip: {
        formatter: (params: any) => {
          const d = params.data || {}
          return `<b>${d.name || ''}</b><br/>${d.humanSize || ''}<br/><span style="color:${levelColor(d.level || '')}">${d.level || ''}</span>`
        },
      },
      series: [
        {
          type: 'treemap',
          roam: false,
          nodeClick: false,
          breadcrumb: { show: false },
          visibleMin: 10,
          label: {
            show: true,
            formatter: '{b}',
            fontSize: 11,
            overflow: 'truncate',
          },
          upperLabel: {
            show: true,
            height: 18,
            color: '#1a1612',
          },
          itemStyle: {
            borderColor: '#fff',
            gapWidth: 2,
          },
          levels: [
            {
              itemStyle: { borderWidth: 4, gapWidth: 4 },
              upperLabel: { show: false },
            },
            {
              itemStyle: { borderWidth: 2, gapWidth: 2 },
              upperLabel: { show: true, height: 18 },
            },
            {
              itemStyle: { borderWidth: 1, gapWidth: 1 },
            },
          ],
          data,
        },
      ],
    },
    true,
  )

  treemapChart.off('click')
  treemapChart.on('click', (params: any) => {
    const path = params.data?.path
    if (!path || !treeData.value) return
    const node = findNode(treeData.value, path)
    if (!node) return
    focusPath.value = path
  })

  treemapChart.resize()
}

function goBack() {
  if (!treeData.value || !focusPath.value) {
    focusPath.value = null
    return
  }
  const parent = findParent(treeData.value, focusPath.value, null)
  focusPath.value = parent?.path || null
}

function toggleExpand(path: string) {
  const next = new Set(expandedPaths.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  expandedPaths.value = next
}

function toggleSelect(path: string) {
  const next = new Set(selectedPaths.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  selectedPaths.value = next
}

function selectAllL1() {
  const next = new Set<string>()
  function collect(node: CleanupNode | null | undefined) {
    if (!node) return
    if (node.level === 'L1' && node.category !== 'root') next.add(node.path)
    for (const child of node.children || []) collect(child)
  }
  if (treeData.value) collect(treeData.value)
  selectedPaths.value = next
}

function copySudoCmd() {
  if (!sudoCmd.value) return
  navigator.clipboard.writeText(sudoCmd.value)
  toast?.('已复制', 'success')
  sudoCmd.value = null
}

function startCleanup() {
  const paths = Array.from(selectedPaths.value)
  const l3Paths = paths.filter(path => selectedList.value.find(node => node.path === path)?.level === 'L3')

  if (l3Paths.length > 0) {
    sudoCmd.value = l3Paths.map(path => `sudo rm -rf "${path}"`).join('\n')
    return
  }

  showConfirm.value = true
}

async function doCleanup() {
  showConfirm.value = false
  const paths = Array.from(selectedPaths.value).filter(path => selectedList.value.find(node => node.path === path)?.level !== 'L3')
  if (!paths.length) return

  try {
    const res = await executeCleanup(paths)
    toast?.(`已释放 ${(res.freed_bytes / 1048576).toFixed(1)} MB`, 'success')
    await loadTree()
  } catch (e) {
    toast?.(`清理失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  }
}

function handleResize() {
  treemapChart?.resize()
}

onMounted(async () => {
  await loadTree()
  window.addEventListener('resize', handleResize)
})

watch(focusPath, () => {
  nextTick(() => renderTreemap())
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  treemapChart?.dispose()
  treemapChart = null
})
</script>

<template>
  <section class="page-grid">
    <article class="panel full">
      <div class="tabs-header">
        <div class="tabs">
          <button :class="{ active: tab === 'main' }" @click="tab = 'main'">主清理</button>
          <button :class="{ active: tab === 'deep' }" @click="tab = 'deep'">深度清理</button>
          <button :class="{ active: tab === 'pkg' }" @click="tab = 'pkg'">包管理</button>
        </div>
        <div class="header-btns">
          <button class="btn-ghost" :disabled="loading" @click="loadTree">{{ loading ? '扫描中...' : '↻ 重新扫描' }}</button>
        </div>
      </div>
    </article>

    <div v-if="tab === 'main'" class="main-content">
      <div class="split-pane">
        <article class="panel treemap-panel">
          <div class="panel-header">
            <button v-if="focusPath" class="back-btn" @click="goBack">← 返回</button>
            <span class="panel-title">{{ focusNode?.name || '存储概览' }}</span>
          </div>
          <div v-if="breadcrumbs.length > 0" class="breadcrumb">
            {{ breadcrumbs.map(node => node.name).join(' / ') }}
          </div>
          <div v-if="loading" class="loading-placeholder"><p>扫描中...</p></div>
          <div v-else ref="treemapRef" class="treemap-container"></div>
          <div class="treemap-legend">
            <span class="legend-item"><span class="legend-dot" style="background:#2e7d32"></span>L1 安全</span>
            <span class="legend-item"><span class="legend-dot" style="background:#f57c00"></span>L2 谨慎</span>
            <span class="legend-item"><span class="legend-dot" style="background:#c62828"></span>L3 系统</span>
          </div>
        </article>

        <article class="panel list-panel">
          <div class="panel-header">
            <span class="panel-title">{{ focusNode?.name || '根目录' }}</span>
          </div>
          <div class="list-header">
            <span class="col-check"></span>
            <span class="col-name">项目</span>
            <span class="col-size">大小</span>
            <span class="col-level">等级</span>
          </div>
          <div class="list-body">
            <div
              v-for="{ node, depth } in rightList"
              :key="node.path"
              class="list-row"
              :class="{ selected: selectedPaths.has(node.path), l3: node.level === 'L3' }"
              :style="{ paddingLeft: `${depth * 20 + 8}px` }"
            >
              <span class="col-check">
                <input type="checkbox" :checked="selectedPaths.has(node.path)" @change="toggleSelect(node.path)" :disabled="node.level === 'L3'">
                <span v-if="node.children?.length" class="expand-btn" @click="toggleExpand(node.path)">
                  {{ expandedPaths.has(node.path) ? '▼' : '▶' }}
                </span>
              </span>
              <span class="col-name" :title="node.path">{{ node.name }}</span>
              <span class="col-size">{{ node.human_size }}</span>
              <span class="col-level">
                <span class="badge" :style="{ background: levelColor(node.level) }">{{ node.level }}</span>
              </span>
            </div>
          </div>
        </article>
      </div>

      <article v-if="selectedList.length > 0" class="panel full">
        <div class="section-title">已选项 ({{ selectedList.length }}) - {{ (selectedSize / 1048576).toFixed(1) }} MB</div>
        <div class="list-header">
          <span class="col-check"></span>
          <span class="col-name">项目</span>
          <span class="col-size">大小</span>
          <span class="col-level">等级</span>
        </div>
        <div class="list-body">
          <div v-for="node in selectedList" :key="node.path" class="list-row selected">
            <span class="col-check"></span>
            <span class="col-name" :title="node.path">{{ node.path }}</span>
            <span class="col-size">{{ node.human_size }}</span>
            <span class="col-level">
              <span class="badge" :style="{ background: levelColor(node.level) }">{{ node.level }}</span>
            </span>
          </div>
        </div>
      </article>

      <article class="panel full">
        <div class="action-bar">
          <span>总量：{{ (totalSize / 1073741824).toFixed(1) }} GB | 已选：{{ (selectedSize / 1048576).toFixed(1) }} MB</span>
          <div class="action-btns">
            <button class="btn-secondary" @click="selectAllL1">一键选无风险</button>
            <button class="btn-primary" :disabled="selectedSize === 0" @click="startCleanup">清理已选</button>
          </div>
        </div>
      </article>
    </div>

    <article v-if="tab === 'deep'" class="panel full"><p>深度清理 — 待实现</p></article>
    <article v-if="tab === 'pkg'" class="panel full"><p>包管理 — 待实现</p></article>

    <ConfirmDialog
      v-if="showConfirm"
      title="确认清理"
      message="将删除已选项目，此操作不可恢复。"
      confirmText="清理"
      cancelText="取消"
      @confirm="doCleanup"
      @cancel="showConfirm = false"
    />

    <div v-if="sudoCmd" class="sudo-dialog" @click.self="sudoCmd = null">
      <div class="sudo-content">
        <h4>需要 root 权限的项目</h4>
        <p class="muted">以下项目需要手动在终端执行：</p>
        <pre class="cmd-block">{{ sudoCmd }}</pre>
        <div class="sudo-btns">
          <button class="btn-secondary" @click="sudoCmd = null">关闭</button>
          <button class="btn-primary" @click="copySudoCmd">复制命令</button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tabs-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.tabs { display: flex; gap: 8px; }
.tabs button { padding: 8px 16px; border: 1px solid #ddd; border-radius: 20px; cursor: pointer; font-size: 13px; background: #f5f0ea; color: #666; transition: all 0.2s; }
.tabs button.active { background: var(--accent, #c9a96e); color: #fff; border-color: var(--accent, #c9a96e); }
.tabs button:hover:not(.active) { background: #ede7e0; }
.header-btns { display: flex; gap: 8px; }
.btn-ghost { padding: 6px 12px; background: none; border: 1px solid #ddd; border-radius: 6px; cursor: pointer; font-size: 13px; transition: all 0.2s; }
.btn-ghost:hover:not(:disabled) { background: #f5f0ea; }
.btn-ghost:disabled { opacity: 0.5; cursor: not-allowed; }
.main-content { display: flex; flex-direction: column; gap: 16px; }
.split-pane { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; height: 450px; }
.treemap-panel { display: flex; flex-direction: column; overflow: hidden; }
.list-panel { display: flex; flex-direction: column; overflow: hidden; }
.panel-header { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; flex-shrink: 0; }
.back-btn { padding: 4px 8px; background: #f5f0ea; border: 1px solid #ddd; border-radius: 4px; cursor: pointer; font-size: 12px; }
.back-btn:hover { background: #ede7e0; }
.panel-title { font-weight: 600; font-size: 13px; color: #1a1612; }
.breadcrumb { font-size: 11px; color: #888; margin-bottom: 8px; padding: 4px 8px; background: #faf8f5; border-radius: 4px; flex-shrink: 0; }
.treemap-container { flex: 1; min-height: 0; }
.treemap-legend { display: flex; gap: 16px; justify-content: center; padding: 8px; font-size: 11px; color: #666; flex-shrink: 0; }
.legend-item { display: flex; align-items: center; gap: 4px; }
.legend-dot { width: 10px; height: 10px; border-radius: 50%; }
.list-header { display: grid; grid-template-columns: 40px 1fr 100px 80px; gap: 8px; padding: 8px; background: #faf8f5; border-bottom: 1px solid #eee; font-size: 12px; font-weight: 600; color: #888; text-transform: uppercase; flex-shrink: 0; }
.list-body { flex: 1; overflow-y: auto; min-height: 0; }
.list-row { display: grid; grid-template-columns: 40px 1fr 100px 80px; gap: 8px; padding: 8px; align-items: center; border-bottom: 1px solid #f5f0ea; font-size: 13px; transition: background 0.1s; }
.list-row:hover { background: rgba(201, 169, 110, 0.05); }
.list-row.selected { background: rgba(201, 169, 110, 0.1); }
.list-row.l3 { opacity: 0.7; }
.col-check { display: flex; align-items: center; gap: 4px; }
.col-check input { cursor: pointer; }
.expand-btn { font-size: 10px; cursor: pointer; color: #888; user-select: none; width: 16px; text-align: center; }
.col-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.col-size { text-align: right; font-weight: 500; }
.col-level { text-align: center; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 11px; color: #fff; font-weight: 500; }
.loading-placeholder, .empty-placeholder { padding: 40px 16px; text-align: center; }
.loading-placeholder p, .empty-placeholder p { margin: 0; color: #888; }
.section-title { font-size: 13px; font-weight: 600; color: #1a1612; margin-bottom: 8px; }
.action-bar { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-top: 1px solid #eee; }
.action-btns { display: flex; gap: 8px; }
.btn-primary { padding: 8px 20px; background: var(--accent, #c9a96e); color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 13px; transition: all 0.2s; }
.btn-primary:hover:not(:disabled) { background: #b8954f; }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { padding: 8px 20px; background: #f5f0ea; color: #1a1612; border: 1px solid #ddd; border-radius: 8px; cursor: pointer; font-size: 13px; transition: all 0.2s; }
.btn-secondary:hover { background: #ede7e0; }
.muted { color: #888; font-size: 13px; margin: 0; }
.sudo-dialog { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: flex; align-items: center; justify-content: center; z-index: 10000; }
.sudo-content { background: #fff; border-radius: 14px; padding: 20px; max-width: 600px; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2); }
.sudo-content h4 { margin: 0 0 8px 0; font-size: 16px; color: #1a1612; }
.cmd-block { background: #f5f0ea; padding: 12px; border-radius: 8px; font-family: monospace; font-size: 12px; overflow-x: auto; margin: 12px 0; white-space: pre-wrap; word-break: break-all; }
.sudo-btns { display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px; }
</style>
