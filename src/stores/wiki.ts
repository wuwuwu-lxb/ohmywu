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
  snippet?: string | null
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
  folder: string
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
  const saving = ref(false)
  const deleting = ref(false)
  const error = ref<string | null>(null)
  const viewMode = ref<"list" | "note" | "graph">("list")

  async function loadNotes() {
    loading.value = true
    error.value = null
    try {
      notes.value = await invoke<NoteMeta[]>("wiki_list_notes")
    } catch (e) {
      console.error("wiki list:", e)
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function readNote(slug: string) {
    loading.value = true
    error.value = null
    try {
      current.value = await invoke<WikiNote>("wiki_read_note", { slug })
      viewMode.value = "note"
    } catch (e) {
      console.error("wiki read:", e)
      error.value = String(e)
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
    error.value = null
    try {
      searchResults.value = await invoke<NoteMeta[]>("wiki_search_notes", { query: q })
    } catch (e) {
      console.error("wiki search:", e)
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function loadGraph() {
    loading.value = true
    error.value = null
    try {
      graph.value = await invoke<GraphData>("wiki_get_graph")
      viewMode.value = "graph"
    } catch (e) {
      console.error("wiki graph:", e)
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function upsertNote(input: {
    currentSlug?: string | null
    slug?: string | null
    title: string
    body: string
    tags: string[]
    folder: string
  }) {
    saving.value = true
    error.value = null
    try {
      const note = await invoke<WikiNote>("wiki_upsert_note", {
        currentSlug: input.currentSlug ?? null,
        slug: input.slug ?? null,
        title: input.title,
        body: input.body,
        tags: input.tags,
        folder: input.folder,
      })
      current.value = note
      viewMode.value = "note"
      await loadNotes()
      return note
    } catch (e) {
      console.error("wiki upsert:", e)
      error.value = String(e)
      throw e
    } finally {
      saving.value = false
    }
  }

  async function deleteNote(slug: string) {
    deleting.value = true
    error.value = null
    try {
      await invoke("wiki_delete_note", { slug })
      if (current.value?.slug === slug) {
        current.value = null
        viewMode.value = "list"
      }
      await loadNotes()
    } catch (e) {
      console.error("wiki delete:", e)
      error.value = String(e)
      throw e
    } finally {
      deleting.value = false
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
    notes, current, graph, searchQuery, searchResults, loading, saving, deleting, error, viewMode,
    loadNotes, readNote, search, loadGraph, upsertNote, deleteNote, selectNote, goBack,
  }
})
