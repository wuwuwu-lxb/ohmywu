<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue"
import ThemeSelect from "../components/ThemeSelect.vue"
import { useWikiStore, type GraphData } from "../stores/wiki"

const store = useWikiStore()
const searchInput = ref("")
const graphCanvas = ref<HTMLCanvasElement>()
const editorTextarea = ref<HTMLTextAreaElement>()
const folderFilter = ref<"all" | "concepts" | "notes" | "daily" | "profile">("all")
const copyMsg = ref("")
const editorMode = ref<"view" | "create" | "edit">("view")
const deleteConfirm = ref(false)
const linkSuggestionIndex = ref(0)
const editorSelectionStart = ref(0)
const editorSelectionEnd = ref(0)
const graphLegend = ref<
  Array<{
    key: string
    label: string
    count: number
  }>
>([])

const draft = ref({
  currentSlug: "",
  slug: "",
  title: "",
  folder: "notes",
  tagsText: "",
  body: "",
})

let searchTimer: ReturnType<typeof setTimeout> | null = null
let graphResizeHandler: (() => void) | null = null

const FOLDER_LABELS: Record<string, string> = {
  all: "全部",
  concepts: "概念",
  notes: "笔记",
  daily: "每日",
  profile: "画像",
  ghost: "悬空",
}

const FOLDER_COLORS: Record<string, string> = {
  concepts: "#7dd3fc",
  notes: "#fbbf24",
  daily: "#86efac",
  profile: "#f9a8d4",
  ghost: "#94a3b8",
}

onMounted(async () => {
  await store.loadNotes()
  graphResizeHandler = () => {
    if (store.viewMode === "graph" && store.graph) {
      drawGraph(graphCanvas.value!, store.graph, openNote)
    }
  }
  window.addEventListener("resize", graphResizeHandler)
})

onBeforeUnmount(() => {
  if (graphResizeHandler) {
    window.removeEventListener("resize", graphResizeHandler)
  }
})

const onSearchInput = () => {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    store.search(searchInput.value)
  }, 200)
}

const openNote = async (slug: string) => {
  editorMode.value = "view"
  deleteConfirm.value = false
  await store.selectNote(slug)
}

const startCreate = () => {
  editorMode.value = "create"
  deleteConfirm.value = false
  store.current = null
  store.viewMode = "note"
  draft.value = {
    currentSlug: "",
    slug: "",
    title: "",
    folder: folderFilter.value === "all" ? "notes" : folderFilter.value,
    tagsText: "",
    body: "",
  }
}

const startEdit = () => {
  if (!store.current) return
  editorMode.value = "edit"
  deleteConfirm.value = false
  draft.value = {
    currentSlug: store.current.slug,
    slug: store.current.slug,
    title: store.current.title,
    folder: store.current.folder,
    tagsText: store.current.tags.join(", "),
    body: store.current.body,
  }
}

const cancelEditor = () => {
  deleteConfirm.value = false
  if (editorMode.value === "create") {
    store.goBack()
  }
  editorMode.value = "view"
}

const clearSearch = () => {
  searchInput.value = ""
  store.search("")
}

const refreshNotes = async () => {
  await store.loadNotes()
  if (store.viewMode === "graph") {
    await renderGraph()
  }
}

const searchByTag = (tag: string) => {
  searchInput.value = tag
  store.search(tag)
}

const copyCurrentNote = async () => {
  const target = editorMode.value === "view" && store.current
    ? `# ${store.current.title}\n\n${store.current.body}`
    : `# ${draft.value.title}\n\n${draft.value.body}`
  try {
    await navigator.clipboard.writeText(target)
    copyMsg.value = "已复制"
    window.setTimeout(() => {
      copyMsg.value = ""
    }, 1800)
  } catch (error) {
    copyMsg.value = `复制失败：${error}`
  }
}

const folderStats = computed(() =>
  ["concepts", "notes", "daily", "profile"].map((folder) => ({
    key: folder,
    label: FOLDER_LABELS[folder],
    count: store.notes.filter((note) => note.folder === folder).length,
  }))
)

const filteredNotes = computed(() =>
  folderFilter.value === "all"
    ? store.notes
    : store.notes.filter((note) => note.folder === folderFilter.value)
)

const filteredSearchResults = computed(() =>
  folderFilter.value === "all"
    ? store.searchResults
    : store.searchResults.filter((note) => note.folder === folderFilter.value)
)

const resultSummary = computed(() => {
  if (store.searchQuery.trim()) {
    return `搜索结果 ${filteredSearchResults.value.length} 条`
  }
  return `${filteredNotes.value.length} 条笔记`
})

