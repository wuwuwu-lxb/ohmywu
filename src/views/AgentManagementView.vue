<script setup lang="ts">
import {
  MEMORY_SCOPE_FOLDERS,
  MEMORY_SCOPE_FOLDER_LABELS,
  defaultMemoryScopeLabel,
  summarizeMemoryScope,
  type AgentProfile,
  type MemoryScopeFolder,
  type MemoryScopeMode,
  useAgentStore,
} from "../stores/agents"

const store = useAgentStore()
const recallLimitOptions = [1, 2, 3, 4, 5, 6, 7, 8]

function defaultScopeLabel(mode: MemoryScopeMode, folders: readonly MemoryScopeFolder[]) {
  return defaultMemoryScopeLabel(mode, folders)
}

function syncScopeLabel(agent: AgentProfile, previousLabel: string, previousMode: MemoryScopeMode, previousFolders: readonly MemoryScopeFolder[]) {
  const previousDefault = defaultScopeLabel(previousMode, previousFolders)
  const nextDefault = defaultScopeLabel(agent.memoryScope.mode, agent.memoryScope.folders)
  if (!agent.memoryScope.label.trim() || previousLabel === previousDefault) {
    agent.memoryScope.label = nextDefault
  }
}

function setScopeMode(agent: AgentProfile, mode: MemoryScopeMode) {
  const previousLabel = agent.memoryScope.label
  const previousMode = agent.memoryScope.mode
  const previousFolders = [...agent.memoryScope.folders]

  agent.memoryScope.mode = mode
  if (mode === "none") {
    agent.memoryScope.folders = []
  } else if (mode === "all") {
    agent.memoryScope.folders = [...MEMORY_SCOPE_FOLDERS]
  } else if (!agent.memoryScope.folders.length) {
    agent.memoryScope.folders = ["notes"]
  }

  syncScopeLabel(agent, previousLabel, previousMode, previousFolders)
}

function toggleFolder(agent: AgentProfile, folder: MemoryScopeFolder) {
  const previousLabel = agent.memoryScope.label
  const previousMode = agent.memoryScope.mode
  const previousFolders = [...agent.memoryScope.folders]

  if (agent.memoryScope.mode !== "focused") {
    agent.memoryScope.mode = "focused"
  }

  const next = new Set(agent.memoryScope.folders)
  if (next.has(folder)) {
    next.delete(folder)
  } else {
    next.add(folder)
  }
  agent.memoryScope.folders = MEMORY_SCOPE_FOLDERS.filter((item) => next.has(item))

  syncScopeLabel(agent, previousLabel, previousMode, previousFolders)
}

function toolText(agent: AgentProfile) {
  return agent.tools.join(" · ")
}
</script>

