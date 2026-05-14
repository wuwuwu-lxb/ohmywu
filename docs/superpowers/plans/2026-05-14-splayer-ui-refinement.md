# OhMyWu 前端 UI 精修计划（SPlayer 参考）

> **Goal:** 基于 SPlayer 设计分析，重做 OhMyWu 前端 UI 使其达到桌面级产品质感

**核心问题：** 当前版本把所有组件都加了 backdrop-filter blur，壁纸简陋，缺少色板层次，看起来像"套了层毛玻璃的 demo"而不是精致桌面产品。

## 设计原则（来自 SPlayer 分析）

1. **blur 只在固定大面板**：侧栏、底栏、右抽屉。卡片只用半透明色底，不加 blur
2. **正文永远不透明**：text-primary 保持 `#eaeaf2`
3. **边框是细 rgba 线**：`rgba(255,255,255, 0.06-0.10)`，不是硬白线
4. **壁纸要有氛围感**：多色渐变 + 纹理，不是单调线性渐变
5. **完整色板**：基于 accent 推导 surface/outline/soft/glow
6. **换字体**：不用 Inter

---

## Task 1: 字体升级 — 换掉 Inter

**Files:** `src/style.css`

把 Inter 换成更有性格的字体。SPlayer 用的是系统原生字体栈。我们选 **DM Sans**（正文）+ **Space Mono**（代码），不再用 Inter。

```css
@import url('https://fonts.googleapis.com/css2?family=DM+Sans:opsz,wght@9..40,400;9..40,500;9..40,600;9..40,700&family=Space+Mono:ital,wght@0,400;0,700;1,400&display=swap');

--font: "DM Sans", -apple-system, "SF Pro Display", system-ui, sans-serif;
--font-mono: "Space Mono", "SF Mono", "Fira Code", monospace;
```

---

## Task 2: 壁纸升级 — 四张有氛围感的 CSS 壁纸

**Files:** `src/lib/theme.ts`

替换当前简陋的单色线性渐变。每张壁纸用多色点 + 径向渐变模拟真实壁纸的深度感。