const availableTags = computed(() => {
  const set = new Set<string>()
  for (const note of filteredNotes.value) {
    for (const tag of note.tags) set.add(tag)
  }
  return [...set].sort()
})

const currentBacklinks = computed(() => {
  if (!store.current) return []
  return store.notes.filter((note) => note.links_to?.includes(store.current!.slug))
})

const editorTags = computed(() =>
  draft.value.tagsText
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean)
)

const draftPreviewHtml = computed(() =>
  renderMd(draft.value.body, extractLinkedSlugs(draft.value.body))
)

const canSaveDraft = computed(() => draft.value.title.trim() && draft.value.body.trim())
const folderOptions = computed(() => [
  { label: "概念", value: "concepts" },
  { label: "笔记", value: "notes" },
  { label: "每日", value: "daily" },
  { label: "画像", value: "profile" },
])

const wikiLinkContext = computed(() => {
  const cursor = editorSelectionStart.value
  const before = draft.value.body.slice(0, cursor)
  const lastOpen = before.lastIndexOf("[[")
  const lastClose = before.lastIndexOf("]]")
  if (lastOpen < 0 || lastOpen < lastClose) return null

  const partial = before.slice(lastOpen + 2)
  if (!partial || /[\n\r\[\]]/.test(partial)) return null

  const query = partial.split("|").pop()?.trim() ?? ""
  return {
    start: lastOpen,
    end: cursor,
    query,
  }
})

const wikiLinkSuggestions = computed(() => {
  const context = wikiLinkContext.value
  if (!context) return []

  const query = context.query.toLowerCase()
  return store.notes
    .filter((note) => {
      if (!query) return true
      return note.slug.toLowerCase().includes(query) || note.title.toLowerCase().includes(query)
    })
    .sort((a, b) => {
      const aTitle = a.title.toLowerCase()
      const bTitle = b.title.toLowerCase()
      const aScore = aTitle.startsWith(query) || a.slug.toLowerCase().startsWith(query) ? 0 : 1
      const bScore = bTitle.startsWith(query) || b.slug.toLowerCase().startsWith(query) ? 0 : 1
      return aScore - bScore || b.updated.localeCompare(a.updated)
    })
    .slice(0, 6)
})

watch(wikiLinkSuggestions, () => {
  syncLinkSuggestionIndex()
})

async function saveDraft() {
  if (!canSaveDraft.value) return
  const note = await store.upsertNote({
    currentSlug: editorMode.value === "edit" ? draft.value.currentSlug : null,
    slug: draft.value.slug.trim() || null,
    title: draft.value.title.trim(),
    body: draft.value.body,
    tags: editorTags.value,
    folder: draft.value.folder,
  })
  editorMode.value = "view"
  deleteConfirm.value = false
  draft.value = {
    currentSlug: note.slug,
    slug: note.slug,
    title: note.title,
    folder: note.folder,
    tagsText: note.tags.join(", "),
    body: note.body,
  }
  if (store.viewMode === "graph") {
    await renderGraph()
  }
}

async function deleteCurrentNote() {
  if (!store.current) return
  await store.deleteNote(store.current.slug)
  deleteConfirm.value = false
  editorMode.value = "view"
  if (store.viewMode === "graph") {
    await renderGraph()
  }
}

function updateEditorSelection() {
  if (!editorTextarea.value) return
  editorSelectionStart.value = editorTextarea.value.selectionStart || 0
  editorSelectionEnd.value = editorTextarea.value.selectionEnd || 0
}

function syncLinkSuggestionIndex() {
  if (!wikiLinkSuggestions.value.length) {
    linkSuggestionIndex.value = 0
    return
  }
  if (linkSuggestionIndex.value >= wikiLinkSuggestions.value.length) {
    linkSuggestionIndex.value = 0
  }
}

function applyWikiLinkSuggestion(slug: string, title: string) {
  const context = wikiLinkContext.value
  if (!context) return

  const inserted = `[[${slug}|${title}]]`
  draft.value.body =
    draft.value.body.slice(0, context.start) +
    inserted +
    draft.value.body.slice(context.end)

  nextTick(() => {
    if (!editorTextarea.value) return
    const cursor = context.start + inserted.length
    editorTextarea.value.focus()
    editorTextarea.value.setSelectionRange(cursor, cursor)
    editorSelectionStart.value = cursor
    editorSelectionEnd.value = cursor
  })
}

