<script setup lang="ts">
import { computed, ref } from "vue"
import ExecutionCard from "./ExecutionCard.vue"
import type { RuntimeTurnView } from "../stores/chat"

const expandedState = new Map<string, boolean>()
const contextExpandedState = new Map<string, boolean>()
const toolExpandedState = new Map<string, boolean>()

const props = defineProps<{
  runtime: RuntimeTurnView
}>()

const expanded = ref(
  expandedState.get(props.runtime.turn.id) ?? props.runtime.turn.status === "running"
)
const contextExpanded = ref(
  contextExpandedState.get(props.runtime.turn.id) ?? false
)
const toolExpanded = ref(
  toolExpandedState.get(props.runtime.turn.id) ?? true
)

const toolCount = computed(() =>
  props.runtime.tools.length || props.runtime.turn.executionCount
)

const delegationCount = computed(() =>
  props.runtime.tools.filter((tool) => !!tool.delegated).length
)

const elapsedLabel = computed(() => {
  const started = Date.parse(props.runtime.turn.startedAt)
  const finished = props.runtime.turn.finishedAt
    ? Date.parse(props.runtime.turn.finishedAt)
    : Date.now()
  if (Number.isNaN(started) || Number.isNaN(finished) || finished < started) {
    return null
  }
  const ms = finished - started
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
})

const ttftLabel = computed(() => {
  const event = props.runtime.events.find((item) => item.kind === "provider.first_token")
  const elapsed = event?.payload?.elapsedMs
  if (typeof elapsed !== "number") return null
  if (elapsed < 1000) return `TTFT ${elapsed}ms`
  return `TTFT ${(elapsed / 1000).toFixed(1)}s`
})

const firstToolLabel = computed(() => {
  const event = props.runtime.events.find((item) => item.kind === "tool.call.ready")
  const elapsed = event?.payload?.elapsedMs
  if (typeof elapsed !== "number") return null
  if (elapsed < 1000) return `首工具 ${elapsed}ms`
  return `首工具 ${(elapsed / 1000).toFixed(1)}s`
})

const statusLabel = computed(() => {
  switch (props.runtime.turn.status) {
    case "completed":
      return "已完成"
    case "cancelled":
      return "已中断"
    case "failed":
      return "失败"
    case "running":
      return "进行中"
    default:
      return props.runtime.turn.status
  }
})

const waitingLabel = computed(() => {
  const lastEvent = props.runtime.events[props.runtime.events.length - 1]
  if (!lastEvent) return "等待工具调用"
  return lastEvent.summary
})

const memoryRecalls = computed(() =>
  props.runtime.events
    .filter((event) => event.kind === "memory.recalled")
    .flatMap((event) => {
      const payload = event.payload || {}
      const hits = Array.isArray(payload.hits) ? payload.hits : []
      return hits
        .map((hit) => {
          if (!hit || typeof hit !== "object") return null
          const item = hit as Record<string, unknown>
          return {
            slug: typeof item.slug === "string" ? item.slug : "",
            title: typeof item.title === "string" ? item.title : "未命名记忆",
            folder: typeof item.folder === "string" ? item.folder : "notes",
            snippet: typeof item.snippet === "string" ? item.snippet : "",
          }
        })
        .filter(Boolean) as Array<{ slug: string, title: string, folder: string, snippet: string }>
    })
)

const executionFacts = computed(() =>
  props.runtime.events
    .filter((event) => event.kind === "execution.facts.recalled")
    .flatMap((event) => {
      const payload = event.payload || {}
      const facts = Array.isArray(payload.facts) ? payload.facts : []
      return facts
        .map((fact) => {
          if (!fact || typeof fact !== "object") return null
          const item = fact as Record<string, unknown>
          return {
            key: typeof item.key === "string" ? item.key : "",
            summary: typeof item.summary === "string" ? item.summary : "",
            sourceTool: typeof item.sourceTool === "string" ? item.sourceTool : "tool",
            sticky: item.sticky === true,
          }
        })
        .filter(Boolean) as Array<{ key: string, summary: string, sourceTool: string, sticky: boolean }>
    })
)

const contextPrepared = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "context.prepared")
)

const taskState = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "task.state.recalled")
)

const compactionRecalled = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "context.compaction.recalled")
)

const compactionCreated = computed(() =>
  props.runtime.events
    .filter((event) => event.kind === "context.compaction.created")
)

