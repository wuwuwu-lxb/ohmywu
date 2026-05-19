import { defineStore } from "pinia"
import { ref } from "vue"
import { invoke } from "@tauri-apps/api/core"

export interface NoteMeta {
  slug: string
  title: string
  folder: string
  tags: string[]
  created: string
  updated: string
  links_to: string[]
  linked_from: string[]
}

export interface WikiNote {
  slug: string
  title: string
  folder: string
  tags: string[]
  created: string
  updated: string
  body: string
}

export interface GraphData {
  nodes: GraphNode[]
  edges: GraphEdge[]
}

export interface GraphNode {
  id: string
  label: string
  tags: string[]
  link_count: number
}

export interface GraphEdge {
  source: string
  target: string
}

export const useWikiStore = defineStore("wiki", () => {
  const notes = ref<NoteMeta[]>([])
  const current = ref<WikiNote | null>(null)
  const graph = ref<GraphData | null>(null)
  const searchQuery = ref("")
  const searchResults = ref<NoteMeta[]>([])
  const loading = ref(false)
  const viewMode = ref<"list" | "note" | "graph">("list")

  async function loadNotes() {
    loading.value = true
    try {
      notes.value = await invoke<NoteMeta[]>("wiki_list_notes")
    } catch (e) {
      console.error("wiki list:", e)
    } finally {
      loading.value = false
    }
  }

  async function readNote(slug: string) {
    loading.value = true
    try {
      current.value = await invoke<WikiNote>("wiki_read_note", { slug })
      viewMode.value = "note"
    } catch (e) {
      console.error("wiki read:", e)
    } finally {
      loading.value = false
    }
  }

  async function search(q: string) {
    searchQuery.value = q
    if (!q.trim()) {
      searchResults.value = []
      return
    }
    loading.value = true
    try {
      searchResults.value = await invoke<NoteMeta[]>("wiki_search_notes", { query: q })
    } catch (e) {
      console.error("wiki search:", e)
    } finally {
      loading.value = false
    }
  }

  async function loadGraph() {
    loading.value = true
    try {
      graph.value = await invoke<GraphData>("wiki_get_graph")
      viewMode.value = "graph"
    } catch (e) {
      console.error("wiki graph:", e)
    } finally {
      loading.value = false
    }
  }

  function selectNote(slug: string) {
    readNote(slug)
  }

  function goBack() {
    viewMode.value = "list"
    current.value = null
  }

  return {
    notes, current, graph, searchQuery, searchResults, loading, viewMode,
    loadNotes, readNote, search, loadGraph, selectNote, goBack,
  }
})