function handleEditorKeydown(event: KeyboardEvent) {
  if (!wikiLinkSuggestions.value.length) return

  if (event.key === "ArrowDown") {
    event.preventDefault()
    linkSuggestionIndex.value = (linkSuggestionIndex.value + 1) % wikiLinkSuggestions.value.length
    return
  }

  if (event.key === "ArrowUp") {
    event.preventDefault()
    linkSuggestionIndex.value =
      (linkSuggestionIndex.value - 1 + wikiLinkSuggestions.value.length) % wikiLinkSuggestions.value.length
    return
  }

  if (event.key === "Enter" || event.key === "Tab") {
    event.preventDefault()
    const selected = wikiLinkSuggestions.value[linkSuggestionIndex.value]
    if (selected) {
      applyWikiLinkSuggestion(selected.slug, selected.title)
    }
    return
  }

  if (event.key === "Escape") {
    linkSuggestionIndex.value = 0
  }
}

function renderSearchSnippet(snippet?: string | null): string {
  if (!snippet) return ""
  const escaped = escapeHtml(snippet)
  const query = store.searchQuery.trim()
  if (!query) return escaped

  const pattern = new RegExp(
    `(${escapeRegExp(query)})`,
    "gi"
  )
  return escaped.replace(pattern, "<mark>$1</mark>")
}

function extractLinkedSlugs(body: string): string[] {
  return body
    .match(/\[\[([^\]|]+)(?:\|[^\]]+)?\]\]/g)
    ?.map((item) => item.slice(2, item.indexOf("]")).split("|")[0].trim()) || []
}

function renderMd(body: string, links: string[]): string {
  const slugSet = new Set(links)
  let html = ""
  let inCode = false
  let inList = false

  const lines = body.split("\n")
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    if (line.trimStart().startsWith("```")) {
      if (!inCode) { html += "<pre><code>"; inCode = true }
      else { html += "</code></pre>"; inCode = false }
      continue
    }
    if (inCode) {
      html += escapeHtml(line) + "\n"
      continue
    }
    if (line.trim() === "") {
      if (inList) { html += "</ul>"; inList = false }
      html += "<br/>"
      continue
    }
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
    if (inList) { html += "</ul>"; inList = false }
    html += `<p>${renderInline(line, slugSet)}</p>`
  }
  if (inList) html += "</ul>"
  return html
}

