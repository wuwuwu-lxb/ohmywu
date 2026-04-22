<script setup lang="ts">
import { computed, inject, onMounted, ref } from 'vue'
import { executeCleanup, fetchCleanupTree, type CleanupNode, type CleanupTreeResult } from '@/lib/api'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

type CleanupGroupKey = 'safe' | 'caution' | 'system' | 'manual'

const toast = inject<(message: string, type: 'success' | 'error' | 'info') => void>('toast')

const treeData = ref<CleanupNode | null>(null)
const loading = ref(false)
const selectedPaths = ref<Set<string>>(new Set())
const showConfirm = ref(false)
const showSystemItems = ref(false)
const showManualItems = ref(false)

function flattenNodes(node: CleanupNode | null | undefined): CleanupNode[] {
  if (!node) return []
  const nodes: CleanupNode[] = []
  for (const child of node.children || []) {
    nodes.push(child)
    nodes.push(...flattenNodes(child))
  }
  return nodes
}

const cleanupItems = computed(() => {
  const seen = new Map<string, CleanupNode>()
  for (const node of flattenNodes(treeData.value)) {
    if (!node.path || seen.has(node.path)) continue
    seen.set(node.path, node)
  }
  return [...seen.values()].sort((a, b) => b.size_bytes - a.size_bytes)
})

function groupFor(node: CleanupNode): CleanupGroupKey | null {
  if (node.level === 'folder' || node.level === 'root') return null
  if (node.level === 'L3') return 'system'
  if (node.level === 'L2') return 'caution'
  if (node.level === 'L1') return 'safe'
  if (node.level === 'unknown') return 'manual'
  return null
}

const safeItems = computed(() => cleanupItems.value.filter((node) => groupFor(node) === 'safe'))
const cautionItems = computed(() => cleanupItems.value.filter((node) => groupFor(node) === 'caution'))
const systemItems = computed(() => cleanupItems.value.filter((node) => groupFor(node) === 'system'))
const manualItems = computed(() => cleanupItems.value.filter((node) => groupFor(node) === 'manual'))

const selectedList = computed(() => cleanupItems.value.filter((node) => selectedPaths.value.has(node.path)))
const selectedSize = computed(() => selectedList.value.reduce((sum, node) => sum + node.size_bytes, 0))
const safeSize = computed(() => safeItems.value.reduce((sum, node) => sum + node.size_bytes, 0))
const cautionSize = computed(() => cautionItems.value.reduce((sum, node) => sum + node.size_bytes, 0))
const manualSize = computed(() => manualItems.value.reduce((sum, node) => sum + node.size_bytes, 0))
const selectedCount = computed(() => selectedList.value.length)

function formatSize(size: number) {
  const gb = size / 1073741824
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  const mb = size / 1048576
  if (mb >= 1) return `${mb.toFixed(1)} MB`
  const kb = size / 1024
  return `${kb.toFixed(0)} KB`
}

const safeDisplay = computed(() => formatSize(safeSize.value))
const cautionDisplay = computed(() => formatSize(cautionSize.value))
const manualDisplay = computed(() => formatSize(manualSize.value))
const selectedDisplay = computed(() => formatSize(selectedSize.value))

function reasonLabel(node: CleanupNode) {
  if (node.reason?.trim()) return node.reason
  const group = groupFor(node)
  if (group === 'safe') return '删除后不会影响系统和个人文件，需要时会自动重新生成。'
  if (group === 'caution') return '可能影响缓存、工具链或某些应用状态，建议确认后清理。'
  if (group === 'system') return '系统级目录，需要更高权限，不属于日常快速清理。'
  if (group === 'manual') return '系统暂时无法确认是否可删，默认不放进快捷清理。'
  return '这是目录分组节点，用于归类展示，不直接作为清理项。'
}

function groupTitle(group: CleanupGroupKey) {
  if (group === 'safe') return '可放心清理'
  if (group === 'caution') return '建议确认后清理'
  if (group === 'manual') return '默认不纳入快捷清理'
  return '系统级项目'
}

function groupDescription(group: CleanupGroupKey) {
  if (group === 'safe') return '不会影响系统和个人文件，主要是可再生缓存与残留文件。'
  if (group === 'caution') return '可能影响应用缓存、开发环境或重新下载成本，请按需选择。'
  if (group === 'manual') return '常见于个人资料、项目目录或无法可靠判定的内容，只做提示，不建议一键清理。'
  return '仅展示给你参考，默认不放进快捷清理主路径。'
}

