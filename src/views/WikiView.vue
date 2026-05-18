<script setup lang="ts">
import { ref, onMounted, nextTick } from "vue"
import { useWikiStore, type GraphData } from "../stores/wiki"

const store = useWikiStore()
const searchInput = ref("")
const graphCanvas = ref<HTMLCanvasElement>()

let searchTimer: ReturnType<typeof setTimeout> | null = null

onMounted(async () => {
  await store.loadNotes()
})

const onSearchInput = () => {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.search(searchInput.value)
  }, 200)
}

const openNote = (slug: string) => {
  store.selectNote(slug)
}

// ── Markdown rendering ─────────────────────────────────────────────

function renderMd(body: string, links: string[]): string {
  const slugSet = new Set(links)
  let html = ""
  let inCode = false
  let inList = false

  const lines = body.split("\n")
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // code fence toggle
    if (line.trimStart().startsWith("```")) {
      if (!inCode) { html += "<pre><code>"; inCode = true }
      else { html += "</code></pre>"; inCode = false }
      continue
    }
    if (inCode) {
      html += escapeHtml(line) + "\n"
      continue
    }

    // blank line ends list
    if (line.trim() === "") {
      if (inList) { html += "</ul>"; inList = false }
      html += "<br/>"
      continue
    }

    // headers
    if (line.startsWith("### ")) {
      html += `<h3>${renderInline(line.slice(4), slugSet)}</h3>`
      continue
    }
    if (line.startsWith("## ")) {
      html += `<h2>${renderInline(line.slice(3), slugSet)}</h2>`
      continue
    }
    if (line.startsWith("# ")) {
      html += `<h1>${renderInline(line.slice(2), slugSet)}</h1>`
      continue
    }

    // unordered list
    if (line.match(/^[\-\*]\s/)) {
      if (!inList) { html += "<ul>"; inList = true }
      html += `<li>${renderInline(line.slice(2).trim(), slugSet)}</li>`
      continue
    }
    if (/^\d+\.\s/.test(line)) {
      if (!inList) { html += "<ol>"; inList = true }
      html += `<li>${renderInline(line.replace(/^\d+\.\s/, ""), slugSet)}</li>`
      continue
    }

    // regular paragraph
    if (inList) { html += "</ul>"; inList = false }
    html += `<p>${renderInline(line, slugSet)}</p>`
  }
  if (inList) html += "</ul>"

  return html
}

function renderInline(text: string, knownSlugs: Set<string>): string {
  let out = escapeHtml(text)
  // bold
  out = out.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
  // italic
  out = out.replace(/\*(.+?)\*/g, "<em>$1</em>")
  // inline code
  out = out.replace(/`(.+?)`/g, "<code>$1</code>")
  // [[wiki links]]
  out = out.replace(/\[\[([^\]]+)\]\]/g, (_, inner: string) => {
    const [slug, display] = inner.includes("|") ? inner.split("|") : [inner, inner]
    const s = slug.trim()
    const label = escapeHtml(display || s)
    if (knownSlugs.has(s)) {
      return `<a class="wiki-link" href="#" data-slug="${escapeHtml(s)}">${label}</a>`
    }
    return `<span class="wiki-link-broken">${label}</span>`
  })
  return out
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

// delegate [[link]] clicks
function onContentClick(e: Event) {
  const el = (e.target as HTMLElement).closest(".wiki-link") as HTMLElement | null
  if (el?.dataset.slug) {
    e.preventDefault()
    openNote(el.dataset.slug)
  }
}

// ── Graph ──────────────────────────────────────────────────────────

async function renderGraph() {
  await store.loadGraph()
  await nextTick()
  if (!graphCanvas.value || !store.graph) return
  drawGraph(graphCanvas.value, store.graph, (slug: string) => openNote(slug))
}

