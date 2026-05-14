# OhMyWu 前端半透明主题实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** 把 OhMyWu 前端从功能原型界面升级为 SPlayer 风半透明桌面产品，修真保存/真测试

**Architecture:** 
- Rust 后端 `config.rs` 加 `wallpaper` / `surface_opacity` 字段，`lib.rs` 加按表单值测试的 command
- 前端 `useTheme.ts` 改为从配置初始化 + 可持久化
- CSS 层新增壁纸/透明度/半透明面板变量体系
- 各 Vue 组件只改样式和结构 class，不改业务逻辑

**Tech Stack:** Rust (Tauri v2), Vue 3 + `<script setup>`, CSS custom properties

---

### Task 1: 扩展 Rust 后端配置模型与测试命令

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 给 `AppConfig` 加 `wallpaper` 和 `surface_opacity` 字段**

```rust
// config.rs 改动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_policy_mode")]
    pub policy_mode: PolicyMode,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_wallpaper")]
    pub wallpaper: String,
    #[serde(default = "default_surface_opacity")]
    pub surface_opacity: u8,
    #[serde(default)]
    pub llm_provider: Option<LlmConfig>,
}

fn default_wallpaper() -> String { "dusk".into() }
fn default_surface_opacity() -> u8 { 72 }

// Default impl 也需要跟上
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            policy_mode: default_policy_mode(),
            theme: default_theme(),
            accent: default_accent(),
            wallpaper: default_wallpaper(),
            surface_opacity: default_surface_opacity(),
            llm_provider: None,
        }
    }
}
```

- [ ] **Step 2: 在 `lib.rs` 添加按表单值测试的 command**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,
    pub model: Option<String>,
    pub latency_ms: Option<u64>,
}

#[tauri::command]
async fn test_llm_connection_with_config(
    provider_type: String,
    endpoint: String,
    model: String,
    api_key: Option<String>,
) -> Result<LlmTestResult, String> {
    let llm_cfg = ohmywu_llm_adapter::LlmConfig::new(&provider_type, &endpoint, &model, api_key);
    let provider = ohmywu_llm_adapter::create_provider(&llm_cfg)
        .map_err(|e| e.user_friendly().to_string())?;
    match provider.health_check().await {
        Ok(status) => {
            let ohmywu_llm_adapter::HealthStatus::Ok { model, latency_ms } = status;
            Ok(LlmTestResult {
                success: true,
                message: format!("连接成功！Model: {}, 延迟: {}ms", model, latency_ms),
                model: Some(model),
                latency_ms: Some(latency_ms),
            })
        }
        Err(e) => Ok(LlmTestResult {
            success: false,
            message: format!("连接失败 — {} — {}", e.user_friendly(), llm_cfg.endpoint),
            model: None,
            latency_ms: None,
        }),
    }
}
```

- [ ] **Step 3: 在 `lib.rs` 的 `generate_handler!` 注册新 command**

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing ...
    save_config,
    test_llm_connection,
    test_llm_connection_with_config,  // ADD
])
```

- [ ] **Step 4: 编译验证**

Run: `cd project/ohmywu && cargo build 2>&1 | tail -20`
Expected: 编译成功，无错误

---

### Task 2: 打通前端外观初始化与持久化

**Files:**
- Modify: `src/lib/theme.ts` — 壁纸预设、透明度类型
- Modify: `src/composables/useTheme.ts` — 从配置初始化、CSS 变量应用

- [ ] **Step 1: 扩展 `theme.ts` 添加壁纸预设**

```ts
export type ThemePreset = "midnight" | "slate" | "amber"

export interface Theme {
  preset: ThemePreset
  accent: string
}

export const THEME_PRESETS: Record<ThemePreset, { label: string; accent: string }> = {
  midnight: { label: "Midnight", accent: "#3b82f6" },
  slate: { label: "Slate", accent: "#a78bfa" },
  amber: { label: "Amber", accent: "#f59e0b" },
}

export type WallpaperId = "aurora" | "dusk" | "lake" | "mono"

export interface WallpaperDef {
  label: string
  // CSS gradient that simulates the wallpaper
  css: string
}

export const WALLPAPERS: Record<WallpaperId, WallpaperDef> = {
  aurora: {
    label: "Aurora",
    css: "linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%)",
  },
  dusk: {
    label: "Dusk",
    css: "linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)",
  },
  lake: {
    label: "Lake",
    css: "linear-gradient(135deg, #0d1117 0%, #161b22 50%, #1a2332 100%)",
  },
  mono: {
    label: "Mono",
    css: "linear-gradient(135deg, #0a0a0a 0%, #141414 50%, #1a1a1a 100%)",
  },
}
```