function sourceLabel(source: string) {
  switch (source) {
    case "system":
      return "system"
    case "tools":
      return "tools"
    case "current_user":
      return "user"
    case "history":
      return "history"
    case "memory":
      return "memory"
    case "task_state":
      return "task"
    case "recent_tool_receipts":
      return "receipts"
    case "execution_facts":
      return "facts"
    case "compressed_history":
      return "summary"
    case "artifacts":
      return "artifact"
    default:
      return source
  }
}

function eventPayloadList(value: unknown): string[] {
  return Array.isArray(value)
    ? value
      .filter((item): item is string => typeof item === "string" && item.trim().length > 0)
      .map((item) => item.trim())
    : []
}

const latestMemoryCandidate = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "memory.candidate.generated")
)

const latestMemorySaved = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "memory.saved")
)

const recentToolReceipts = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "tool.receipts.recalled")
)

const actionReferences = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "action.references.injected")
)

const memoryReferences = computed(() =>
  [...props.runtime.events]
    .reverse()
    .find((event) => event.kind === "memory.references.injected")
)

const hasContextSection = computed(() =>
  !!contextPrepared.value
  || !!taskState.value
  || !!compactionRecalled.value
  || compactionCreated.value.length > 0
  || executionFacts.value.length > 0
  || !!actionReferences.value
  || !!memoryReferences.value
  || !!recentToolReceipts.value
  || memoryRecalls.value.length > 0
  || !!latestMemoryCandidate.value
  || !!latestMemorySaved.value
)

const hasToolSection = computed(() =>
  props.runtime.tools.length > 0 || props.runtime.delegatedTurns.length > 0
)

function runtimeStatusLabel(status: string) {
  switch (status) {
    case "completed":
      return "已完成"
    case "cancelled":
      return "已中断"
    case "failed":
      return "失败"
    case "running":
      return "进行中"
    default:
      return status
  }
}

