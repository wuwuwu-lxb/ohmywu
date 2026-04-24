<template>
  <div
    class="live2d-pet"
    :class="{ 'lightweight': isLightweightMode, 'dragging': isDragging }"
    :style="petStyle"
    ref="petRef"
    @mousedown="onMouseDown"
    v-show="visible"
  >
    <div class="pet-container" ref="containerRef">
      <canvas ref="canvasRef"></canvas>
    </div>
    <div v-if="isLightweightMode" class="exit-hint" @click.stop="exitLightweight">
      点击返回桌面
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'

interface Props {
  isLightweightMode?: boolean
}
const props = withDefaults(defineProps<Props>(), {
  isLightweightMode: false,
})
const emit = defineEmits<{
  (e: 'exit-lightweight'): void
}>()

const visible = ref(true)
const canvasRef = ref<HTMLCanvasElement>()
const containerRef = ref<HTMLDivElement>()
const petRef = ref<HTMLDivElement>()

const isDragging = ref(false)
const dragOffset = ref({ x: 0, y: 0 })
const position = ref({ x: 0, y: 0 })

let app: any = null
let live2dModel: any = null
let idleTimer: ReturnType<typeof setInterval> | null = null

const petStyle = computed(() => {
  if (props.isLightweightMode) {
    return {
      left: '50%',
      top: '50%',
      transform: 'translate(-50%, -50%)',
      width: '300px',
      height: '400px',
    }
  }
  return {
    right: `${position.value.x}px`,
    bottom: `${position.value.y}px`,
    left: 'auto',
    top: 'auto',
    transform: 'none',
    width: '180px',
    height: '240px',
  }
})

const MODELS: Record<string, { name: string; path: string }> = {
  shizuku: { name: '🌸 初音未来', path: '/live2d/shizuku.model3.json' },
  xingye:  { name: '⭐ 星野',     path: '/live2d/xingye/星野.model3.json' },
  ug:      { name: '🎮 明日方舟', path: '/live2d/ug/ugofficial.model3.json' },
}
const CURRENT = 'shizuku'

function installRenderGuard(pixiApp: any) {
  const renderer = pixiApp.renderer
  if (!renderer) return
  const originalRender = renderer.render.bind(renderer)
  renderer.render = function (...args: any[]) {
    try {
      originalRender(...args)
    } catch (err) {
      console.warn('[RenderGuard] WebGL 错误已捕获:', err)
    }
  }
}

function startIdleLoop() {
  if (!live2dModel) return
  try { live2dModel.motion('Idle') } catch (e) { /* ignore */ }
  idleTimer = setInterval(() => {
    if (live2dModel) {
      try { live2dModel.motion('Idle') } catch (e) { /* ignore */ }
    }
  }, 8000)
}

function stopIdleLoop() {
  if (idleTimer !== null) {
    clearInterval(idleTimer)
    idleTimer = null
  }
}