- [ ] **Step 2: 重写 `useTheme.ts` 支持初始化与 CSS 变量**

```ts
import { ref } from "vue"
import type { ThemePreset, WallpaperId } from "../lib/theme"
import { THEME_PRESETS, WALLPAPERS } from "../lib/theme"

const preset = ref<ThemePreset>("midnight")
const accent = ref(THEME_PRESETS.midnight.accent)
const wallpaper = ref<WallpaperId>("dusk")
const surfaceOpacity = ref(72)

export function useTheme() {
  const initFromConfig = (cfg: { theme: string; accent: string; wallpaper: string; surface_opacity: number }) => {
    preset.value = (cfg.theme as ThemePreset) || "midnight"
    accent.value = cfg.accent || THEME_PRESETS[preset.value].accent
    wallpaper.value = (cfg.wallpaper as WallpaperId) || "dusk"
    surfaceOpacity.value = cfg.surface_opacity ?? 72
    applyTheme()
  }

  const setPreset = (p: ThemePreset) => {
    preset.value = p
    accent.value = THEME_PRESETS[p].accent
    applyTheme()
  }

  const setAccent = (color: string) => {
    accent.value = color
    applyTheme()
  }

  const setWallpaper = (id: WallpaperId) => {
    wallpaper.value = id
    applyTheme()
  }

  const setSurfaceOpacity = (v: number) => {
    surfaceOpacity.value = Math.max(35, Math.min(88, v))
    applyTheme()
  }

  const applyTheme = () => {
    const d = document.documentElement
    d.style.setProperty("--accent", accent.value)
    d.setAttribute("data-theme", preset.value)
    d.style.setProperty("--wallpaper-css", WALLPAPERS[wallpaper.value].css)
    d.style.setProperty("--wallpaper-id", wallpaper.value)

    // surface opacity drives panel backgrounds
    const surfaceBg = `rgba(12, 12, 20, ${1 - surfaceOpacity.value / 100})`
    const borderOpacity = Math.max(0.08, (1 - surfaceOpacity.value / 100) * 0.6)
    d.style.setProperty("--surface-bg", surfaceBg)
    d.style.setProperty("--surface-border", `rgba(255, 255, 255, ${borderOpacity})`)
  }

  return {
    preset, accent, wallpaper, surfaceOpacity,
    initFromConfig, setPreset, setAccent, setWallpaper, setSurfaceOpacity,
  }
}

export function useSidebar() {
  const collapsed = ref(false)
  const toggle = () => { collapsed.value = !collapsed.value }
  return { collapsed, toggle }
}
```

- [ ] **Step 3: 在 `App.vue` mounted 时从配置初始化**

```ts
// App.vue <script> 部分改动
import { onMounted } from "vue"
import { invoke } from "@tauri-apps/api/core"

const { preset, initFromConfig } = useTheme()

onMounted(async () => {
  try {
    const cfg = await invoke<{ theme: string; accent: string; wallpaper: string; surface_opacity: number }>("get_config")
    initFromConfig(cfg)
  } catch (e) {
    console.error("Init config:", e)
  }
  register({ id: "chat", label: "对话", icon: "💬" })
  register({ id: "actions", label: "Actions", icon: "⚡" })
  register({ id: "audit", label: "审计日志", icon: "📋" })
})
```

---

### Task 3: CSS 全局视觉 token + App Shell 壁纸层

**Files:**
- Modify: `src/style.css` — 壁纸层、半透明变量、面板变量
- Modify: `src/App.vue` — 壁纸层 DOM + 主容器透明壳

- [ ] **Step 1: `style.css` 增加壁纸/半透明 token**

```css
:root {
  /* ... existing vars ... */

  /* NEW — wallpaper & translucency tokens */
  --wallpaper-css: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
  --wallpaper-id: dusk;
  --surface-bg: rgba(12, 12, 20, 0.28);
  --surface-border: rgba(255, 255, 255, 0.08);
  --surface-radius: 12px;
}

/* replace body background */
body {
  /* was: background: var(--bg-base); */
  background: var(--wallpaper-css);
  background-attachment: fixed;
  background-size: cover;
}

/* noise overlay stays */
body::after { /* unchanged */ }
```