function runtimeElapsedLabel(startedAt: string, finishedAt?: string | null) {
  const started = Date.parse(startedAt)
  const finished = finishedAt ? Date.parse(finishedAt) : Date.now()
  if (Number.isNaN(started) || Number.isNaN(finished) || finished < started) {
    return null
  }
  const ms = finished - started
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function childWaitingLabel(runtime: RuntimeTurnView) {
  const lastEvent = runtime.events[runtime.events.length - 1]
  if (!lastEvent) return "等待工具调用"
  return lastEvent.summary
}
</script>

<template>
  <div class="runtime-summary">
    <button
      class="runtime-toggle"
      @click="
        expanded = !expanded;
        expandedState.set(runtime.turn.id, expanded)
      "
    >
      <div class="runtime-copy">
        <span class="runtime-title">Runtime</span>
        <span class="runtime-pill">{{ statusLabel }}</span>
        <span class="runtime-meta">{{ toolCount }} 个工具</span>
        <span v-if="delegationCount" class="runtime-meta">{{ delegationCount }} 次委派</span>
        <span v-if="elapsedLabel" class="runtime-meta">{{ elapsedLabel }}</span>
        <span v-if="ttftLabel" class="runtime-meta">{{ ttftLabel }}</span>
        <span v-if="firstToolLabel" class="runtime-meta">{{ firstToolLabel }}</span>
      </div>
      <span class="runtime-chevron">{{ expanded ? "▾" : "▸" }}</span>
    </button>

    <div v-if="expanded" class="runtime-body">
      <div v-if="hasContextSection" class="runtime-block">
        <button
          class="runtime-subtoggle"
          @click="
            contextExpanded = !contextExpanded;
            contextExpandedState.set(runtime.turn.id, contextExpanded)
          "
        >
          <div class="runtime-subcopy">
            <span class="runtime-section-title">上下文</span>
            <span
              v-if="contextPrepared"
              class="runtime-meta"
            >
              {{ Number(contextPrepared.payload?.historyMessages || 0) }} history ·
              {{ Number(contextPrepared.payload?.memoryHitCount || 0) }} memory ·
              {{ Number(contextPrepared.payload?.executionFactCount || 0) }} facts
            </span>
          </div>
          <span class="runtime-chevron">{{ contextExpanded ? "▾" : "▸" }}</span>
        </button>

        <div v-if="contextExpanded" class="runtime-subbody">
          <div v-if="contextPrepared" class="context-card">
            <div class="runtime-section-title">上下文来源</div>
            <div class="context-line">
              <span class="context-label">组成</span>
              <div class="context-pill-row">
                <span
                  v-for="source in Array.isArray(contextPrepared.payload?.sources) ? contextPrepared.payload.sources : []"
                  :key="String(source)"
                  class="context-pill"
                >
                  {{ sourceLabel(String(source)) }}
                </span>
              </div>
            </div>
            <div class="context-line">
              <span class="context-label">历史</span>
              <span>
                {{ Number(contextPrepared.payload?.historyMessages || 0) }} 条消息 /
                {{ Number(contextPrepared.payload?.historyTurns || 0) }} 个回合
              </span>
            </div>
            <div class="context-line">
              <span class="context-label">记忆</span>
              <span>{{ Number(contextPrepared.payload?.memoryHitCount || 0) }} 条</span>
            </div>
            <div class="context-line">
              <span class="context-label">事实</span>
              <span>
                {{ Number(contextPrepared.payload?.executionFactCount || 0) }} 条
                <template v-if="Number(contextPrepared.payload?.stickyExecutionFactCount || 0)">
                  · {{ Number(contextPrepared.payload?.stickyExecutionFactCount || 0) }} 条 sticky
                </template>
              </span>
            </div>
            <div class="context-line">
              <span class="context-label">artifact</span>
              <span>{{ Number(contextPrepared.payload?.artifactReferenceCount || 0) }} 个引用</span>
            </div>
            <div class="context-line">
              <span class="context-label">任务状态</span>
              <span>
                已完成 {{ Number(contextPrepared.payload?.taskCompletedCount || 0) }}
                · 待确认 {{ Number(contextPrepared.payload?.taskPendingConfirmationCount || 0) }}
                · 阻塞 {{ Number(contextPrepared.payload?.taskBlockerCount || 0) }}
              </span>
            </div>
            <div class="context-line">
              <span class="context-label">体积</span>
              <span>{{ Number(contextPrepared.payload?.approxContextBytes || 0) }} bytes · {{ Number(contextPrepared.payload?.toolCount || 0) }} 个工具定义</span>
            </div>
          </div>

          <div v-if="taskState" class="task-state-card">
            <div class="runtime-section-title">任务状态</div>
            <div v-if="taskState.payload?.lastUserGoal" class="task-state-line">
              <span class="task-state-label">最近目标</span>
              <span>{{ String(taskState.payload.lastUserGoal) }}</span>
            </div>
            <div v-if="taskState.payload?.lastAgentSummary" class="task-state-line">
              <span class="task-state-label">最近回复</span>
              <span>{{ String(taskState.payload.lastAgentSummary) }}</span>
            </div>
            <div v-if="Array.isArray(taskState.payload?.completed) && taskState.payload.completed.length" class="task-state-group">
              <span class="task-state-label">已完成</span>
              <div
                v-for="(item, index) in taskState.payload.completed"
                :key="`completed-${index}`"
                class="task-state-bullet"
              >
                {{ String(item) }}
              </div>
            </div>
            <div v-if="Array.isArray(taskState.payload?.pendingConfirmation) && taskState.payload.pendingConfirmation.length" class="task-state-group">
              <span class="task-state-label">待确认</span>
              <div
                v-for="(item, index) in taskState.payload.pendingConfirmation"
                :key="`pending-${index}`"
                class="task-state-bullet"
              >
                {{ String(item) }}
              </div>
            </div>
            <div v-if="Array.isArray(taskState.payload?.blockers) && taskState.payload.blockers.length" class="task-state-group">
              <span class="task-state-label">当前阻塞</span>
              <div
                v-for="(item, index) in taskState.payload.blockers"
                :key="`blocker-${index}`"
                class="task-state-bullet"
              >
                {{ String(item) }}
              </div>
            </div>
          </div>

          <div v-if="compactionRecalled || compactionCreated.length" class="compaction-list">
            <div class="runtime-section-title">历史压缩</div>

            <div v-if="compactionRecalled" class="compaction-card">
              <div class="compaction-top">
                <span class="compaction-badge">已注入</span>
                <span class="compaction-meta">
                  {{ Number(compactionRecalled.payload?.blockCount || 0) }} 个压缩块
                </span>
              </div>
              <div
                v-if="typeof compactionRecalled.payload?.preview === 'string' && compactionRecalled.payload.preview"
                class="compaction-summary"
              >
                {{ String(compactionRecalled.payload.preview) }}
              </div>
            </div>

            <div
              v-for="(event, index) in compactionCreated"
              :key="event.id || `compaction-${index}`"
              class="compaction-card"
            >
              <div class="compaction-top">
                <span class="compaction-badge created">新生成</span>
                <span class="compaction-meta">
                  覆盖 {{ Number(event.payload?.sourceStartIndex || 0) }}..{{ Number(event.payload?.sourceEndIndex || 0) }}
                </span>
                <span class="compaction-meta">
                  {{ Number(event.payload?.messageCount || 0) }} 条消息
                </span>
              </div>
              <div v-if="event.payload?.goal" class="compaction-line">
                <span class="compaction-label">目标</span>
                <span>{{ String(event.payload.goal) }}</span>
              </div>
              <div v-if="event.payload?.summaryText" class="compaction-summary">
                {{ String(event.payload.summaryText) }}
              </div>
              <div v-if="eventPayloadList(event.payload?.completed).length" class="compaction-group">
                <span class="compaction-label">已完成</span>
                <div
                  v-for="(item, itemIndex) in eventPayloadList(event.payload?.completed)"
                  :key="`completed-${index}-${itemIndex}`"
                  class="compaction-bullet"
                >
                  {{ item }}
                </div>
              </div>
              <div v-if="eventPayloadList(event.payload?.pending).length" class="compaction-group">
                <span class="compaction-label">待处理</span>
                <div
                  v-for="(item, itemIndex) in eventPayloadList(event.payload?.pending)"
                  :key="`pending-${index}-${itemIndex}`"
                  class="compaction-bullet"
                >
                  {{ item }}
                </div>
              </div>
              <div v-if="eventPayloadList(event.payload?.blockers).length" class="compaction-group">
                <span class="compaction-label">阻塞</span>
                <div
                  v-for="(item, itemIndex) in eventPayloadList(event.payload?.blockers)"
                  :key="`blocker-${index}-${itemIndex}`"
                  class="compaction-bullet"
                >
                  {{ item }}
                </div>
              </div>
              <div v-if="eventPayloadList(event.payload?.artifactRefs).length" class="compaction-group">
                <span class="compaction-label">artifact</span>
                <div
                  v-for="(item, itemIndex) in eventPayloadList(event.payload?.artifactRefs)"
                  :key="`artifact-${index}-${itemIndex}`"
                  class="compaction-bullet mono"
                >
                  {{ item }}
                </div>
              </div>
            </div>
          </div>

          <div v-if="executionFacts.length" class="fact-list">
            <div class="runtime-section-title">已验证事实</div>
            <div
              v-for="fact in executionFacts"
              :key="`${runtime.turn.id}-${fact.key}`"
              class="fact-card"
            >
              <div class="fact-top">
                <span class="fact-tool">{{ fact.sourceTool }}</span>
                <span v-if="fact.sticky" class="fact-badge">sticky</span>
              </div>
              <div class="fact-summary">{{ fact.summary }}</div>
            </div>
          </div>

          <div v-if="recentToolReceipts" class="memory-runtime-meta">
            <div class="memory-runtime-line">
              <span class="runtime-section-title inline">最近工具回执</span>
              <span>{{ recentToolReceipts.summary }}</span>
            </div>
            <div
              v-if="typeof recentToolReceipts.payload?.preview === 'string' && recentToolReceipts.payload.preview"
              class="compaction-summary"
            >
              {{ String(recentToolReceipts.payload.preview) }}
            </div>
          </div>

          <div v-if="actionReferences" class="memory-runtime-meta">
            <div class="memory-runtime-line">
              <span class="runtime-section-title inline">Action 引用</span>
              <span>{{ actionReferences.summary }}</span>
            </div>
            <div
              v-if="Array.isArray(actionReferences.payload?.actionIds) && actionReferences.payload?.actionIds.length"
              class="context-pill-row"
            >
              <span
                v-for="actionId in actionReferences.payload.actionIds"
                :key="String(actionId)"
                class="context-pill"
              >
                {{ String(actionId) }}
              </span>
            </div>
          </div>

          <div v-if="memoryReferences" class="memory-runtime-meta">
            <div class="memory-runtime-line">
              <span class="runtime-section-title inline">记忆引用</span>
              <span>{{ memoryReferences.summary }}</span>
            </div>
            <div
              v-if="Array.isArray(memoryReferences.payload?.memorySlugs) && memoryReferences.payload?.memorySlugs.length"
              class="context-pill-row"
            >
              <span
                v-for="slug in memoryReferences.payload.memorySlugs"
                :key="String(slug)"
                class="context-pill"
              >
                {{ String(slug) }}
              </span>
            </div>
          </div>

          <div v-if="memoryRecalls.length" class="memory-recall-list">
            <div class="runtime-section-title">记忆召回</div>
            <div
              v-for="memory in memoryRecalls"
              :key="`${runtime.turn.id}-${memory.slug}-${memory.title}`"
              class="memory-recall-card"
            >
              <div class="memory-recall-top">
                <span class="memory-folder">{{ memory.folder }}</span>
                <span class="memory-title-line">{{ memory.title }}</span>
              </div>
              <div class="memory-snippet">{{ memory.snippet }}</div>
            </div>
          </div>

          <div v-if="latestMemoryCandidate || latestMemorySaved" class="memory-runtime-meta">
            <div v-if="latestMemoryCandidate" class="memory-runtime-line">
              <span class="runtime-section-title inline">记忆候选</span>
              <span>{{ latestMemoryCandidate.summary }}</span>
            </div>
            <div v-if="latestMemorySaved" class="memory-runtime-line">
              <span class="runtime-section-title inline">知识库写入</span>
              <span>{{ latestMemorySaved.summary }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="runtime-block">
        <button
          class="runtime-subtoggle"
          @click="
            toolExpanded = !toolExpanded;
            toolExpandedState.set(runtime.turn.id, toolExpanded)
          "
        >
          <div class="runtime-subcopy">
            <span class="runtime-section-title">工具调用</span>
            <span class="runtime-meta">
              {{ toolCount }} 个工具<template v-if="delegationCount"> · {{ delegationCount }} 次委派</template>
            </span>
          </div>
          <span class="runtime-chevron">{{ toolExpanded ? "▾" : "▸" }}</span>
        </button>

        <div v-if="toolExpanded" class="runtime-subbody">
          <div v-if="runtime.delegatedTurns.length" class="delegate-turn-list">
            <div class="runtime-section-title">子 Agent</div>
            <div
              v-for="child in runtime.delegatedTurns"
              :key="child.turn.id"
              class="delegate-turn-card"
            >
              <div class="delegate-turn-head">
                <div class="delegate-turn-main">
                  <span class="delegate-turn-name">{{ child.turn.agentName || "子 Agent" }}</span>
                  <span class="delegate-turn-status">{{ runtimeStatusLabel(child.turn.status) }}</span>
                  <span v-if="runtimeElapsedLabel(child.turn.startedAt, child.turn.finishedAt)" class="delegate-turn-meta">
                    {{ runtimeElapsedLabel(child.turn.startedAt, child.turn.finishedAt) }}
                  </span>
                </div>
                <div class="delegate-turn-task">{{ child.turn.userContent }}</div>
              </div>

              <div v-if="child.tools.length" class="delegate-turn-tools">
                <ExecutionCard
                  v-for="(tool, index) in child.tools"
                  :key="`${child.turn.id}-${index}`"
                  :exec="tool"
                />
              </div>
              <div v-else class="delegate-turn-empty">{{ childWaitingLabel(child) }}</div>
            </div>
          </div>

          <div v-if="runtime.tools.length" class="runtime-tools">
            <ExecutionCard
              v-for="(tool, index) in runtime.tools"
              :key="`${runtime.turn.id}-${index}`"
              :exec="tool"
            />
          </div>
          <div v-else-if="!hasToolSection" class="runtime-empty">{{ waitingLabel }}</div>
          <div v-else class="runtime-empty">暂无直接工具输出</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.runtime-summary {
  margin-top: 10px;
  border: 1px solid rgba(var(--accent-rgb), 0.12);
  border-radius: 16px;
  background: rgba(var(--accent-rgb), 0.04);
  overflow: hidden;
  position: relative;
}

.runtime-summary::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(120deg, transparent 20%, rgba(255, 255, 255, 0.06) 50%, transparent 80%);
  transform: translateX(-120%);
  opacity: 0;
}

.runtime-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
  transition: background 160ms ease, transform 160ms ease;
}