async function initLive2D() {
  const modelConfig = MODELS[CURRENT]
  const MODEL_PATH = modelConfig.path

  console.log('[Live2DPet] 加载模型:', modelConfig.name, MODEL_PATH)

  const maxWait = 10000
  const interval = 200
  let elapsed = 0

  await new Promise<void>((resolve) => {
    const check = setInterval(() => {
      if (window.PIXI && window.PIXI.live2d && window.PIXI.live2d.Live2DModel) {
        clearInterval(check)
        resolve()
      } else {
        elapsed += interval
        if (elapsed >= maxWait) {
          clearInterval(check)
          resolve()
        }
      }
    }, interval)
  })

  if (!window.PIXI || !window.PIXI.live2d || !window.PIXI.live2d.Live2DModel) {
    console.warn('[Live2DPet] PIXI/Live2D 库未加载，跳过初始化')
    return
  }

  const Live2DModel = window.PIXI.live2d.Live2DModel

  try {
    const width = props.isLightweightMode ? 300 : 180
    const height = props.isLightweightMode ? 400 : 240

    app = new window.PIXI.Application({
      width,
      height,
      backgroundAlpha: 0,
      antialias: true,
      preserveDrawingBuffer: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
      view: canvasRef.value,
    })

    installRenderGuard(app)

    const model = await Live2DModel.from(MODEL_PATH, { autoInteract: false })
    live2dModel = model

    const scale = props.isLightweightMode ? 0.8 : 0.5
    model.x = 20
    model.y = 40
    model.width = 160 * scale
    model.height = 240 * scale

    app.stage.addChild(model)

    canvasRef.value?.addEventListener('pointermove', (e) => {
      model.focus(e.clientX, e.clientY)
    })

    startIdleLoop()

    model.on('hit', (hitAreas: string[]) => {
      console.log('[Live2DPet] HitArea:', hitAreas)
      if (hitAreas.includes('Head')) {
        console.log('[Live2DPet] 点击头部')
        model.motion('FlickUp')
      } else if (hitAreas.includes('Body')) {
        console.log('[Live2DPet] 点击身体')
        model.motion('Tap')
      }
    })

    canvasRef.value?.addEventListener('pointerdown', (e) => {
      const rect = canvasRef.value!.getBoundingClientRect()
      const y = e.clientY - rect.top
      if (y < rect.height / 2) {
        console.log('[Live2DPet] 点击上半部分 (头部区域)')
        model.motion('FlickUp')
      } else {
        console.log('[Live2DPet] 点击下半部分 (身体区域)')
        model.motion('Tap')
      }
    })

    model.interactive = true
    model.on('pointerover', () => { document.body.style.cursor = 'pointer' })
    model.on('pointerout', () => { document.body.style.cursor = 'default' })

    console.log('[Live2DPet] ✅ 模型加载成功')
  } catch (err) {
    console.error('[Live2DPet] ❌ 模型加载失败:', err)
  }
}

function onMouseDown(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('.exit-hint')) return
  if (props.isLightweightMode) return

  isDragging.value = true
  const rect = petRef.value!.getBoundingClientRect()
  dragOffset.value = {
    x: e.clientX - rect.left,
    y: e.clientY - rect.top,
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

function onMouseMove(e: MouseEvent) {
  if (!isDragging.value) return
  const parent = petRef.value!.parentElement!
  const parentRect = parent.getBoundingClientRect()
  const newX = parentRect.right - e.clientX + dragOffset.value.x
  const newY = parentRect.bottom - e.clientY + dragOffset.value.y
  position.value = { x: newX, y: newY }
}

function onMouseUp() {
  isDragging.value = false
  document.removeEventListener('mousemove', onMouseMove)
  document.removeEventListener('mouseup', onMouseUp)
}

function exitLightweight() {
  emit('exit-lightweight')
}

onMounted(async () => {
  await nextTick()
  await initLive2D()
})

onUnmounted(() => {
  stopIdleLoop()
  if (app) {
    app.destroy(false)
    app = null
  }
})

defineExpose({
  setVisible(v: boolean) {
    visible.value = v
  },
})
</script>

<style scoped>
.live2d-pet {
  position: fixed;
  z-index: 9999;
  cursor: grab;
  user-select: none;
}

.live2d-pet.dragging {
  cursor: grabbing;
}

.live2d-pet.lightweight {
  cursor: default;
}

.pet-container {
  width: 100%;
  height: 100%;
  pointer-events: auto;
}

.pet-container canvas {
  display: block;
  width: 100% !important;
  height: 100% !important;
}

.exit-hint {
  position: absolute;
  bottom: 8px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  color: rgba(255, 255, 255, 0.7);
  background: rgba(0, 0, 0, 0.4);
  padding: 4px 12px;
  border-radius: 12px;
  cursor: pointer;
  pointer-events: auto;
  backdrop-filter: blur(4px);
  transition: color 0.2s, background 0.2s;
}

.exit-hint:hover {
  color: #fff;
  background: rgba(0, 0, 0, 0.6);
}
</style>