- [ ] **Step 2: `App.vue` 改造为壁纸壳层**

```vue
<template>
  <div class="app-shell">
    <!-- wallpaper + ambient overlay -->
    <div class="wallpaper-layer" />
    <div class="ambient-overlay" />

    <!-- main glass container -->
    <div class="app-container">
      <Sidebar :active-id="activeView" @select="onNavSelect" />

      <main class="main-area">
        <ChatView
          v-if="activeView === 'chat'"
          @show-task="handleShowTask"
        />
        <component
          v-else
          :is="viewMap[activeView] || viewMap['chat']"
        />
      </main>

      <RightPanel
        :open="rightPanelOpen"
        title="执行链路"
        @close="rightPanelOpen = false"
      >
        <div v-if="rightPanelTaskId">
          <p>Task ID: {{ rightPanelTaskId }}</p>
          <p class="panel-hint">完整执行链路将在后续版本中展示。</p>
        </div>
        <p v-else class="panel-placeholder">选中一条消息后，这里会显示执行链路详情。</p>
      </RightPanel>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  width: 100vw;
  position: relative;
  overflow: hidden;
}

.wallpaper-layer {
  position: fixed;
  inset: 0;
  background: var(--wallpaper-css);
  background-attachment: fixed;
  z-index: 0;
  transition: background 0.6s ease;
}

.ambient-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  z-index: 1;
  pointer-events: none;
}

.app-container {
  position: relative;
  z-index: 2;
  display: flex;
  height: 100vh;
  background: var(--surface-bg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  margin: 0;
  border-radius: 0;
  overflow: hidden;
  transition: background 0.4s ease;
}

.main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
```

删除旧的 `background: var(--bg-base)` 和 `.app-shell { background: var(--bg-base) }`。

---

### Task 4: Settings 页改造 — 外观卡 + 模型卡 + 真测试

**Files:**
- Modify: `src/views/SettingsView.vue` — 重构为双卡片 + 透明度滑杆 + 壁纸选择 + 真测试
- Modify: `src/composables/useTheme.ts` — 暴露 `currentConfig` 供保存

- [ ] **Step 1: 在 `useTheme.ts` 添加 `currentConfig` 供 save 用**

在 `return` 前加：

```ts
const currentConfig = computed(() => ({
  theme: preset.value,
  accent: accent.value,
  wallpaper: wallpaper.value,
  surface_opacity: surfaceOpacity.value,
}))
```

并在 `return { ..., currentConfig }` 中导出。

- [ ] **Step 2: 重写 `SettingsView.vue` 模板**

替换整个 `<template>` 内容为双卡片结构：