// Force-directed graph using Canvas
function drawGraph(canvas: HTMLCanvasElement, data: GraphData, onClick: (s: string) => void) {
  const dpr = window.devicePixelRatio || 1
  const rect = canvas.getBoundingClientRect()
  canvas.width = rect.width * dpr
  canvas.height = rect.height * dpr
  const ctx = canvas.getContext("2d")!
  ctx.scale(dpr, dpr)

  const W = rect.width
  const H = rect.height
  const cx = W / 2
  const cy = H / 2

  interface SimNode {
    id: string
    label: string
    linkCount: number
    x: number
    y: number
    vx: number
    vy: number
  }

  const nodes: SimNode[] = data.nodes.map(n => ({
    id: n.id,
    label: n.label,
    linkCount: n.link_count,
    x: cx + (Math.random() - 0.5) * W * 0.5,
    y: cy + (Math.random() - 0.5) * H * 0.5,
    vx: 0, vy: 0,
  }))

  const nodeMap = new Map(nodes.map(n => [n.id, n]))
  const edges = data.edges.filter(e => nodeMap.has(e.source) && nodeMap.has(e.target))

  const collide = 60
  const linkDist = 80
  const centerGravity = 0.005
  const alpha = 0.3
  const iterations = 200

  for (let iter = 0; iter < iterations; iter++) {
    // repulsion
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const dx = nodes[j].x - nodes[i].x
        const dy = nodes[j].y - nodes[i].y
        const d = Math.max(1, Math.sqrt(dx * dx + dy * dy))
        const f = collide * collide / d * alpha
        const fx = (dx / d) * f
        const fy = (dy / d) * f
        nodes[i].vx -= fx; nodes[i].vy -= fy
        nodes[j].vx += fx; nodes[j].vy += fy
      }
    }
    // attraction
    for (const e of edges) {
      const s = nodeMap.get(e.source)!
      const t = nodeMap.get(e.target)!
      const dx = t.x - s.x
      const dy = t.y - s.y
      const d = Math.max(1, Math.sqrt(dx * dx + dy * dy))
      const f = (d - linkDist) / d * alpha * 0.5
      const fx = dx * f; const fy = dy * f
      s.vx += fx; s.vy += fy
      t.vx -= fx; t.vy -= fy
    }
    // center gravity + apply
    for (const n of nodes) {
      n.vx += (cx - n.x) * centerGravity
      n.vy += (cy - n.y) * centerGravity
      n.x += n.vx; n.y += n.vy
      n.vx *= 0.6; n.vy *= 0.6
      // clamp
      const pad = 30
      n.x = Math.max(pad, Math.min(W - pad, n.x))
      n.y = Math.max(pad, Math.min(H - pad, n.y))
    }
  }

  // draw
  ctx.clearRect(0, 0, W, H)
  // edges
  ctx.strokeStyle = "var(--border-color, rgba(255,255,255,0.15))"
  ctx.lineWidth = 1
  for (const e of edges) {
    const s = nodeMap.get(e.source)!, t = nodeMap.get(e.target)!
    ctx.beginPath()
    ctx.moveTo(s.x, s.y)
    ctx.lineTo(t.x, t.y)
    ctx.stroke()
  }
  // nodes
  for (const n of nodes) {
    const r = Math.min(20, 6 + n.linkCount * 1.5)
    ctx.beginPath()
    ctx.arc(n.x, n.y, r, 0, Math.PI * 2)
    ctx.fillStyle = "var(--accent, #3b82f6)"
    ctx.globalAlpha = 0.8
    ctx.fill()
    ctx.globalAlpha = 1
    // label
    ctx.fillStyle = "var(--text-primary, #fff)"
    ctx.font = `${10 + Math.min(r / 4, 4)}px var(--font-mono, monospace)`
    ctx.textAlign = "center"
    ctx.fillText(n.label.slice(0, 12), n.x, n.y + r + 14)
  }

  // click handler
  canvas.onclick = (e: MouseEvent) => {
    const mx = e.offsetX, my = e.offsetY
    for (const n of nodes) {
      const r = Math.min(20, 6 + n.linkCount * 1.5)
      const dx = mx - n.x, dy = my - n.y
      if (dx * dx + dy * dy < r * r + 60) {
        onClick(n.id)
        return
      }
    }
  }
}

// ── Tags filter ────────────────────────────────────────────────────

const uniqueTags = () => {
  const s = new Set<string>()
  for (const n of store.notes) {
    for (const t of n.tags) s.add(t)
  }
  return [...s].sort()
}
</script>