<template>
  <div class="agents-view">
    <header class="page-head">
      <div>
        <h2 class="page-title">Agent 管理</h2>
        <p class="page-subtitle">
          当前重点是把 Agent 的人格、记忆策略和知识范围定义清楚。这里不再绑定固定分类，你可以直接扩展出自己的 Agent 和 Scope。
        </p>
      </div>
      <div class="head-actions">
        <div class="head-pill">Prototype</div>
        <button class="primary-btn" @click="store.addAgent">新增 Agent</button>
      </div>
    </header>

    <section class="overview-grid">
      <article class="overview-card">
        <span class="overview-label">当前策略</span>
        <strong class="overview-value">主 Agent + 可扩展副 Agent</strong>
        <p class="overview-note">不锁死在“编码 / 学习”这样的预设分类，先把配置层做成可生长的结构。</p>
      </article>
      <article class="overview-card">
        <span class="overview-label">记忆模型</span>
        <strong class="overview-value">结构化 Scope</strong>
        <p class="overview-note">支持禁用、全量、定向召回，并可配置标签、目录组合、召回上限和策略说明。</p>
      </article>
      <article class="overview-card">
        <span class="overview-label">后续延展</span>
        <strong class="overview-value">多 Agent / 子调用 / 更细粒度记忆</strong>
        <p class="overview-note">当前这套配置会继续承接真实调度、权限白名单和后续知识库增强。</p>
      </article>
    </section>

    <section class="flow-card">
      <div>
        <h3 class="flow-title">协作草图</h3>
        <p class="flow-note">默认预设只是起点。后续你可以继续拆出“产品研究”、“长期偏好”、“复盘归档”等专门 Agent，而不是被固定角色限制。</p>
      </div>
      <div class="flow-row">
        <span class="flow-node primary">主 Agent</span>
        <span class="flow-arrow">→</span>
        <span class="flow-node">自定义记忆 Agent</span>
        <span class="flow-arrow">→</span>
        <span class="flow-node">自定义执行 Agent</span>
      </div>
    </section>

    <section class="agent-list">
      <article v-for="agent in store.agents" :key="agent.id" class="agent-card">
        <div class="agent-top">
          <div class="agent-head-main">
            <div class="agent-name-row">
              <h3 class="agent-name">{{ agent.name }}</h3>
              <span v-if="agent.primary" class="agent-badge">Primary</span>
              <span v-if="agent.id === store.activeAgentId" class="agent-badge active">Active</span>
            </div>
            <p class="agent-role-preview">{{ agent.role }}</p>
          </div>
          <div class="agent-actions">
            <button class="small-btn" @click="store.setActiveAgent(agent.id)">切换</button>
            <button class="small-btn" @click="store.duplicateAgent(agent.id)">复制</button>
            <button
              class="small-btn danger"
              :disabled="agent.primary"
              @click="store.removeAgent(agent.id)"
            >
              删除
            </button>
          </div>
        </div>

        <div class="two-col">
          <label class="field">
            <span>名称</span>
            <input v-model="agent.name" class="field-input" type="text" />
          </label>
          <label class="field">
            <span>角色</span>
            <input v-model="agent.role" class="field-input" type="text" />
          </label>
        </div>

        <label class="field">
          <span>人格</span>
          <textarea v-model="agent.persona" rows="4" class="field-input multiline" />
        </label>

        <section class="scope-panel">
          <div class="scope-head">
            <div>
              <div class="scope-title">记忆 Scope</div>
              <div class="scope-summary">{{ summarizeMemoryScope(agent.memoryScope) }}</div>
            </div>
            <div class="scope-mode-group">
              <button
                v-for="mode in ['none', 'focused', 'all']"
                :key="mode"
                class="scope-mode-chip"
                :class="{ active: agent.memoryScope.mode === mode }"
                @click="setScopeMode(agent, mode as MemoryScopeMode)"
              >
                {{ mode === "none" ? "禁用" : mode === "all" ? "全量" : "定向" }}
              </button>
            </div>
          </div>

          <div class="scope-grid">
            <label class="field">
              <span>Scope 名称</span>
              <input v-model="agent.memoryScope.label" class="field-input" type="text" placeholder="比如：产品研究 / 长期偏好 / 项目上下文" />
            </label>

            <label class="field">
              <span>召回上限</span>
              <select v-model.number="agent.memoryScope.recallLimit" class="field-input">
                <option v-for="limit in recallLimitOptions" :key="limit" :value="limit">
                  {{ limit }} 条
                </option>
              </select>
            </label>
          </div>

          <div class="field">
            <span>知识目录</span>
            <div class="folder-row">
              <button
                v-for="folder in MEMORY_SCOPE_FOLDERS"
                :key="folder"
                class="folder-chip"
                :class="{
                  active: agent.memoryScope.folders.includes(folder),
                  disabled: agent.memoryScope.mode !== 'focused',
                }"
                @click="toggleFolder(agent, folder)"
              >
                {{ MEMORY_SCOPE_FOLDER_LABELS[folder] }}
                <span class="folder-code">{{ folder }}</span>
              </button>
            </div>
            <div class="scope-tip">
              {{
                agent.memoryScope.mode === "none"
                  ? "当前 Agent 不注入长期记忆。"
                  : agent.memoryScope.mode === "all"
                    ? "当前 Agent 会读取全部知识目录。"
                    : "定向模式下可自由组合目录，后续还能继续扩展到标签、时间窗等更细粒度规则。"
              }}
            </div>
          </div>

          <label class="field">
            <span>记忆策略说明</span>
            <textarea
              v-model="agent.memoryScope.notes"
              rows="3"
              class="field-input multiline"
              placeholder="比如：优先召回产品决策、用户偏好和近期复盘，不要把零碎临时信息都塞进上下文。"
            />
          </label>
        </section>

        <div class="field">
          <span>工具范围</span>
          <div class="tool-block">{{ toolText(agent) }}</div>
        </div>

        <div class="agent-id-row">
          <span class="agent-id">{{ agent.id }}</span>
        </div>
      </article>
    </section>

    <div class="actions-row">
      <button class="reset-btn" @click="store.reset">恢复默认原型</button>
    </div>
  </div>