```ts
export const WALLPAPERS: Record<WallpaperId, WallpaperDef> = {
  aurora: {
    label: "Aurora",
    css: `
      radial-gradient(ellipse 80% 60% at 30% 20%, rgba(76, 0, 255, 0.18) 0%, transparent 60%),
      radial-gradient(ellipse 60% 50% at 70% 80%, rgba(0, 200, 180, 0.12) 0%, transparent 55%),
      radial-gradient(ellipse 50% 40% at 50% 50%, rgba(20, 20, 60, 0.6) 0%, transparent 50%),
      linear-gradient(180deg, #0a0a1a 0%, #0d1025 40%, #0f1428 100%)
    `,
  },
  dusk: {
    label: "Dusk", 
    css: `
      radial-gradient(ellipse 70% 50% at 60% 30%, rgba(255, 120, 40, 0.12) 0%, transparent 55%),
      radial-gradient(ellipse 60% 60% at 25% 70%, rgba(80, 60, 160, 0.15) 0%, transparent 50%),
      radial-gradient(ellipse 50% 50% at 50% 50%, rgba(15, 18, 30, 0.5) 0%, transparent 50%),
      linear-gradient(180deg, #0f111e 0%, #161829 50%, #0e1322 100%)
    `,
  },
  lake: {
    label: "Lake",
    css: `
      radial-gradient(ellipse 80% 40% at 50% 80%, rgba(0, 120, 140, 0.14) 0%, transparent 55%),
      radial-gradient(ellipse 60% 50% at 20% 25%, rgba(30, 60, 120, 0.15) 0%, transparent 50%),
      radial-gradient(ellipse 40% 40% at 70% 45%, rgba(10, 30, 50, 0.4) 0%, transparent 55%),
      linear-gradient(180deg, #080e14 0%, #0c141e 50%, #09121c 100%)
    `,
  },
  mono: {
    label: "Mono",
    css: `
      radial-gradient(ellipse 60% 40% at 40% 35%, rgba(255, 255, 255, 0.03) 0%, transparent 55%),
      radial-gradient(ellipse 50% 50% at 60% 60%, rgba(255, 255, 255, 0.02) 0%, transparent 50%),
      linear-gradient(180deg, #0a0a0c 0%, #0d0d12 50%, #0a0a0e 100%)
    `,
  },
}
```

---

## Task 3: 透明度分层 — 只有大面板 blur，卡片不 blur

**Files:** 所有 Vue 组件的 `<style scoped>`

**规则：**
- **blur(12px) 给**：`.app-container`、`.sidebar`、`.right-panel`
- **blur(8px) 给**：`.session-bar`（顶栏）、`.input-wrapper`（输入框）
- **不加 blur，只用半透明色底**：消息气泡、action 卡片、audit 行、设置卡片、预设按钮

当前问题：设置卡片、消息气泡、action 列表全都加了 blur(6-8px)，这导致性能差且看起来像一坨糊。

---

## Task 4: 边框收口 — 统一细线风格

**Files:** `src/style.css` + 所有组件样式

把 `--surface-border` 改为更细的 alpha 线：

```css
--surface-border: rgba(255, 255, 255, 0.07);
--surface-border-hover: rgba(255, 255, 255, 0.12);
--surface-border-active: rgba(255, 255, 255, 0.18);
```

所有组件统一用这三个 token，不再各自设 border 值。

---

## Task 5: 色板扩展 — 从单一 accent 到完整 surface 系统

**Files:** `src/style.css` + `useTheme.ts`

当前只有 `--accent`。需要基于它推导更多 token：

```css
:root {
  /* keep existing */
  --accent: #3b82f6;
  
  /* 新的 SURFACE 系统 */
  --surface-1: rgba(255, 255, 255, 0.04);   /* 最低层 */
  --surface-2: rgba(255, 255, 255, 0.06);   /* 卡片 */
  --surface-3: rgba(255, 255, 255, 0.08);   /* 浮层 */
  --surface-bg: rgba(12, 12, 20, 0.35);      /* 面板背景 */
  
  /* 新的 BORDER 系统 */
  --border-1: rgba(255, 255, 255, 0.06);
  --border-2: rgba(255, 255, 255, 0.09);
  --border-3: rgba(255, 255, 255, 0.14);
  
  /* 阴影系统 */
  --shadow-surface: 0 1px 3px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.04);
  --shadow-float: 0 4px 24px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.06);
}
```

`useTheme.ts` 中的 `applyTheme()` 基于 `surfaceOpacity` 推导这些值。

---

## Task 6: Sidebar 精修

**Files:** `src/components/Sidebar.vue`

- 左侧栏整体保持 blur(12px)
- 导航项的 hover/active 用 `--surface-1` / `color-mix(in srgb, var(--accent) 10%, transparent)`
- 品牌区加点个性：brand-mark 用 accent 渐变
- 折叠态 40px 宽时只显示图标，hover 展开到完整宽度 tooltip

---

## Task 7: ChatView 精修

**Files:** `src/views/ChatView.vue`, `src/components/ChatMessage.vue`

- 消息气泡不加 blur，只加 `background: var(--surface-1); border: 1px solid var(--border-1);`
- 空状态更品牌化：✧ 图标用 accent 渐变圆底
- 输入框 blur(8px)，focus 时 accent 边框 + 柔和 glow
- 消息淡入动画保留

---

## Task 8: RightPanel 精修

**Files:** `src/components/RightPanel.vue`

- 保持 blur(12px) 抽屉
- 展开动画从 0.25s 调为 0.3s ease-out
- 关闭按钮 hover 用 accent 色

---

## Task 9: Actions / Audit / Settings 卡片精修

**Files:** `src/views/ActionsView.vue`, `src/views/AuditView.vue`, `src/views/SettingsView.vue`

- 所有卡片**去掉 backdrop-filter blur**
- 用 `background: var(--surface-1); border: 1px solid var(--border-1); box-shadow: var(--shadow-surface);`
- 壁纸缩略图做大一点（56x36），hover 有 scale 微动
- 颜色预设按钮 hover 时边框变亮

---

## Task 10: 页面入场动画

**Files:** `src/App.vue`（全局）+ 各视图组件

App Shell 加载时侧栏从左侧滑入 + 主区域淡入：

```css
.sidebar {
  animation: slideIn 0.4s var(--ease-out);
}
@keyframes slideIn {
  from { transform: translateX(-20px); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.main-area {
  animation: fadeIn 0.5s 0.1s var(--ease-out) both;
}
```

---

## Task 11: 构建验证

- `npm run build` 通过
- `cargo build` 通过
- 手工检查：壁纸不单调、blur 不糊、文字可读、动画流畅