```vue
<template>
  <div class="settings-view">
    <!-- ── Appearance Card ── -->
    <section class="card appearance-card">
      <div class="card-header">
        <h3 class="card-title">外观</h3>
      </div>

      <!-- Theme presets -->
      <div class="field-group">
        <label class="field-label">主题预设</label>
        <div class="preset-grid">
          <button
            v-for="[key, val] in presets"
            :key="key"
            :class="['preset-btn', { active: preset === key }]"
            @click="setPreset(key)"
            :style="{ '--preset-color': val.accent }"
          >
            <span class="preset-swatch" />
            <span class="preset-label">{{ val.label }}</span>
          </button>
        </div>
      </div>

      <!-- Accent color -->
      <div class="field-group">
        <label class="field-label">强调色</label>
        <div class="color-row">
          <input type="color" :value="accent" @input="setAccent(($event.target as HTMLInputElement).value)" class="color-input" />
          <span class="color-value">{{ accent }}</span>
          <button class="reset-btn" @click="setAccent(THEME_PRESETS[preset].accent)">重置</button>
        </div>
      </div>

      <!-- Wallpaper -->
      <div class="field-group">
        <label class="field-label">壁纸</label>
        <div class="wallpaper-grid">
          <button
            v-for="[id, def] in wallpapers"
            :key="id"
            :class="['wallpaper-btn', { active: wallpaper === id }]"
            @click="setWallpaper(id as WallpaperId)"
          >
            <span class="wallpaper-thumb" :style="{ background: def.css }" />
            <span class="wallpaper-label">{{ def.label }}</span>
          </button>
        </div>
      </div>

      <!-- Opacity slider -->
      <div class="field-group">
        <label class="field-label">透明度</label>
        <div class="slider-row">
          <input
            type="range"
            min="35"
            max="88"
            :value="surfaceOpacity"
            @input="setSurfaceOpacity(Number(($event.target as HTMLInputElement).value))"
            class="opacity-slider"
          />
          <span class="slider-value">{{ surfaceOpacity }}%</span>
        </div>
      </div>

      <div class="card-actions">
        <button class="save-btn" :disabled="appearanceSaving" @click="saveAppearance">
          {{ appearanceSaving ? "保存中..." : "保存外观" }}
        </button>
        <span v-if="appearanceMsg" class="msg" :class="{ error: appearanceMsg.startsWith('保存失败') }">{{ appearanceMsg }}</span>
      </div>
    </section>

    <!-- ── LLM Model Card ── -->
    <section class="card model-card">
      <div class="card-header">
        <h3 class="card-title">模型</h3>
        <label class="toggle">
          <input type="checkbox" v-model="llmEnabled" />
          <span class="toggle-track" />
        </label>
      </div>

      <div v-if="llmEnabled" class="model-fields">
        <div class="field-group">
          <label class="field-label">Provider</label>
          <div class="provider-row">
            <select v-model="llmProvider" class="form-input" @change="onProviderChange">
              <option v-for="p in providers" :key="p.id" :value="p.id">{{ p.name }}</option>
            </select>
            <span v-if="currentProvider?.iconColor" class="provider-dot" :style="{ background: currentProvider.iconColor }" />
            <span v-if="currentProvider" class="provider-id">{{ currentProvider.id }}</span>
            <a v-if="currentProvider?.websiteUrl" :href="currentProvider.websiteUrl" target="_blank" class="provider-link">官网</a>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">Endpoint</label>
          <input v-model="llmEndpoint" class="form-input" type="text" :placeholder="currentProvider ? defaultEndpointFor(currentProvider.id) : ''" />
        </div>

        <div class="field-group">
          <label class="field-label">Model</label>
          <input v-model="llmModel" class="form-input" type="text" :placeholder="currentProvider?.defaultModel || ''" />
        </div>

        <div v-if="currentProvider && needsKeyFor(currentProvider.id)" class="field-group">
          <label class="field-label">API Key</label>
          <input v-model="llmApiKey" class="form-input" type="password" placeholder="sk-..." />
        </div>

        <div class="card-actions">
          <button class="save-btn" :disabled="configSaving" @click="saveLlmConfig">
            {{ configSaving ? "保存中..." : "保存" }}
          </button>
          <button class="test-btn" :disabled="testingConnection" @click="testWithCurrentForm">
            {{ testingConnection ? "测试中..." : "测试连接" }}
          </button>
          <span v-if="configMsg" class="msg" :class="{ error: !testSuccess && configMsg !== '已保存' }">{{ configMsg }}</span>
        </div>
      </div>
    </section>
  </div>
</template>
```

- [ ] **Step 3: 更新 `<script>` 添加壁纸相关 import 和 state**

```ts
import type { ThemePreset, WallpaperId } from "../lib/theme"
import { THEME_PRESETS, WALLPAPERS } from "../lib/theme"

const { preset, accent, wallpaper, surfaceOpacity, setPreset, setAccent, setWallpaper, setSurfaceOpacity, currentConfig } = useTheme()
const presets = Object.entries(THEME_PRESETS) as [ThemePreset, { label: string; accent: string }][]
const wallpapers = Object.entries(WALLPAPERS) as [string, { label: string; css: string }][]

const appearanceSaving = ref(false)
const appearanceMsg = ref("")
```

更新 `AppConfig` interface:

```ts
interface AppConfig {
  policy_mode: string
  theme: string
  accent: string
  wallpaper: string
  surface_opacity: number
  llm_provider: LlmConfig | null
}
```

- [ ] **Step 4: 添加 `saveAppearance` 和 `testWithCurrentForm` 方法**

