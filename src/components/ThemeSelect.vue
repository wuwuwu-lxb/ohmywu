<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue"

type SelectValue = string | number

interface SelectOption {
  label: string
  value: SelectValue
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue: SelectValue
  options: SelectOption[]
  placeholder?: string
  disabled?: boolean
}>(), {
  placeholder: "请选择",
  disabled: false,
})

const emit = defineEmits<{
  "update:modelValue": [value: SelectValue]
  "change": [value: SelectValue]
}>()

const rootEl = ref<HTMLElement | null>(null)
const menuEl = ref<HTMLElement | null>(null)
const open = ref(false)
const activeIndex = ref(0)
const openUpward = ref(false)
const menuStyle = ref<Record<string, string>>({})

const selectedOption = computed(
  () => props.options.find((option) => option.value === props.modelValue) || null
)

const enabledOptions = computed(() => props.options.filter((option) => !option.disabled))

function closeMenu() {
  open.value = false
}

function toggleMenu() {
  if (props.disabled) return
  open.value = !open.value
  syncActiveIndex()
}

function syncActiveIndex() {
  const selectedIndex = enabledOptions.value.findIndex(
    (option) => option.value === props.modelValue
  )
  activeIndex.value = selectedIndex >= 0 ? selectedIndex : 0
}

function selectOption(option: SelectOption) {
  if (option.disabled) return
  emit("update:modelValue", option.value)
  emit("change", option.value)
  closeMenu()
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as Node
  if (rootEl.value?.contains(target) || menuEl.value?.contains(target)) {
    return
  }
  if (!rootEl.value) return
  if (!rootEl.value.contains(target)) {
    closeMenu()
  }
}

function updateMenuPosition() {
  if (!open.value || !rootEl.value || !menuEl.value) return

  const rect = rootEl.value.getBoundingClientRect()
  const viewportHeight = window.innerHeight
  const viewportWidth = window.innerWidth
  const gap = 8
  const preferredMaxHeight = 240
  const spaceBelow = viewportHeight - rect.bottom - gap
  const spaceAbove = rect.top - gap
  const maxHeight = Math.min(
    preferredMaxHeight,
    Math.max(120, Math.max(spaceBelow, spaceAbove))
  )

  openUpward.value = spaceBelow < 180 && spaceAbove > spaceBelow

  const width = Math.max(rect.width, 160)
  const left = Math.min(rect.left, viewportWidth - width - gap)
  const availableHeight = Math.min(
    maxHeight,
    Math.max(120, openUpward.value ? spaceAbove : spaceBelow)
  )

  menuStyle.value = {
    position: "fixed",
    left: `${Math.max(gap, left)}px`,
    width: `${width}px`,
    maxHeight: `${availableHeight}px`,
    overflowY: "auto",
  }

  const measuredHeight = Math.min(menuEl.value.offsetHeight || 0, availableHeight)
  const top = openUpward.value
    ? Math.max(gap, rect.top - measuredHeight - gap)
    : Math.min(viewportHeight - measuredHeight - gap, rect.bottom + gap)

  menuStyle.value = {
    ...menuStyle.value,
    top: `${Math.max(gap, top)}px`,
  }
}

function handleMenuWheel(event: WheelEvent) {
  if (!menuEl.value) return
  const el = menuEl.value
  const atTop = el.scrollTop <= 0
  const atBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 1

  if ((event.deltaY < 0 && atTop) || (event.deltaY > 0 && atBottom)) {
    event.preventDefault()
  }
  event.stopPropagation()
}

function handleKeydown(event: KeyboardEvent) {
  if (props.disabled) return

  if ((event.key === "Enter" || event.key === " ") && !open.value) {
    event.preventDefault()
    open.value = true
    syncActiveIndex()
    return
  }

  if (!open.value) return

  if (event.key === "Escape") {
    event.preventDefault()
    closeMenu()
    return
  }

  if (event.key === "ArrowDown") {
    event.preventDefault()
    activeIndex.value = (activeIndex.value + 1) % Math.max(1, enabledOptions.value.length)
    return
  }

  if (event.key === "ArrowUp") {
    event.preventDefault()
    activeIndex.value =
      (activeIndex.value - 1 + Math.max(1, enabledOptions.value.length)) %
      Math.max(1, enabledOptions.value.length)
    return
  }

  if (event.key === "Enter" || event.key === "Tab") {
    const option = enabledOptions.value[activeIndex.value]
    if (option) {
      event.preventDefault()
      selectOption(option)
    }
  }
}

onMounted(() => {
  document.addEventListener("mousedown", handleClickOutside)
  syncActiveIndex()
  window.addEventListener("resize", updateMenuPosition)
  window.addEventListener("scroll", updateMenuPosition, true)
})

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", handleClickOutside)
  window.removeEventListener("resize", updateMenuPosition)
  window.removeEventListener("scroll", updateMenuPosition, true)
})

watch(open, async (value) => {
  if (!value) return
  await nextTick()
  updateMenuPosition()
  menuEl.value?.scrollTo({
    top: Math.max(0, activeIndex.value * 40 - 80),
    behavior: "auto",
  })
})
</script>

<template>
  <div
    ref="rootEl"
    class="theme-select"
    :class="{ open, disabled }"
    tabindex="0"
    @keydown="handleKeydown"
  >
    <button
      class="theme-select-trigger"
      type="button"
      :disabled="disabled"
      @click="toggleMenu"
    >
      <span class="theme-select-label truncate">
        {{ selectedOption?.label || placeholder }}
      </span>
      <span class="theme-select-arrow">⌄</span>
    </button>

    <Teleport to="body">
      <div
        v-if="open"
        ref="menuEl"
        class="theme-select-menu"
        :class="{ upward: openUpward }"
        :style="menuStyle"
        @wheel="handleMenuWheel"
      >
        <button
          v-for="option in options"
          :key="String(option.value)"
          class="theme-select-option"
          :class="{
            active: option.value === modelValue,
            disabled: option.disabled,
          }"
          type="button"
          :disabled="option.disabled"
          @click="selectOption(option)"
        >
          <span class="truncate">{{ option.label }}</span>
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.theme-select {
  position: relative;
  min-width: 0;
  min-height: 40px;
  padding: 0 !important;
  border-radius: 12px;
  outline: none;
}

.theme-select-trigger {
  width: 100%;
  height: 100%;
  min-height: 40px;
  padding: 0 12px;
  border-radius: inherit;
  border: inherit;
  background: inherit;
  color: inherit;
  font: inherit;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  cursor: pointer;
}

.theme-select-label {
  min-width: 0;
  text-align: left;
}

.theme-select-arrow {
  font-size: 11px;
  color: var(--text-tertiary);
  transition: transform var(--duration-fast) var(--ease-out);
}

.theme-select.open .theme-select-arrow {
  transform: rotate(180deg);
}

.theme-select-menu {
  z-index: 999;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  border-radius: 16px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(14, 18, 26, 0.98);
  box-shadow: 0 18px 36px rgba(0, 0, 0, 0.34);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  overscroll-behavior: contain;
}

.theme-select-menu.upward {
  box-shadow: 0 -18px 36px rgba(0, 0, 0, 0.24);
}

.theme-select-option {
  width: 100%;
  min-height: 36px;
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: 12px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  text-align: left;
  display: flex;
  align-items: center;
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.theme-select-option:hover,
.theme-select-option.active {
  background: rgba(var(--accent-rgb), 0.1);
  border-color: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.theme-select-option.disabled,
.theme-select.disabled .theme-select-trigger {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