function renderInline(text: string, knownSlugs: Set<string>): string {
  let out = escapeHtml(text)
  out = out.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
  out = out.replace(/\*(.+?)\*/g, "<em>$1</em>")
  out = out.replace(/`(.+?)`/g, "<code>$1</code>")
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

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function onContentClick(e: Event) {
  const el = (e.target as HTMLElement).closest(".wiki-link") as HTMLElement | null
  if (el?.dataset.slug) {
    e.preventDefault()
    openNote(el.dataset.slug)
  }
}

async function renderGraph() {
  await store.loadGraph()
  await nextTick()
  if (!graphCanvas.value || !store.graph) return

  const counts = new Map<string, number>()
  for (const node of store.graph.nodes) {
    counts.set(node.folder || "ghost", (counts.get(node.folder || "ghost") || 0) + 1)
  }
  graphLegend.value = [...counts.entries()].map(([key, count]) => ({
    key,
    label: FOLDER_LABELS[key] || key,
    count,
  }))

  drawGraph(graphCanvas.value, store.graph, openNote)
}

function colorForFolder(folder: string): string {
  return FOLDER_COLORS[folder] || "#c084fc"
}

function drawGraph(canvas: HTMLCanvasElement, data: GraphData, onClick: (s: string) => void) {
  const dpr = window.devicePixelRatio || 1
  const rect = canvas.getBoundingClientRect()
  canvas.width = rect.width * dpr
  canvas.height = rect.height * dpr
  const ctx = canvas.getContext("2d")!
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  const W = rect.width
  const H = rect.height
  const cx = W / 2
  const cy = H / 2

  interface SimNode {
    id: string
    label: string
    folder: string
    tags: string[]
    linkCount: number
    x: number
    y: number
    vx: number
    vy: number
    radius: number
  }

  const folderKeys = [...new Set(data.nodes.map((node) => node.folder || "ghost"))]
  const folderGroups = new Map<string, number>()
  folderKeys.forEach((key, index) => folderGroups.set(key, index))

  const nodes: SimNode[] = data.nodes.map((node) => {
    const ringIndex = folderGroups.get(node.folder || "ghost") || 0
    const groupCount = Math.max(1, folderKeys.length)
    const baseAngle = (Math.PI * 2 * ringIndex) / groupCount
    const clusterRadius = Math.min(W, H) * 0.14
    const anchorX = cx + Math.cos(baseAngle) * clusterRadius
    const anchorY = cy + Math.sin(baseAngle) * clusterRadius
    const radius = Math.min(10, 3.8 + node.link_count * 0.55)
    return {
      id: node.id,
      label: node.label,
      folder: node.folder || "ghost",
      tags: node.tags,
      linkCount: node.link_count,
      x: anchorX + (Math.random() - 0.5) * 120,
      y: anchorY + (Math.random() - 0.5) * 120,
      vx: 0,
      vy: 0,
      radius,
    }
  })

  const nodeMap = new Map(nodes.map((node) => [node.id, node]))
  const edges = data.edges.filter((edge) => nodeMap.has(edge.source) && nodeMap.has(edge.target))
  let hovered: SimNode | null = null
  const alwaysLabeledIds = new Set(
    [...nodes]
      .sort((a, b) => b.linkCount - a.linkCount)
      .slice(0, Math.min(14, Math.ceil(nodes.length * 0.14)))
      .map((node) => node.id)
  )

  for (let iter = 0; iter < 260; iter++) {
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i]
        const b = nodes[j]
        const dx = b.x - a.x
        const dy = b.y - a.y
        const d = Math.max(1, Math.sqrt(dx * dx + dy * dy))
        const push = (a.radius + b.radius + 28) / d
        const fx = (dx / d) * push * 0.23
        const fy = (dy / d) * push * 0.23
        a.vx -= fx
        a.vy -= fy
        b.vx += fx
        b.vy += fy
      }
    }

    for (const edge of edges) {
      const source = nodeMap.get(edge.source)!
      const target = nodeMap.get(edge.target)!
      const dx = target.x - source.x
      const dy = target.y - source.y
      const d = Math.max(1, Math.sqrt(dx * dx + dy * dy))
      const ideal = 82
      const pull = ((d - ideal) / d) * 0.018
      source.vx += dx * pull
      source.vy += dy * pull
      target.vx -= dx * pull
      target.vy -= dy * pull
    }

    for (const node of nodes) {
      const groupIndex = folderGroups.get(node.folder) || 0
      const baseAngle = (Math.PI * 2 * groupIndex) / Math.max(1, folderKeys.length)
      const anchorRadius = Math.min(W, H) * 0.14
      const anchorX = cx + Math.cos(baseAngle) * anchorRadius
      const anchorY = cy + Math.sin(baseAngle) * anchorRadius
      node.vx += (anchorX - node.x) * 0.0018
      node.vy += (anchorY - node.y) * 0.0018
      node.vx += (cx - node.x) * 0.00025
      node.vy += (cy - node.y) * 0.00025

      node.x += node.vx
      node.y += node.vy
      node.vx *= 0.8
      node.vy *= 0.8

      const pad = node.radius + 18
      node.x = Math.max(pad, Math.min(W - pad, node.x))
      node.y = Math.max(pad, Math.min(H - pad, node.y))
    }
  }

  const paint = () => {
    ctx.clearRect(0, 0, W, H)

    ctx.fillStyle = "rgba(9, 12, 18, 0.98)"
    ctx.fillRect(0, 0, W, H)

    ctx.strokeStyle = "rgba(255, 255, 255, 0.03)"
    ctx.lineWidth = 1
    for (let x = 0; x < W; x += 36) {
      ctx.beginPath()
      ctx.moveTo(x, 0)
      ctx.lineTo(x, H)
      ctx.stroke()
    }
    for (let y = 0; y < H; y += 36) {
      ctx.beginPath()
      ctx.moveTo(0, y)
      ctx.lineTo(W, y)
      ctx.stroke()
    }

    for (const edge of edges) {
      const source = nodeMap.get(edge.source)!
      const target = nodeMap.get(edge.target)!
      const connected = hovered && (hovered.id === source.id || hovered.id === target.id)
      ctx.strokeStyle = connected
        ? "rgba(255, 255, 255, 0.34)"
        : "rgba(255, 255, 255, 0.1)"
      ctx.lineWidth = connected ? 1.25 : 0.8
      ctx.beginPath()
      ctx.moveTo(source.x, source.y)
      ctx.lineTo(target.x, target.y)
      ctx.stroke()
    }

    for (const node of nodes) {
      const color = colorForFolder(node.folder)
      const active = hovered?.id === node.id

      ctx.beginPath()
      ctx.fillStyle = color
      ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2)
      ctx.fill()

      ctx.lineWidth = active ? 1.8 : 0.9
      ctx.strokeStyle = active ? "#f8fafc" : "rgba(10, 12, 18, 0.9)"
      ctx.stroke()

      if (active || alwaysLabeledIds.has(node.id)) {
        const label = node.label.length > 18 ? `${node.label.slice(0, 18)}…` : node.label
        ctx.font = active ? "600 12px var(--font, sans-serif)" : "11px var(--font, sans-serif)"
        ctx.textAlign = "left"
        ctx.textBaseline = "middle"
        ctx.fillStyle = active ? "#f8fafc" : "rgba(234, 234, 242, 0.82)"
        ctx.fillText(label, node.x + node.radius + 8, node.y)
      }
    }
  }

  const hitNode = (mx: number, my: number) =>
    nodes.find((node) => {
      const dx = mx - node.x
      const dy = my - node.y
      return dx * dx + dy * dy <= (node.radius + 8) * (node.radius + 8)
    }) || null

  canvas.onmousemove = (event: MouseEvent) => {
    hovered = hitNode(event.offsetX, event.offsetY)
    canvas.style.cursor = hovered ? "pointer" : "default"
    paint()
  }

  canvas.onmouseleave = () => {
    hovered = null
    canvas.style.cursor = "default"
    paint()
  }

  canvas.onclick = (event: MouseEvent) => {
    const target = hitNode(event.offsetX, event.offsetY)
    if (target) onClick(target.id)
  }
  paint()
}
</script>