.runtime-toggle:hover {
  background: rgba(var(--accent-rgb), 0.05);
  transform: translateY(-1px);
}

.runtime-summary:hover::before {
  opacity: 1;
  animation: runtimeSweep 1.6s ease;
}

.runtime-copy {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
}

.runtime-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-secondary);
}

.runtime-pill {
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(var(--accent-rgb), 0.12);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  font-size: 11px;
  color: var(--text-primary);
}

.runtime-meta {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.runtime-chevron {
  color: var(--text-tertiary);
  font-size: 10px;
  flex-shrink: 0;
  transition: transform 160ms ease;
}

.runtime-summary:hover .runtime-chevron {
  transform: translateX(1px);
}

.runtime-body {
  border-top: 1px solid rgba(var(--accent-rgb), 0.08);
  padding: 0 10px 10px;
  animation: runtimeBodyIn 0.2s var(--ease-out) both;
}

@keyframes runtimeSweep {
  from {
    transform: translateX(-120%);
  }
  to {
    transform: translateX(120%);
  }
}

@keyframes runtimeBodyIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.runtime-tools {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}

.runtime-block {
  margin-top: 10px;
  border: 1px solid rgba(var(--accent-rgb), 0.08);
  border-radius: 14px;
  background: rgba(var(--accent-rgb), 0.03);
  overflow: hidden;
}

.runtime-subtoggle {
  width: 100%;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: inherit;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  cursor: pointer;
  text-align: left;
}

.runtime-subtoggle:hover {
  background: rgba(var(--accent-rgb), 0.04);
}

.runtime-subcopy {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.runtime-subbody {
  border-top: 1px solid rgba(var(--accent-rgb), 0.08);
  padding: 0 10px 10px;
}

.runtime-section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.runtime-section-title.inline {
  min-width: 72px;
}

.context-card,
.compaction-list,
.fact-list,
.memory-recall-list {
  margin-top: 10px;
}

.context-card,
.compaction-card,
.task-state-card,
.fact-card,
.memory-recall-card,
.memory-runtime-meta {
  margin-top: 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--accent-rgb), 0.1);
  background: rgba(var(--accent-rgb), 0.04);
}