<template>
  <div class="wiki-view">
    <!-- header -->
    <div class="wiki-header">
      <h1 class="wiki-title">知识库</h1>
      <div class="wiki-nav">
        <button :class="{ active: store.viewMode === 'list' }" @click="store.viewMode = 'list'">浏览</button>
        <button :class="{ active: store.viewMode === 'graph' }" @click="renderGraph">图谱</button>
      </div>
    </div>

    <div class="wiki-body">
      <!-- List View -->
      <div v-if="store.viewMode === 'list'" class="list-panel">
        <div class="search-box">
          <input
            v-model="searchInput"
            type="text"
            placeholder="搜索笔记..."
            @input="onSearchInput"
            class="search-input"
          />
          <span v-if="store.loading" class="search-spinner" />
        </div>

        <div v-if="store.searchQuery && store.searchResults.length" class="note-list">
          <h3 class="section-title">搜索结果</h3>
          <button
            v-for="n in store.searchResults"
            :key="n.slug"
            class="note-card"
            @click="openNote(n.slug)"
          >
            <div class="note-title">{{ n.title }}</div>
            <div class="note-meta">
              <span v-for="t in n.tags" :key="t" class="tag">{{ t }}</span>
              <span class="note-date">{{ n.updated.slice(0, 10) }}</span>
            </div>
          </button>
        </div>

        <div v-if="!searchInput" class="note-list">
          <div v-if="uniqueTags().length" class="tag-bar">
            <span v-for="t in uniqueTags()" :key="t" class="tag clickable" @click="searchInput = t; store.search(t)">
              {{ t }}
            </span>
          </div>
          <h3 class="section-title">最近更新</h3>
          <button
            v-for="n in store.notes"
            :key="n.slug"
            class="note-card"
            @click="openNote(n.slug)"
          >
            <div class="note-title">{{ n.title }}</div>
            <div class="note-meta">
              <span v-for="t in n.tags" :key="t" class="tag">{{ t }}</span>
              <span class="note-date">{{ n.updated.slice(0, 10) }}</span>
            </div>
          </button>
          <p v-if="!store.notes.length && !store.loading" class="empty-hint">
            还没有笔记。在对话中让 agent 帮你记录知识，或者 agent 会自动发现知识点。
          </p>
        </div>
      </div>

      <!-- Note Detail View -->
      <div v-else-if="store.viewMode === 'note' && store.current" class="note-detail">
        <button class="back-btn" @click="store.goBack">← 返回</button>

        <article class="note-article">
          <header class="article-header">
            <h1>{{ store.current.title }}</h1>
            <div class="article-meta">
              <span v-for="t in store.current.tags" :key="t" class="tag">{{ t }}</span>
              <span class="note-date">更新于 {{ store.current.updated.slice(0, 10) }}</span>
            </div>
          </header>
          <div
            class="article-body"
            v-html="renderMd(store.current.body, store.current.body.match(/\[\[([^\]|]+)(?:\|[^\]]+)?\]\]/g)?.map(m => m.slice(2, m.indexOf(']') > -1 ? m.indexOf(']') : m.length - 2).split('|')[0].trim()) || [])"
            @click="onContentClick"
          />
        </article>

        <!-- Backlinks -->
        <aside v-if="store.notes.filter(n => n.links_to?.includes(store.current!.slug)).length" class="backlinks">
          <h3>反向链接</h3>
          <button
            v-for="n in store.notes.filter(nn => nn.links_to?.includes(store.current!.slug))"
            :key="n.slug"
            class="note-card small"
            @click="openNote(n.slug)"
          >
            {{ n.title }}
          </button>
        </aside>
      </div>

      <!-- Graph View -->
      <div v-else-if="store.viewMode === 'graph'" class="graph-panel">
        <canvas ref="graphCanvas" class="graph-canvas" />
        <p class="graph-hint">点击节点打开笔记</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wiki-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* header */
.wiki-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 22px 14px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  background: rgba(8, 10, 14, 0.18);
}
.wiki-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}
.wiki-nav {
  display: flex;
  gap: 8px;
}
.wiki-nav button {
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 13px;
  font-family: var(--font);
  cursor: pointer;
  transition: all var(--duration-fast);
}
.wiki-nav button:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--text-primary); border-color: rgba(var(--accent-rgb), 0.18); }
.wiki-nav button.active { background: rgba(var(--accent-rgb), 0.14); color: var(--text-primary); border-color: rgba(var(--accent-rgb), 0.22); }

/* body */
.wiki-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 22px 24px;
}