<template>
  <div class="wiki-view">
    <div class="wiki-header">
      <div>
        <h1 class="wiki-title">知识库</h1>
        <p class="wiki-subtitle">现在不只是浏览器，也是一套可编辑、可维护的本地知识面板。</p>
      </div>
      <div class="wiki-nav">
        <button class="ghost-btn" @click="startCreate">新建笔记</button>
        <button class="ghost-btn" @click="refreshNotes">刷新</button>
        <button :class="{ active: store.viewMode === 'list' }" @click="store.viewMode = 'list'">浏览</button>
        <button :class="{ active: store.viewMode === 'graph' }" @click="renderGraph">图谱</button>
      </div>
    </div>

    <div class="wiki-body">
      <div v-if="store.viewMode === 'list'" class="list-panel">
        <div class="scope-grid">
          <button :class="['scope-card', { active: folderFilter === 'all' }]" @click="folderFilter = 'all'">
            <span class="scope-label">全部范围</span>
            <strong class="scope-value">{{ store.notes.length }}</strong>
            <span class="scope-note">当前知识库所有条目</span>
          </button>
          <button
            v-for="scope in folderStats"
            :key="scope.key"
            :class="['scope-card', { active: folderFilter === scope.key }]"
            @click="folderFilter = scope.key as 'concepts' | 'notes' | 'daily' | 'profile'"
          >
            <span class="scope-label">{{ scope.label }}</span>
            <strong class="scope-value">{{ scope.count }}</strong>
            <span class="scope-note">可作为后续 agent 记忆范围</span>
          </button>
        </div>

        <div class="search-box">
          <input
            v-model="searchInput"
            type="text"
            placeholder="搜索笔记、标签或正文..."
            @input="onSearchInput"
            class="search-input"
          />
          <button v-if="searchInput" class="ghost-btn" @click="clearSearch">清空</button>
          <span class="result-summary">{{ resultSummary }}</span>
          <span v-if="store.loading" class="search-spinner" />
        </div>

        <div v-if="store.searchQuery && filteredSearchResults.length" class="note-list">
          <h3 class="section-title">搜索结果</h3>
          <button
            v-for="note in filteredSearchResults"
            :key="note.slug"
            class="note-card"
            @click="openNote(note.slug)"
          >
            <div class="note-title">{{ note.title }}</div>
            <div class="note-meta">
              <span class="scope-pill">{{ FOLDER_LABELS[note.folder] || note.folder }}</span>
              <span v-for="tag in note.tags" :key="tag" class="tag">{{ tag }}</span>
              <span class="note-date">{{ note.updated.slice(0, 10) }}</span>
            </div>
            <p v-if="note.snippet" class="note-snippet" v-html="renderSearchSnippet(note.snippet)" />
          </button>
        </div>

        <div v-if="!searchInput" class="note-list">
          <div v-if="availableTags.length" class="tag-bar">
            <span v-for="tag in availableTags" :key="tag" class="tag clickable" @click="searchByTag(tag)">
              {{ tag }}
            </span>
          </div>
          <h3 class="section-title">最近更新 · {{ FOLDER_LABELS[folderFilter] }}</h3>
          <button
            v-for="note in filteredNotes"
            :key="note.slug"
            class="note-card"
            @click="openNote(note.slug)"
          >
            <div class="note-title">{{ note.title }}</div>
            <div class="note-meta">
              <span class="scope-pill">{{ FOLDER_LABELS[note.folder] || note.folder }}</span>
              <span v-for="tag in note.tags" :key="tag" class="tag">{{ tag }}</span>
              <span class="note-date">{{ note.updated.slice(0, 10) }}</span>
            </div>
          </button>
          <p v-if="!filteredNotes.length && !store.loading" class="empty-hint">
            还没有笔记。现在可以直接在这里新建和维护知识条目了。
          </p>
        </div>
      </div>

      <div v-else-if="store.viewMode === 'note'" class="note-detail">
        <div class="detail-toolbar">
          <button class="back-btn" @click="store.goBack">← 返回</button>
          <div class="detail-actions">
            <button class="ghost-btn" @click="copyCurrentNote">复制 Markdown</button>
            <button v-if="editorMode === 'view' && store.current" class="ghost-btn" @click="startEdit">编辑</button>
            <button v-if="editorMode === 'view' && store.current" class="ghost-btn danger-ghost" @click="deleteConfirm = !deleteConfirm">
              {{ deleteConfirm ? "取消删除" : "删除" }}
            </button>
            <button v-if="editorMode !== 'view'" class="ghost-btn" @click="cancelEditor">取消</button>
            <button
              v-if="editorMode !== 'view'"
              class="primary-btn"
              :disabled="!canSaveDraft || store.saving"
              @click="saveDraft"
            >
              {{ store.saving ? "保存中..." : "保存笔记" }}
            </button>
            <span v-if="copyMsg" class="detail-msg">{{ copyMsg }}</span>
          </div>
        </div>

        <div v-if="deleteConfirm && store.current && editorMode === 'view'" class="danger-banner">
          <span>确定删除「{{ store.current.title }}」吗？删除后不可恢复。</span>
          <button class="danger-btn" :disabled="store.deleting" @click="deleteCurrentNote">
            {{ store.deleting ? "删除中..." : "确认删除" }}
          </button>
        </div>

        <div v-if="store.error" class="error-banner">{{ store.error }}</div>

        <div v-if="editorMode === 'view' && store.current" class="detail-layout">
          <article class="note-article">
            <header class="article-header">
              <h1>{{ store.current.title }}</h1>
              <div class="article-meta">
                <span class="scope-pill">{{ FOLDER_LABELS[store.current.folder] || store.current.folder }}</span>
                <span v-for="tag in store.current.tags" :key="tag" class="tag">{{ tag }}</span>
                <span class="note-date">更新于 {{ store.current.updated.slice(0, 10) }}</span>
                <span class="note-date">slug: {{ store.current.slug }}</span>
              </div>
            </header>
            <div
              class="article-body"
              v-html="renderMd(store.current.body, extractLinkedSlugs(store.current.body))"
              @click="onContentClick"
            />
          </article>

          <aside class="side-column">
            <section class="mini-panel">
              <div class="mini-title">反向链接</div>
              <button
                v-for="note in currentBacklinks"
                :key="note.slug"
                class="note-card small"
                @click="openNote(note.slug)"
              >
                {{ note.title }}
              </button>
              <div v-if="!currentBacklinks.length" class="mini-empty">暂时没有条目引用这篇笔记。</div>
            </section>
          </aside>
        </div>

        <div v-else class="editor-layout">
          <section class="editor-panel">
            <div class="editor-grid">
              <label class="field">
                <span class="field-label">标题</span>
                <input v-model="draft.title" class="editor-input" type="text" placeholder="例如：Agent 设计思路" />
              </label>

              <label class="field">
                <span class="field-label">Slug</span>
                <input v-model="draft.slug" class="editor-input" type="text" placeholder="留空则根据标题生成" />
              </label>

              <label class="field">
                <span class="field-label">分类</span>
                <ThemeSelect
                  class="editor-input editor-select"
                  :model-value="draft.folder"
                  :options="folderOptions"
                  @update:model-value="(value) => draft.folder = String(value)"
                />
              </label>

              <label class="field field-wide">
                <span class="field-label">标签</span>
                <input v-model="draft.tagsText" class="editor-input" type="text" placeholder="用英文逗号分隔，例如：agent, memory, ui" />
              </label>
            </div>

            <label class="field">
              <span class="field-label">正文</span>
              <textarea
                ref="editorTextarea"
                v-model="draft.body"
                class="editor-textarea"
                placeholder="支持 Markdown 与 [[wiki-link]] 语法"
                @click="updateEditorSelection"
                @keyup="updateEditorSelection"
                @select="updateEditorSelection"
                @keydown="handleEditorKeydown"
              />
            </label>

            <div v-if="wikiLinkSuggestions.length" class="link-suggest-panel">
              <div class="mini-title">Wiki Link 联想</div>
              <div class="link-suggest-list">
                <button
                  v-for="(note, index) in wikiLinkSuggestions"
                  :key="note.slug"
                  :class="['link-suggest-item', { active: index === linkSuggestionIndex }]"
                  @mousedown.prevent="applyWikiLinkSuggestion(note.slug, note.title)"
                >
                  <div class="link-suggest-main">
                    <span class="link-suggest-title">{{ note.title }}</span>
                    <span class="link-suggest-slug">{{ note.slug }}</span>
                  </div>
                  <span class="scope-pill">{{ FOLDER_LABELS[note.folder] || note.folder }}</span>
                </button>
              </div>
            </div>
          </section>

          <section class="preview-panel">
            <div class="mini-title">实时预览</div>
            <div class="article-body preview-body" v-html="draftPreviewHtml" @click="onContentClick" />
          </section>
        </div>
      </div>

      <div v-else-if="store.viewMode === 'graph'" class="graph-layout">
        <section class="graph-panel">
          <canvas ref="graphCanvas" class="graph-canvas" />
        </section>
        <aside class="graph-sidebar">
          <section class="mini-panel">
            <div class="mini-title">图谱图例</div>
            <div class="legend-list">
              <div v-for="item in graphLegend" :key="item.key" class="legend-item">
                <span class="legend-swatch" :style="{ background: colorForFolder(item.key) }" />
                <span class="legend-label">{{ item.label }}</span>
                <span class="legend-count">{{ item.count }}</span>
              </div>
            </div>
          </section>

          <section class="mini-panel">
            <div class="mini-title">阅读提示</div>
            <p class="graph-hint">节点按知识范围分组上色，悬停时会高亮相关连线，点击节点直接打开对应笔记。</p>
          </section>
        </aside>
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