</template>

<style scoped>
.agents-view {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 32px 40px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.page-head,
.agent-top,
.head-actions,
.agent-actions,
.scope-head,
.scope-mode-group,
.two-col,
.agent-name-row,
.flow-row,
.folder-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.page-head,
.agent-top,
.scope-head {
  justify-content: space-between;
}

.page-title,
.agent-name,
.flow-title {
  margin: 0;
  color: var(--text-primary);
}

.page-title {
  font-size: 22px;
}

.page-subtitle,
.overview-note,
.flow-note,
.agent-role-preview,
.scope-summary,
.scope-tip {
  margin: 6px 0 0;
  color: var(--text-secondary);
  line-height: 1.6;
  font-size: 13px;
}

.head-pill,
.agent-badge,
.tool-block,
.reset-btn,
.small-btn,
.primary-btn,
.scope-mode-chip,
.folder-chip {
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
}

.head-pill,
.agent-badge {
  padding: 6px 10px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.agent-badge.active {
  background: rgba(var(--accent-rgb), 0.16);
}

.primary-btn,
.small-btn,
.reset-btn,
.scope-mode-chip,
.folder-chip {
  cursor: pointer;
}

.primary-btn,
.reset-btn {
  padding: 9px 14px;
  font-size: 12px;
}

.small-btn {
  padding: 7px 11px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.small-btn.danger:disabled {
  opacity: 0.45;
  cursor: default;
}

.overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 10px;
}

.overview-card,
.agent-card,
.flow-card,
.scope-panel {
  border: 1px solid var(--border-color);
  border-radius: 20px;
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
}

.overview-card,
.flow-card {
  padding: 18px;
}

.flow-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.flow-node,
.flow-arrow,
.agent-id,
.folder-code {
  font-family: var(--font-mono);
}

.flow-node {
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--text-primary);
  font-size: 12px;
}

.flow-node.primary {
  border-color: rgba(var(--accent-rgb), 0.24);
  background: rgba(var(--accent-rgb), 0.12);
}

.flow-arrow {
  color: var(--text-tertiary);
  font-size: 12px;
}

.overview-label,
.field span {
  display: block;
  margin-bottom: 8px;
  color: var(--text-tertiary);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.overview-value {
  display: block;
  color: var(--text-primary);
  font-size: 18px;
}

.agent-list {
  display: grid;
  gap: 14px;
}

.agent-card {
  padding: 20px;
}

.agent-head-main {
  min-width: 0;
}

.field {
  display: block;
  margin-top: 14px;
}

.field-input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: rgba(255, 255, 255, 0.022);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}

.field-input:focus {
  border-color: rgba(var(--accent-rgb), 0.22);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
  background: var(--surface-2);
}

.multiline {
  resize: vertical;
  min-height: 96px;
  line-height: 1.6;
}

.two-col {
  align-items: stretch;
}

.two-col > * {
  flex: 1;
  min-width: 0;
}

.scope-panel {
  margin-top: 16px;
  padding: 16px;
}

.scope-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}

.scope-mode-group {
  flex-wrap: wrap;
  justify-content: flex-end;
}

.scope-mode-chip {
  padding: 8px 12px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.scope-mode-chip.active,
.folder-chip.active,
.primary-btn {
  background: rgba(var(--accent-rgb), 0.16);
  border-color: rgba(var(--accent-rgb), 0.28);
}

.scope-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(180px, 0.6fr);
  gap: 12px;
}

.folder-row {
  flex-wrap: wrap;
}

.folder-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  font-size: 12px;
}

.folder-chip.disabled {
  opacity: 0.55;
}

.folder-code {
  color: var(--text-tertiary);
  font-size: 10px;
}

.tool-block {
  border-radius: 16px;
  padding: 12px 14px;
  font-size: 12px;
  line-height: 1.7;
}

.agent-id-row,
.actions-row {
  display: flex;
  justify-content: flex-end;
}

.agent-id {
  color: var(--text-tertiary);
  font-size: 11px;
}

@media (max-width: 880px) {
  .agents-view {
    padding: 20px 18px 28px;
  }

  .page-head,
  .agent-top,
  .two-col,
  .scope-head,
  .head-actions {
    flex-direction: column;
  }

  .scope-grid {
    grid-template-columns: 1fr;
  }
}
</style>