```ts
async function saveAppearance() {
  appearanceSaving.value = true
  appearanceMsg.value = ""
  try {
    const current = await invoke<AppConfig>("get_config")
    const updated: AppConfig = {
      ...current,
      theme: preset.value,
      accent: accent.value,
      wallpaper: wallpaper.value,
      surface_opacity: surfaceOpacity.value,
    }
    await invoke("save_config", { config: updated })
    appearanceMsg.value = "已保存"
    setTimeout(() => (appearanceMsg.value = ""), 2000)
  } catch (e) {
    appearanceMsg.value = `保存失败：${e}`
  } finally {
    appearanceSaving.value = false
  }
}

const testingConnection = ref(false)
const testSuccess = ref(false)

async function testWithCurrentForm() {
  testingConnection.value = true
  configMsg.value = ""
  testSuccess.value = false
  try {
    const result = await invoke<{ success: boolean; message: string; model?: string; latency_ms?: number }>(
      "test_llm_connection_with_config",
      {
        providerType: llmProvider.value,
        endpoint: llmEndpoint.value,
        model: llmModel.value,
        apiKey: llmApiKey.value || null,
      }
    )
    testSuccess.value = result.success
    configMsg.value = result.message
  } catch (e) {
    testSuccess.value = false
    configMsg.value = String(e)
  } finally {
    testingConnection.value = false
  }
}
```

- [ ] **Step 5: 重写 Settings 样式（半透明卡片感）**

完整替换 `<style scoped>`：

```css
.settings-view {
  padding: 24px 32px;
  max-width: 560px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* ── Cards ── */
.card {
  background: var(--surface-bg);
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-xl);
  padding: 20px 24px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  transition: background 0.3s ease, border-color 0.3s ease;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 18px;
}

.card-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.field-group {
  margin-bottom: 16px;
}

.field-group:last-child {
  margin-bottom: 0;
}

.field-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
}

/* Presets */
.preset-grid {
  display: flex;
  gap: 8px;
}

.preset-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.04);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s ease;
}

.preset-btn:hover {
  border-color: var(--preset-color);
  background: rgba(255,255,255,0.06);
}

.preset-btn.active {
  border-color: var(--preset-color);
  background: color-mix(in srgb, var(--preset-color) 15%, transparent);
  box-shadow: 0 0 12px color-mix(in srgb, var(--preset-color) 30%, transparent);
}

.preset-swatch {
  width: 12px; height: 12px;
  border-radius: 50%;
  background: var(--preset-color);
}

.preset-label { font-weight: 500; }

/* Color */
.color-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.color-input {
  width: 36px; height: 36px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-sm);
  padding: 2px;
  background: none;
  cursor: pointer;
}

.color-input::-webkit-color-swatch-wrapper { padding: 0; }
.color-input::-webkit-color-swatch { border: none; border-radius: 4px; }

.color-value {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
}

.reset-btn {
  padding: 4px 10px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-sm);
  background: rgba(255,255,255,0.04);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  font-family: inherit;
}

.reset-btn:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

/* Wallpaper */
.wallpaper-grid {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.wallpaper-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.04);
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s ease;
  min-width: 64px;
}

.wallpaper-btn:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.wallpaper-btn.active {
  border-color: var(--accent);
  color: var(--accent);
}

.wallpaper-thumb {
  width: 48px;
  height: 32px;
  border-radius: var(--radius-xs);
  border: 1px solid rgba(255,255,255,0.08);
}

.wallpaper-label { font-weight: 500; }

/* Slider */
.slider-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.opacity-slider {
  flex: 1;
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: rgba(255,255,255,0.12);
  outline: none;
  cursor: pointer;
}

.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px; height: 16px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid var(--text-primary);
  cursor: pointer;
  transition: transform 0.1s ease;
}

.opacity-slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
}

.opacity-slider::-moz-range-thumb {
  width: 16px; height: 16px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid var(--text-primary);
  cursor: pointer;
}

.slider-value {
  min-width: 40px;
  font-size: 13px;
  font-family: var(--font-mono);
  color: var(--text-secondary);
  text-align: right;
}

/* Model card */
.model-fields {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.toggle { /* switch */ }
.toggle input { display: none; }

.toggle-track {
  display: block;
  width: 36px; height: 20px;
  border-radius: 10px;
  background: rgba(255,255,255,0.1);
  position: relative;
  cursor: pointer;
  transition: background 0.2s ease;
}

.toggle-track::after {
  content: "";
  position: absolute;
  top: 2px; left: 2px;
  width: 16px; height: 16px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition: all 0.2s ease;
}

.toggle input:checked + .toggle-track {
  background: var(--accent);
}

.toggle input:checked + .toggle-track::after {
  left: 18px;
  background: var(--text-on-accent);
}

.provider-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.provider-row select { flex: 1; }

.provider-dot {
  width: 10px; height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.provider-id {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.provider-link {
  font-size: 11px;
  color: var(--accent);
  text-decoration: none;
}

.provider-link:hover { text-decoration: underline; }

.form-input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-sm);
  background: rgba(0,0,0,0.2);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s ease;
}

.form-input:focus {
  border-color: var(--accent);
}

.form-input::placeholder { color: var(--text-tertiary); }

select.form-input { cursor: pointer; }

/* Actions */
.card-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 6px;
}

.save-btn {
  padding: 6px 16px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: var(--text-on-accent);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  transition: opacity 0.15s ease;
}

.save-btn:disabled { opacity: 0.5; cursor: default; }
.save-btn:hover:not(:disabled) { opacity: 0.9; }

.test-btn {
  padding: 6px 12px;
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-sm);
  background: rgba(255,255,255,0.04);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s ease;
}

.test-btn:disabled { opacity: 0.5; cursor: default; }
.test-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.msg {
  font-size: 12px;
  color: var(--text-secondary);
}

.msg.error {
  color: var(--red);
}
```