.task-state-line,
.context-line,
.compaction-top,
.compaction-line,
.fact-top,
.memory-recall-top,
.memory-runtime-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.fact-tool,
.fact-badge,
.memory-folder {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10px;
  font-family: var(--font-mono);
}

.fact-tool,
.memory-folder {
  background: rgba(var(--accent-rgb), 0.12);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.fact-badge {
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.14);
  color: var(--accent);
}

.context-label,
.compaction-label,
.task-state-label,
.fact-summary,
.memory-title-line {
  font-size: 12px;
  color: var(--text-primary);
}

.context-label,
.compaction-label,
.task-state-label {
  min-width: 68px;
  color: var(--text-secondary);
  font-weight: 600;
}

.compaction-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10px;
  font-family: var(--font-mono);
  background: rgba(var(--accent-rgb), 0.12);
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
}

.compaction-badge.created {
  background: rgba(var(--accent-rgb), 0.18);
}

.compaction-meta {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
}

.compaction-summary,
.compaction-bullet {
  margin-top: 8px;
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.55;
}

.compaction-group {
  margin-top: 8px;
}

.compaction-bullet.mono {
  font-family: var(--font-mono);
}

.context-pill-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.context-pill {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text-primary);
  font-size: 10px;
  font-family: var(--font-mono);
}

.task-state-group {
  margin-top: 8px;
}

.task-state-bullet {
  margin-top: 6px;
  color: var(--text-primary);
  font-size: 12px;
  line-height: 1.55;
}

.fact-summary {
  margin-top: 8px;
  line-height: 1.55;
}

.memory-snippet {
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.55;
}

.runtime-empty {
  padding: 12px 4px 2px;
  color: var(--text-tertiary);
  font-size: 12px;
}

.delegate-turn-list {
  margin-top: 10px;
}

.delegate-turn-card {
  margin-top: 8px;
  padding: 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--accent-rgb), 0.12);
  background: rgba(var(--accent-rgb), 0.05);
}

.delegate-turn-head {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.delegate-turn-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.delegate-turn-name {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-primary);
}

.delegate-turn-status {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--text-primary);
  font-size: 10px;
}

.delegate-turn-meta,
.delegate-turn-task,
.delegate-turn-empty {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.55;
}

.delegate-turn-meta {
  font-family: var(--font-mono);
  color: var(--text-tertiary);
}

.delegate-turn-tools {
  margin-top: 8px;
}

.delegate-turn-empty {
  margin-top: 8px;
}
</style>