.wiki-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 22px 14px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  background: rgba(8, 10, 14, 0.18);
}

.wiki-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 6px;
}

.wiki-subtitle {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.wiki-nav {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.wiki-nav button,
.ghost-btn,
.primary-btn,
.danger-btn,
.back-btn {
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

.wiki-nav button:hover,
.ghost-btn:hover,
.back-btn:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.18);
}

.wiki-nav button.active,
.primary-btn {
  background: rgba(var(--accent-rgb), 0.14);
  color: var(--text-primary);
  border-color: rgba(var(--accent-rgb), 0.22);
}

.danger-btn,
.danger-ghost:hover {
  border-color: rgba(255, 109, 109, 0.22);
  background: rgba(255, 109, 109, 0.08);
  color: #ffd0d0;
}

.wiki-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 22px 24px;
}

.scope-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
  margin-bottom: 16px;
}

.scope-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px;
  border: 1px solid var(--border-color);
  border-radius: 18px;
  background: var(--surface-1);
  text-align: left;
  cursor: pointer;
  transition: all var(--duration-fast);
  box-shadow: var(--shadow-surface);
}

.scope-card:hover,
.scope-card.active {
  background: rgba(var(--accent-rgb), 0.06);
  border-color: rgba(var(--accent-rgb), 0.18);
  transform: translateY(-1px);
}

