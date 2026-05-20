<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue"
import { clamp, hexToRgb, hsvToRgb, rgbToHex, rgbToHsv } from "../lib/color"

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{
  "update:modelValue": [value: string]
}>()

const svRef = ref<HTMLElement | null>(null)
const hueRef = ref<HTMLElement | null>(null)
const textValue = ref(props.modelValue)

const hsv = ref(hexToHsv(props.modelValue))
let dragCleanup: null | (() => void) = null

watch(
  () => props.modelValue,
  (value) => {
    textValue.value = value
    hsv.value = hexToHsv(value)
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  dragCleanup?.()
})

const previewColor = computed(() => hsvToHex(hsv.value.h, hsv.value.s, hsv.value.v))
const hueColor = computed(() => hsvToHex(hsv.value.h, 1, 1))
const svCursorStyle = computed(() => ({
  left: `${hsv.value.s * 100}%`,
  top: `${(1 - hsv.value.v) * 100}%`,
}))
const hueCursorStyle = computed(() => ({
  left: `${hsv.value.h * 100}%`,
}))

function hexToHsv(hex: string) {
  const [r, g, b] = hexToRgb(normalizeHex(hex) || "#3b82f6")
  return rgbToHsv(r, g, b)
}

function hsvToHex(h: number, s: number, v: number) {
  const rgb = hsvToRgb(h, s, v)
  return rgbToHex(rgb.r, rgb.g, rgb.b)
}

function normalizeHex(value: string): string | null {
  const trimmed = value.trim()
  if (!/^#?([\da-fA-F]{6}|[\da-fA-F]{3})$/.test(trimmed)) {
    return null
  }
  const withHash = trimmed.startsWith("#") ? trimmed : `#${trimmed}`
  if (withHash.length === 4) {
    return `#${withHash[1]}${withHash[1]}${withHash[2]}${withHash[2]}${withHash[3]}${withHash[3]}`.toLowerCase()
  }
  return withHash.toLowerCase()
}

function commitColor(value: string) {
  emit("update:modelValue", value)
}

function commitTextValue() {
  const normalized = normalizeHex(textValue.value)
  if (!normalized) {
    textValue.value = previewColor.value
    return
  }
  hsv.value = hexToHsv(normalized)
  commitColor(normalized)
}

function startPointerDrag(event: MouseEvent, move: (next: MouseEvent) => void) {
  move(event)
  const onMove = (next: MouseEvent) => move(next)
  const onUp = () => {
    document.removeEventListener("mousemove", onMove)
    document.removeEventListener("mouseup", onUp)
    dragCleanup = null
  }
  document.addEventListener("mousemove", onMove)
  document.addEventListener("mouseup", onUp)
  dragCleanup = onUp
}

function pickSv(event: MouseEvent) {
  if (!svRef.value) return
  const rect = svRef.value.getBoundingClientRect()
  const s = clamp((event.clientX - rect.left) / rect.width, 0, 1)
  const v = clamp(1 - (event.clientY - rect.top) / rect.height, 0, 1)
  hsv.value = { ...hsv.value, s, v }
  commitColor(hsvToHex(hsv.value.h, s, v))
}

function pickHue(event: MouseEvent) {
  if (!hueRef.value) return
  const rect = hueRef.value.getBoundingClientRect()
  const h = clamp((event.clientX - rect.left) / rect.width, 0, 1)
  hsv.value = { ...hsv.value, h }
  commitColor(hsvToHex(h, hsv.value.s, hsv.value.v))
}
</script>

<template>
  <div class="color-picker-field">
    <div class="picker-shell">
      <div
        ref="svRef"
        class="sv-panel"
        :style="{ backgroundColor: hueColor }"
        @mousedown.prevent="startPointerDrag($event, pickSv)"
      >
        <div class="sv-white" />
        <div class="sv-black" />
        <span class="sv-cursor" :style="svCursorStyle" />
      </div>

      <div
        ref="hueRef"
        class="hue-strip"
        @mousedown.prevent="startPointerDrag($event, pickHue)"
      >
        <span class="hue-cursor" :style="hueCursorStyle" />
      </div>
    </div>

    <div class="picker-inputs">
      <div class="picker-preview" :style="{ background: previewColor }" />
      <input
        v-model="textValue"
        class="picker-text"
        type="text"
        spellcheck="false"
        :placeholder="placeholder || '#3b82f6'"
        @change="commitTextValue"
        @blur="commitTextValue"
      />
    </div>
  </div>
</template>

<style scoped>
.color-picker-field {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
}

.picker-shell {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.sv-panel {
  position: relative;
  width: 100%;
  min-height: 172px;
  border-radius: 16px;
  overflow: hidden;
  border: 1px solid var(--border-color);
  cursor: crosshair;
}

.sv-white,
.sv-black {
  position: absolute;
  inset: 0;
}

.sv-white {
  background: linear-gradient(90deg, #fff, rgba(255, 255, 255, 0));
}

.sv-black {
  background: linear-gradient(180deg, rgba(0, 0, 0, 0), #000);
}

.sv-cursor,
.hue-cursor {
  position: absolute;
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.sv-cursor {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid #fff;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
}

.hue-strip {
  position: relative;
  height: 14px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: linear-gradient(
    90deg,
    #ff0000 0%,
    #ffff00 16.66%,
    #00ff00 33.33%,
    #00ffff 50%,
    #0000ff 66.66%,
    #ff00ff 83.33%,
    #ff0000 100%
  );
  cursor: ew-resize;
}

.hue-cursor {
  top: 50%;
  width: 12px;
  height: 22px;
  border-radius: 999px;
  border: 2px solid #fff;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
}

.picker-inputs {
  display: flex;
  align-items: center;
  gap: 12px;
}

.picker-preview {
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  border-radius: 14px;
  border: 1px solid var(--border-color);
}

.picker-text {
  width: 100%;
  min-width: 0;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.022);
  color: var(--text-primary);
  font-size: 13px;
  font-family: var(--font-mono);
  outline: none;
}

.picker-text:focus {
  border-color: rgba(var(--accent-rgb), 0.22);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
  background: var(--surface-2);
}
</style>