async function loadTree() {
  loading.value = true
  try {
    const result: CleanupTreeResult = await fetchCleanupTree()
    treeData.value = result.root
    selectedPaths.value = new Set()
  } catch (e) {
    toast?.(`扫描失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  } finally {
    loading.value = false
  }
}

function toggleSelect(path: string) {
  const next = new Set(selectedPaths.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  selectedPaths.value = next
}

function selectSafeItems() {
  selectedPaths.value = new Set(safeItems.value.map((node) => node.path))
  toast?.(`已选中 ${safeItems.value.length} 个可放心清理项目`, 'info')
}

function clearSelection() {
  selectedPaths.value = new Set()
}

function startCleanup() {
  if (!selectedCount.value) {
    toast?.('请先选择要清理的项目', 'info')
    return
  }
  showConfirm.value = true
}

async function doCleanup() {
  showConfirm.value = false
  const paths = selectedList.value.map((node) => node.path)
  if (!paths.length) return

  try {
    const res = await executeCleanup(paths)
    toast?.(`已释放 ${formatSize(res.freed_bytes)}`, 'success')
    await loadTree()
  } catch (e) {
    toast?.(`清理失败: ${e instanceof Error ? e.message : '未知错误'}`, 'error')
  }
}

onMounted(async () => {
  await loadTree()
})
</script>

<template>
  <section class="page-grid storage-page">
    <article class="panel full storage-hero">
      <div>
        <p class="eyebrow">storage cleanup</p>
        <h3>存储清理</h3>
        <p class="muted">
          默认优先帮你处理可放心清理的项目。系统已经先做过风险判断，你只需要确认是否执行。
        </p>
      </div>
      <div class="hero-actions">
        <button class="btn-secondary" :disabled="loading" @click="loadTree">
          {{ loading ? '扫描中…' : '重新扫描' }}
        </button>
      </div>
    </article>

    <div v-if="loading && !treeData" class="panel full loading-state">
      <p class="muted">正在扫描可清理项目…</p>
    </div>

    <template v-else>
      <article class="panel full summary-strip">
        <div class="summary-card emphasis">
          <span class="summary-label">可放心清理</span>
          <strong>{{ safeDisplay }}</strong>
          <p class="muted">不会影响系统和个人文件</p>
        </div>
        <div class="summary-card">
          <span class="summary-label">建议确认后清理</span>
          <strong>{{ cautionDisplay }}</strong>
          <p class="muted">可能影响缓存、工具链或下载速度</p>
        </div>
        <div class="summary-card">
          <span class="summary-label">默认不纳入快捷清理</span>
          <strong>{{ manualDisplay }}</strong>
          <p class="muted">个人资料和未分类目录只展示不默认操作</p>
        </div>
        <div class="summary-card">
          <span class="summary-label">当前已选</span>
          <strong>{{ selectedDisplay }}</strong>
          <p class="muted">{{ selectedCount }} 个项目</p>
        </div>
      </article>

      <article class="panel full primary-action-panel">
        <div>
          <p class="eyebrow">quick cleanup</p>
          <h3>一键安全清理</h3>
          <p class="muted">优先走最短路径：先选中所有可放心清理项，再由你一键确认执行。</p>
        </div>
        <div class="primary-actions">
          <button class="btn-primary" :disabled="safeItems.length === 0" @click="selectSafeItems">
            选中全部可放心清理项
          </button>
          <button class="btn-secondary" :disabled="selectedCount === 0" @click="clearSelection">清空选择</button>
          <button class="btn-primary strong" :disabled="selectedCount === 0" @click="startCleanup">清理所选项目</button>
        </div>
      </article>

      <article class="panel full cleanup-group">
        <div class="group-head">
          <div>
            <p class="eyebrow">safe items</p>
            <h3>{{ groupTitle('safe') }}</h3>
            <p class="muted">{{ groupDescription('safe') }}</p>
          </div>
          <span class="group-total">{{ safeItems.length }} 项 · {{ safeDisplay }}</span>
        </div>

        <div v-if="safeItems.length === 0" class="empty-state muted">当前没有可放心清理项</div>
        <div v-else class="item-list">
          <label v-for="node in safeItems" :key="node.path" class="item-row item-safe">
            <input :checked="selectedPaths.has(node.path)" type="checkbox" @change="toggleSelect(node.path)" />
            <div class="item-main">
              <div class="item-title-row">
                <strong>{{ node.name }}</strong>
                <span class="item-size">{{ node.human_size }}</span>
              </div>
              <p class="muted item-reason">{{ reasonLabel(node) }}</p>
              <p class="item-path">{{ node.path }}</p>
            </div>
          </label>
        </div>
      </article>

      <article class="panel full cleanup-group">
        <div class="group-head">
          <div>
            <p class="eyebrow">caution items</p>
            <h3>{{ groupTitle('caution') }}</h3>
            <p class="muted">{{ groupDescription('caution') }}</p>
          </div>
          <span class="group-total">{{ cautionItems.length }} 项 · {{ cautionDisplay }}</span>
        </div>

        <div v-if="cautionItems.length === 0" class="empty-state muted">当前没有需要额外确认的项目</div>
        <div v-else class="item-list">
          <label v-for="node in cautionItems" :key="node.path" class="item-row item-caution">
            <input :checked="selectedPaths.has(node.path)" type="checkbox" @change="toggleSelect(node.path)" />
            <div class="item-main">
              <div class="item-title-row">
                <strong>{{ node.name }}</strong>
                <span class="item-size">{{ node.human_size }}</span>
              </div>
              <p class="muted item-reason">{{ reasonLabel(node) }}</p>
              <p class="item-path">{{ node.path }}</p>
            </div>
          </label>
        </div>
      </article>

      <article class="panel full cleanup-group">
        <button class="system-toggle" @click="showManualItems = !showManualItems">
          <span>
            <p class="eyebrow">manual review</p>
            <h3>{{ groupTitle('manual') }}</h3>
            <p class="muted">{{ groupDescription('manual') }}</p>
          </span>
          <span class="group-total">{{ manualItems.length }} 项 · {{ manualDisplay }}</span>
        </button>

        <div v-if="showManualItems" class="item-list manual-list">
          <div v-if="manualItems.length === 0" class="empty-state muted">当前没有需要人工确认的目录</div>
          <div v-for="node in manualItems" :key="node.path" class="item-row item-manual readonly">
            <div class="item-main">
              <div class="item-title-row">
                <strong>{{ node.name }}</strong>
                <span class="item-size">{{ node.human_size }}</span>
              </div>
              <p class="muted item-reason">{{ reasonLabel(node) }}</p>
              <p class="item-path">{{ node.path }}</p>
            </div>
          </div>
        </div>
      </article>

      <article class="panel full cleanup-group">
        <button class="system-toggle" @click="showSystemItems = !showSystemItems">
          <span>
            <p class="eyebrow">system items</p>
            <h3>{{ groupTitle('system') }}</h3>
            <p class="muted">{{ groupDescription('system') }}</p>
          </span>
          <span class="group-total">{{ systemItems.length }} 项</span>
        </button>

        <div v-if="showSystemItems" class="item-list system-list">
          <div v-if="systemItems.length === 0" class="empty-state muted">当前没有系统级项目</div>
          <div v-for="node in systemItems" :key="node.path" class="item-row item-system readonly">
            <div class="item-main">
              <div class="item-title-row">
                <strong>{{ node.name }}</strong>
                <span class="item-size">{{ node.human_size }}</span>
              </div>
              <p class="muted item-reason">{{ reasonLabel(node) }}</p>
              <p class="item-path">{{ node.path }}</p>
            </div>
          </div>
        </div>
      </article>
    </template>

    <ConfirmDialog
      v-if="showConfirm"
      title="确认清理"
      :message="`确定要清理 ${selectedCount} 个项目吗？预计释放 ${selectedDisplay} 空间。`"
      confirmText="确认清理"
      cancelText="取消"
      @confirm="doCleanup"
      @cancel="showConfirm = false"
    />
  </section>
</template>

<style scoped>
.storage-page,
.storage-hero,
.primary-action-panel,
.cleanup-group {
  display: grid;
  gap: 16px;
}

.storage-hero {
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: start;
}

.hero-actions,
.primary-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.summary-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.summary-card {
  padding: 18px;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.45);
  border: 1px solid var(--line);
  display: grid;
  gap: 6px;
}

.summary-card.emphasis {
  background: rgba(200, 90, 46, 0.1);
  border-color: rgba(200, 90, 46, 0.26);
}

.summary-card strong {
  font-size: 1.7rem;
}

.summary-label,
.group-total,
.item-path {
  color: var(--muted);
  font-size: 0.84rem;
}

.btn-primary,
.btn-secondary {
  border: 0;
  border-radius: 14px;
  padding: 12px 18px;
  font-weight: 700;
}

.btn-primary.strong {
  background: #8c3317;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.65);
  color: var(--ink);
  border: 1px solid var(--line);
}

.btn-secondary:disabled,
.btn-primary:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.group-head,
.system-toggle {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}

.system-toggle {
  border: 0;
  background: transparent;
  padding: 0;
  text-align: left;
  cursor: pointer;
}

.item-list {
  display: grid;
  gap: 10px;
}

.item-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 14px;
  padding: 16px;
  border-radius: 16px;
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.52);
}

.item-row.readonly {
  grid-template-columns: minmax(0, 1fr);
}

.item-safe {
  border-color: rgba(76, 175, 80, 0.22);
}

.item-caution {
  border-color: rgba(255, 152, 0, 0.22);
}

.item-manual {
  border-color: rgba(96, 125, 139, 0.24);
}

.item-system {
  border-color: rgba(244, 67, 54, 0.22);
}

.item-row input {
  margin-top: 4px;
  width: 18px;
  height: 18px;
}

.item-main {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.item-title-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
}

.item-size {
  font-weight: 700;
  color: var(--ink);
  white-space: nowrap;
}

.item-reason {
  margin: 0;
}

.item-path {
  overflow-wrap: anywhere;
}

.loading-state,
.empty-state {
  padding: 20px 0;
}

@media (max-width: 960px) {
  .storage-hero,
  .summary-strip {
    grid-template-columns: 1fr;
  }

  .item-title-row,
  .group-head,
  .system-toggle {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