---

### Task 5: Sidebar 半透明改造

**Files:**
- Modify: `src/components/Sidebar.vue` — 改样式

- [ ] **Step 1: 替换 `<style scoped>` 为半透明面板风格**

```css
.sidebar {
  display: flex;
  flex-direction: column;
  width: var(--sidebar-w);
  min-width: 0;
  background: var(--surface-bg);
  border-right: 1px solid var(--surface-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  transition: width var(--duration-normal) var(--ease-in-out), background 0.3s ease;
  overflow: hidden;
}

/* rest of the styles keep existing selectors but 
   background colors change from solid to translucent */
.sidebar-header {
  border-bottom-color: var(--surface-border);
}

.nav-item.active {
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  color: var(--text-primary);
}

.footer-divider {
  background: var(--surface-border);
}
```

其他选择器基本不变，只改 `background` 和 `border-*` 为 `var(--surface-bg)` / `var(--surface-border)`。

---

### Task 6: ChatView 半透明改造

**Files:**
- Modify: `src/views/ChatView.vue` — 改样式
- Modify: `src/components/ChatMessage.vue` — 改消息气泡样式

- [ ] **Step 1: ChatView 改掉 `background: var(--bg-base)`，去掉硬背景**

```css
.chat-view {
  /* remove: background: var(--bg-base) */
}

.session-bar {
  background: var(--surface-bg);
  border-bottom-color: var(--surface-border);
}

.chat-messages {
  /* remove the solid background */
}

.input-bar {
  /* keep, but cards inside get the surface treatment */
}

.input-wrapper {
  background: var(--surface-bg);
  border-color: var(--surface-border);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}
```

- [ ] **Step 2: `ChatMessage.vue` 用户气泡改半透明**

```css
.user-text {
  background: var(--surface-bg);
  border-color: var(--surface-border);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}
```

---

### Task 7: ActionsView / AuditView / RightPanel 半透明改造

**Files:**
- Modify: `src/views/ActionsView.vue` — 样式改为卡片列表
- Modify: `src/views/AuditView.vue` — 样式改为卡片列表
- Modify: `src/components/RightPanel.vue` — 改为半透明抽屉

- [ ] **Step 1: `ActionsView.vue` 卡片改为半透明**

```css
.action-card {
  background: var(--surface-bg);
  border-color: var(--surface-border);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}
```

- [ ] **Step 2: `AuditView.vue` 行改为半透明**

```css
.audit-row {
  background: var(--surface-bg);
  border: 1px solid var(--surface-border);
  border-radius: var(--radius-md);
  padding: 10px 14px;
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}
```

- [ ] **Step 3: `RightPanel.vue` 改为半透明**

```css
.right-panel {
  background: var(--surface-bg);
  border-left-color: var(--surface-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  transition: width 0.25s ease, background 0.3s ease;
}

.panel-header {
  border-bottom-color: var(--surface-border);
}
```

---

### Task 8: 构建验证

**Files:** (无代码改动，纯运行命令)

- [ ] **Step 1: 前端构建验证**

Run: `cd project/ohmywu && npm run build 2>&1 | tail -30`
Expected: 构建成功，无 TypeScript 错误

- [ ] **Step 2: Rust 构建验证**

Run: `cd project/ohmywu && cargo build 2>&1 | tail -20`
Expected: 编译成功

- [ ] **Step 3: clippy 检查**

Run: `cd project/ohmywu && cargo clippy -- -D warnings 2>&1 | tail -20`
Expected: 无警告
