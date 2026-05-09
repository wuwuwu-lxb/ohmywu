import { ref, type Component, markRaw } from "vue"

export interface NavItem {
  id: string
  label: string
  icon: string
  component?: Component
  route?: string
  badge?: number
}

const items = ref<NavItem[]>([])

export function useSidebarNav() {
  const register = (item: NavItem) => {
    if (!items.value.find((i) => i.id === item.id)) {
      items.value.push({ ...item, component: item.component ? markRaw(item.component) : undefined })
    }
  }

  const unregister = (id: string) => {
    items.value = items.value.filter((i) => i.id !== id)
  }

  const setBadge = (id: string, count: number) => {
    const item = items.value.find((i) => i.id === id)
    if (item) item.badge = count
  }

  return { items, register, unregister, setBadge }
}