.scope-label {
  font-size: 11px;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.scope-value {
  font-size: 20px;
  color: var(--text-primary);
}

.scope-note {
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.search-input,
.editor-input,
.editor-textarea {
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: var(--surface-1);
  color: var(--text-primary);
  font-family: var(--font);
  outline: none;
  transition: border-color var(--duration-fast), box-shadow var(--duration-fast), background var(--duration-fast);
}

.search-input,
.editor-input {
  padding: 11px 14px;
  font-size: 14px;
}

.search-input {
  flex: 1;
}

.search-input:focus,
.editor-input:focus,
.editor-textarea:focus {
  border-color: rgba(var(--accent-rgb), 0.22);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
  background: var(--surface-2);
}

.result-summary,
.detail-msg {
  font-size: 12px;
  color: var(--text-tertiary);
}

.result-summary {
  margin-left: auto;
}

.search-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.tag-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 16px;
}

.scope-pill,
.tag {
  padding: 2px 10px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 500;
}

.tag {
  background: var(--accent-soft);
  color: var(--accent);
}

.scope-pill {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-secondary);
}

.tag.clickable {
  cursor: pointer;
  transition: all var(--duration-fast);
}

.tag.clickable:hover {
  background: var(--accent);
  color: var(--text-on-accent);
}