/* search */
.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}
.search-input {
  flex: 1;
  padding: 11px 14px;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: var(--surface-1);
  color: var(--text-primary);
  font-size: 14px;
  font-family: var(--font);
  outline: none;
  transition: border-color var(--duration-fast), box-shadow var(--duration-fast), background var(--duration-fast);
}
.search-input:focus { border-color: rgba(var(--accent-rgb), 0.22); box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08); background: var(--surface-2); }
.search-input::placeholder { color: var(--text-disabled); }
.search-spinner {
  width: 16px; height: 16px;
  border: 2px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* tags */
.tag-bar { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 16px; }
.tag {
  padding: 2px 10px;
  border-radius: 10px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 12px;
  font-weight: 500;
}
.tag.clickable { cursor: pointer; transition: all var(--duration-fast); }
.tag.clickable:hover { background: var(--accent); color: var(--text-on-accent); }

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 16px 0 8px;
}

/* note cards */
.note-list { display: flex; flex-direction: column; gap: 2px; }
.note-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--surface-1);
  text-align: left;
  cursor: pointer;
  transition: all var(--duration-fast);
  width: 100%;
  box-shadow: var(--shadow-surface);
}
.note-card:hover { background: rgba(var(--accent-rgb), 0.06);
  border-color: rgba(var(--accent-rgb), 0.18);
  transform: translateY(-1px);
}
.note-card.small { padding: 6px 10px; }
.note-title { font-size: 14px; font-weight: 600; color: var(--text-primary); }
.note-meta { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.note-date { font-size: 12px; color: var(--text-tertiary); }

/* note detail */
.note-detail { max-width: 760px; margin: 0 auto; }
.back-btn {
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--surface-1);
  color: var(--text-secondary);
  font-size: 13px;
  font-family: var(--font);
  cursor: pointer;
  margin-bottom: 12px;
  transition: all var(--duration-fast);
}
.back-btn:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--text-primary); border-color: rgba(var(--accent-rgb), 0.18); }

.note-article {
  padding: 24px 24px 26px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: rgba(9, 11, 15, 0.62);
  box-shadow: var(--shadow-float);
}

.article-header { margin-bottom: 20px; }
.article-header h1 { font-size: 22px; font-weight: 700; color: var(--text-primary); margin: 0 0 8px; }
.article-meta { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }

/* rendered markdown */
.article-body {
  font-size: 15px;
  line-height: 1.8;
  color: var(--text-primary);
}

.backlinks {
  margin-top: 16px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: var(--surface-1);
}

.backlinks h3 {
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--text-secondary);
}

.graph-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  padding: 18px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: rgba(9, 11, 15, 0.62);
  box-shadow: var(--shadow-float);
}
.article-body :deep(h1) { font-size: 20px; margin: 24px 0 12px; }
.article-body :deep(h2) { font-size: 17px; margin: 20px 0 10px; color: var(--text-primary); }
.article-body :deep(h3) { font-size: 15px; margin: 16px 0 8px; color: var(--text-secondary); }
.article-body :deep(p) { margin: 0 0 12px; }
.article-body :deep(ul), .article-body :deep(ol) { margin: 0 0 12px; padding-left: 20px; }
.article-body :deep(li) { margin-bottom: 4px; }
.article-body :deep(strong) { font-weight: 600; color: var(--text-primary); }
.article-body :deep(em) { font-style: italic; }
.article-body :deep(code) {
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--surface-2);
  font-family: var(--font-mono, monospace);
  font-size: 13px;
}
.article-body :deep(pre) {
  padding: 14px;
  border-radius: var(--radius-md);
  background: var(--surface-2);
  overflow-x: auto;
  margin: 0 0 16px;
  font-size: 13px;
  line-height: 1.5;
}
.article-body :deep(pre code) { background: none; padding: 0; }
.article-body :deep(.wiki-link) {
  color: var(--accent);
  text-decoration: none;
  border-bottom: 1px dashed var(--accent);
  cursor: pointer;
}
.article-body :deep(.wiki-link:hover) { border-bottom-style: solid; }
.article-body :deep(.wiki-link-broken) {
  color: var(--text-tertiary);
  border-bottom: 1px dashed var(--text-tertiary);
}

.backlinks .note-card { display: inline-flex; margin-right: 4px; }
.graph-panel { position: relative; min-height: 400px; }
.graph-canvas { width: 100%; height: 100%; cursor: pointer; flex: 1; }
.graph-hint { text-align: center; font-size: 12px; color: var(--text-tertiary); }

.empty-hint { color: var(--text-tertiary); font-size: 14px; padding: 32px 0; text-align: center; }
</style>