.section-title,
.mini-title,
.field-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.section-title {
  margin: 16px 0 8px;
}

.note-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

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
  width: 100%;
  box-shadow: var(--shadow-surface);
  transition: all var(--duration-fast);
}

.note-card:hover {
  background: rgba(var(--accent-rgb), 0.06);
  border-color: rgba(var(--accent-rgb), 0.18);
  transform: translateY(-1px);
}

.note-card.small {
  padding: 10px 12px;
}

.note-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.note-snippet {
  margin: 0;
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-secondary);
}

.note-snippet :deep(mark) {
  padding: 0 3px;
  border-radius: 4px;
  background: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.note-meta,
.article-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.note-date {
  font-size: 12px;
  color: var(--text-tertiary);
}

.note-detail {
  max-width: 1180px;
  margin: 0 auto;
}

.detail-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.detail-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.detail-layout,
.editor-layout,
.graph-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 280px;
  gap: 18px;
}

.note-article,
.mini-panel,
.editor-panel,
.preview-panel,
.graph-panel,
.graph-sidebar {
  border-radius: 22px;
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-float);
}

.note-article,
.editor-panel,
.preview-panel,
.graph-sidebar {
  background: rgba(9, 11, 15, 0.62);
}

.note-article {
  padding: 24px 24px 26px;
}

.article-header h1 {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 8px;
}

.article-header {
  margin-bottom: 20px;
}

.article-body {
  font-size: 15px;
  line-height: 1.8;
  color: var(--text-primary);
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

.side-column,
.graph-sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.mini-panel,
.graph-sidebar {
  padding: 18px;
  background: var(--surface-1);
}

.mini-empty,
.graph-hint,
.empty-hint {
  color: var(--text-tertiary);
  font-size: 13px;
  line-height: 1.65;
}

.editor-panel,
.preview-panel {
  padding: 18px;
}

.editor-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin-bottom: 14px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-wide {
  grid-column: span 3;
}

.editor-textarea {
  min-height: 420px;
  padding: 14px;
  resize: vertical;
  font-size: 14px;
  line-height: 1.7;
}

.link-suggest-panel {
  margin-top: 14px;
  padding: 14px;
  border-radius: 18px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.06);
}

.link-suggest-list {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.link-suggest-item {
  width: 100%;
  padding: 10px 12px;
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: rgba(255, 255, 255, 0.02);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  cursor: pointer;
  text-align: left;
  transition: all var(--duration-fast);
}

.link-suggest-item:hover,
.link-suggest-item.active {
  background: rgba(var(--accent-rgb), 0.1);
  border-color: rgba(var(--accent-rgb), 0.22);
}

.link-suggest-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.link-suggest-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.link-suggest-slug {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.preview-body {
  min-height: 480px;
}

.danger-banner,
.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 16px;
  margin-bottom: 14px;
  font-size: 12px;
}

.danger-banner {
  border: 1px solid rgba(255, 109, 109, 0.18);
  background: rgba(255, 109, 109, 0.08);
  color: #ffd0d0;
}

.error-banner {
  border: 1px solid rgba(255, 196, 109, 0.18);
  background: rgba(255, 196, 109, 0.08);
  color: #ffe3b0;
}

.graph-layout {
  align-items: stretch;
}

.graph-panel {
  min-height: 680px;
  overflow: hidden;
  background: rgba(9, 11, 15, 0.62);
}

.graph-canvas {
  width: 100%;
  height: 100%;
  display: block;
}

.legend-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.legend-item {
  display: grid;
  grid-template-columns: 12px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
}

.legend-swatch {
  width: 12px;
  height: 12px;
  border-radius: 999px;
  box-shadow: 0 0 0 4px rgba(255, 255, 255, 0.03);
}

.legend-label {
  color: var(--text-secondary);
  font-size: 13px;
}

.legend-count {
  color: var(--text-tertiary);
  font-size: 12px;
  font-family: var(--font-mono);
}

@media (max-width: 980px) {
  .detail-layout,
  .editor-layout,
  .graph-layout {
    grid-template-columns: 1fr;
  }

  .editor-grid {
    grid-template-columns: 1fr;
  }

  .field-wide {
    grid-column: span 1;
  }
}

@media (max-width: 768px) {
  .wiki-header {
    flex-direction: column;
    align-items: stretch;
  }

  .wiki-body {
    padding: 16px;
  }

  .scope-grid {
    grid-template-columns: 1fr;
  }

  .resultSummary,
  .result-summary {
    width: 100%;
    margin-left: 0;
  }

  .detail-toolbar,
  .detail-actions {
    align-items: stretch;
  }
}
</style>
