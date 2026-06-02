<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { computed, onBeforeUnmount, onMounted, ref } from "vue"
import ThemeSelect from "../components/ThemeSelect.vue"

interface WechatBridgeConfig {
  agentId?: string | null
  llmProfileId?: string | null
  bridgeToken: string
}

interface AgentOption {
  id: string
  name: string
  role: string
}

interface LlmProfile {
  id: string
  name: string
  provider_type: string
  model: string
}

interface AppConfig {
  llm_profiles: LlmProfile[]
}

interface ExternalSessionSummary {
  id: string
  name: string
  category: string
  updatedAt: string
}

interface ExternalRouteStateView {
  routeKey: string
  provider: string
  accountId: string
  chatType: string
  peerId: string
  peerName?: string | null
  activeSessionId?: string | null
  activeSessionName?: string | null
  sessionIds: string[]
  sessions: ExternalSessionSummary[]
  updatedAt: string
}

interface WechatBridgeView {
  config: WechatBridgeConfig
  bridgeRunning: boolean
  bridgeLastError?: string | null
  latestQrContent?: string | null
  latestQrUpdatedAt?: string | null
  loginStatus?: string | null
  connected: boolean
  connectedAccountId?: string | null
  connectedUserId?: string | null
  qrSessionKey?: string | null
  qrError?: string | null
}

interface BridgeEvent {
  kind?: string
  content?: string
  updatedAt?: string
  status?: string
  error?: string | null
  accountId?: string | null
  userId?: string | null
}

const loading = ref(false)
const saving = ref(false)
const statusMsg = ref("")
const agents = ref<AgentOption[]>([])
const llmProfiles = ref<LlmProfile[]>([])
const routeStates = ref<ExternalRouteStateView[]>([])
const qrContent = ref("")
const qrSvg = ref("")
const qrMsg = ref("")
const qrGenerating = ref(false)
const bridgeRunning = ref(false)
const bridgeLastError = ref("")
const latestQrUpdatedAt = ref("")
const loginStatus = ref("")
const connected = ref(false)
const connectedAccountId = ref("")
const connectedUserId = ref("")
const qrError = ref("")

const draft = ref<WechatBridgeConfig>({
  agentId: null,
  llmProfileId: null,
  bridgeToken: "ohmywu-local-bridge",
})

let unlistenBridge: UnlistenFn | null = null

const agentOptions = computed(() => [
  { label: "跟随主对话 Agent", value: "__none__" },
  ...agents.value.map((agent) => ({
    label: `${agent.name} · ${agent.role}`,
    value: agent.id,
  })),
])

const llmProfileOptions = computed(() => [
  { label: "跟随当前全局模型", value: "__none__" },
  ...llmProfiles.value.map((profile) => ({
    label: `${profile.name} · ${profile.provider_type || "custom"} · ${profile.model || "未设置模型"}`,
    value: profile.id,
  })),
])

const routeCountLabel = computed(() => `${routeStates.value.length} 个微信会话`)
const bridgeStatusLabel = computed(() => (bridgeRunning.value ? "通道监听中" : "通道未就绪"))
const qrUpdatedLabel = computed(() =>
  latestQrUpdatedAt.value ? `最近二维码更新时间：${latestQrUpdatedAt.value}` : "还没有收到二维码",
)
const loginStatusLabel = computed(() => {
  if (connected.value) return "已登录"
  if (loginStatus.value === "confirmed") return "已确认登录"
  if (loginStatus.value === "expired") return "二维码已过期"
  if (loginStatus.value === "cancel" || loginStatus.value === "canceled" || loginStatus.value === "denied") return "登录已取消"
  if (loginStatus.value) return `等待扫码 · ${loginStatus.value}`
  return "未发起登录"
})

async function loadRouteStates() {
  routeStates.value = await invoke<ExternalRouteStateView[]>("list_external_route_states", {
    provider: "wechat",
  })
}

async function refreshQrSvg(preferredContent?: string) {
  const payload = (preferredContent || qrContent.value).trim()
  if (!payload) {
    qrSvg.value = ""
    qrMsg.value = "还没有可渲染的二维码内容。等待微信适配器推送，或手动粘贴内容。"
    return
  }

  qrGenerating.value = true
  qrMsg.value = ""
  try {
    qrSvg.value = await invoke<string>("render_qr_svg", { content: payload })
    qrContent.value = payload
    qrMsg.value = "二维码已刷新"
  } catch (error) {
    qrSvg.value = ""
    qrMsg.value = String(error)
  } finally {
    qrGenerating.value = false
  }
}

async function requestLoginQr() {
  qrGenerating.value = true
  qrMsg.value = ""
  try {
    const view = await invoke<WechatBridgeView>("request_wechat_login_qr")
    loginStatus.value = view.loginStatus || "wait"
    qrError.value = view.qrError || ""
    const latestContent = (view.latestQrContent || "").trim()
    if (latestContent) {
      qrContent.value = latestContent
      await refreshQrSvg(latestContent)
    }
  } catch (error) {
    qrMsg.value = String(error)
  } finally {
    qrGenerating.value = false
  }
}

async function loadView() {
  loading.value = true
  try {
    const [view, agentList, appConfig] = await Promise.all([
      invoke<WechatBridgeView>("get_wechat_bridge"),
      invoke<AgentOption[]>("get_agents"),
      invoke<AppConfig>("get_config"),
      loadRouteStates(),
    ])
    draft.value = {
      ...view.config,
      agentId: view.config.agentId ?? null,
      llmProfileId: view.config.llmProfileId ?? null,
    }
    bridgeRunning.value = view.bridgeRunning
    bridgeLastError.value = view.bridgeLastError || ""
    latestQrUpdatedAt.value = view.latestQrUpdatedAt || ""
    loginStatus.value = view.loginStatus || ""
    connected.value = view.connected
    connectedAccountId.value = view.connectedAccountId || ""
    connectedUserId.value = view.connectedUserId || ""
    qrError.value = view.qrError || ""
    agents.value = agentList
    llmProfiles.value = appConfig.llm_profiles || []

    const latestContent = (view.latestQrContent || "").trim()
    if (latestContent) {
      qrContent.value = latestContent
      await refreshQrSvg(latestContent)
    } else {
      qrContent.value = ""
      qrSvg.value = ""
    }
  } catch (error) {
    statusMsg.value = String(error)
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  saving.value = true
  statusMsg.value = ""
  try {
    await invoke("save_wechat_bridge_config", {
      config: draft.value,
    })
    statusMsg.value = "微信接入配置已保存"
    await loadView()
  } catch (error) {
    statusMsg.value = String(error)
  } finally {
    saving.value = false
  }
}

function updateAgentId(value: string | number) {
  const next = String(value)
  draft.value.agentId = next === "__none__" ? null : next
}

function updateLlmProfileId(value: string | number) {
  const next = String(value)
  draft.value.llmProfileId = next === "__none__" ? null : next
}

onMounted(async () => {
  await loadView()
  unlistenBridge = await listen<BridgeEvent>("wechat-bridge-event", async (event) => {
    const payload = event.payload || {}
    if (payload.kind === "qr.updated") {
      qrContent.value = (payload.content || "").trim()
      latestQrUpdatedAt.value = payload.updatedAt || ""
      await refreshQrSvg(qrContent.value)
      return
    }
    if (payload.kind === "qr.status") {
      loginStatus.value = payload.status || ""
      qrError.value = payload.error || ""
      connectedAccountId.value = payload.accountId || ""
      connectedUserId.value = payload.userId || ""
      if (payload.status === "confirmed") {
        connected.value = true
      }
      return
    }
    if (payload.kind === "qr.error") {
      qrError.value = payload.error || "微信登录状态轮询失败"
      return
    }
    if (payload.kind === "bridge.inbound") {
      await loadRouteStates()
    }
  })
})

onBeforeUnmount(() => {
  if (unlistenBridge) {
    unlistenBridge()
    unlistenBridge = null
  }
})
</script>

<template>
  <div class="wechat-view">
    <header class="section-head">
      <div>
        <h2 class="view-title">微信接入</h2>
        <p class="view-subtitle">这里管理 OhMyWu 自己的微信通道：微信会话映射、专属 Agent、专属模型，以及二维码展示。</p>
      </div>
      <div class="header-side">
        <span class="status-chip">{{ routeCountLabel }}</span>
        <span :class="['status-chip', 'status-tone', { offline: !bridgeRunning }]">{{ bridgeStatusLabel }}</span>
        <span :class="['status-chip', { 'status-tone': connected }]">{{ loginStatusLabel }}</span>
      </div>
    </header>

    <section v-if="bridgeLastError" class="alert-card">
      {{ bridgeLastError }}
    </section>

    <section class="wechat-grid">
      <article class="panel">
        <div class="panel-head">
          <div>
            <h3 class="panel-title">对话管理</h3>
            <p class="panel-subtitle">同一个微信联系人或群里，用 `/new` 可以切出新的本地对话。下面显示当前已经映射的微信会话。</p>
          </div>
        </div>

        <div v-if="routeStates.length" class="route-list">
          <div v-for="route in routeStates" :key="route.routeKey" class="route-card">
            <div class="route-top">
              <div>
                <div class="route-name">{{ route.peerName || route.peerId }}</div>
                <div class="route-meta">
                  {{ route.chatType === "group" ? "群聊" : "私聊" }} · {{ route.accountId }}
                </div>
              </div>
              <span class="route-pill">{{ route.sessions.length }} 个对话</span>
            </div>

            <div class="route-active">
              当前激活：
              <strong>{{ route.activeSessionName || route.activeSessionId || "未建立" }}</strong>
            </div>

            <div v-if="route.sessions.length" class="session-chip-row">
              <span
                v-for="session in route.sessions"
                :key="session.id"
                :class="['session-chip', { active: session.id === route.activeSessionId }]"
              >
                {{ session.name }}
              </span>
            </div>
          </div>
        </div>

        <div v-else class="empty-state">当前还没有微信对话映射。等第一条微信消息进来，或在微信里执行一次 `/new` 后，这里会出现记录。</div>
      </article>

      <article class="panel">
        <div class="panel-head">
          <div>
            <h3 class="panel-title">接入分配</h3>
            <p class="panel-subtitle">给整条微信通道指定专属 Agent 和专属模型，不影响主界面的默认对话。</p>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">专属 Agent</label>
          <ThemeSelect
            :model-value="draft.agentId || '__none__'"
            :options="agentOptions"
            @update:model-value="updateAgentId"
          />
        </div>

        <div class="field-group">
          <label class="field-label">专属模型配置</label>
          <ThemeSelect
            :model-value="draft.llmProfileId || '__none__'"
            :options="llmProfileOptions"
            @update:model-value="updateLlmProfileId"
          />
        </div>

        <div class="field-group">
          <label class="field-label">桥接令牌</label>
          <input
            v-model="draft.bridgeToken"
            class="field-input"
            type="text"
            placeholder="用于微信适配器访问 OhMyWu 本地桥接服务"
          />
        </div>

        <div class="card-actions">
          <button class="primary-btn" type="button" :disabled="saving || loading" @click="saveConfig">
            {{ saving ? "保存中..." : "保存分配" }}
          </button>
          <span v-if="statusMsg" class="msg">{{ statusMsg }}</span>
        </div>
      </article>

      <article class="panel">
        <div class="panel-head">
          <div>
            <h3 class="panel-title">二维码内容</h3>
            <p class="panel-subtitle">由 OhMyWu 主动向微信接口申请登录二维码。扫码确认后，这里会自动更新状态。</p>
          </div>
        </div>

        <div class="card-actions compact">
          <button class="primary-btn" type="button" :disabled="qrGenerating" @click="requestLoginQr">
            {{ qrGenerating ? "生成中..." : "生成登录二维码" }}
          </button>
          <span class="msg">{{ loginStatusLabel }}</span>
        </div>

        <div v-if="connected" class="refresh-msg">
          已登录账号：{{ connectedAccountId || "未知账号" }}<span v-if="connectedUserId"> · {{ connectedUserId }}</span>
        </div>

        <div class="field-group">
          <label class="field-label">登录链接 / 二维码文本</label>
          <textarea
            v-model="qrContent"
            class="field-input multiline"
            rows="4"
            placeholder="等待微信适配器自动推送，或手动粘贴二维码内容。"
          />
        </div>

        <div class="card-actions compact">
          <button class="primary-btn" type="button" :disabled="qrGenerating" @click="refreshQrSvg()">
            {{ qrGenerating ? "刷新中..." : "刷新二维码展示" }}
          </button>
          <span class="msg">{{ qrUpdatedLabel }}</span>
        </div>
        <div v-if="qrError" class="refresh-msg">{{ qrError }}</div>
      </article>

      <article class="panel">
        <div class="panel-head">
          <div>
            <h3 class="panel-title">二维码展示</h3>
            <p class="panel-subtitle">这里显示当前可扫码的真实二维码。</p>
          </div>
        </div>

        <div v-if="qrSvg" class="qr-preview">
          <div class="qr-stage qr-svg" v-html="qrSvg" />
        </div>
        <div v-else class="empty-state">还没有可展示的二维码。等待适配器推送二维码，或手动填入二维码内容。</div>

        <div v-if="qrMsg" class="refresh-msg">{{ qrMsg }}</div>
      </article>
    </section>
  </div>
</template>

<style scoped>
.wechat-view {
  padding: 28px 32px 32px;
  width: 100%;
  max-width: 1120px;
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.section-head,
.header-side,
.panel-head,
.card-actions,
.route-top,
.session-chip-row {
  display: flex;
  gap: 12px;
}

.section-head,
.panel-head,
.route-top {
  justify-content: space-between;
  align-items: flex-start;
}

.header-side {
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.view-title {
  margin: 0 0 6px;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.view-subtitle,
.panel-subtitle,
.route-meta,
.route-active,
.msg,
.empty-state {
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.6;
}

.alert-card {
  padding: 14px 16px;
  border-radius: 18px;
  border: 1px solid rgba(255, 127, 102, 0.24);
  background: rgba(255, 127, 102, 0.08);
  color: var(--text-primary);
}

.wechat-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
  align-items: start;
}

.panel {
  padding: 22px 24px;
  border-radius: 22px;
  border: 1px solid var(--border-color);
  background: var(--panel-bg);
  box-shadow: var(--shadow-surface);
}

.panel-title {
  margin: 0 0 4px;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.status-chip,
.route-pill,
.session-chip {
  display: inline-flex;
  align-items: center;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--panel-bg);
  color: var(--text-secondary);
  font-size: 11px;
  font-family: var(--font-mono);
}

.status-tone {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--text-primary);
}

.status-tone.offline {
  border-color: rgba(255, 127, 102, 0.24);
  background: rgba(255, 127, 102, 0.08);
}

.route-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.route-card {
  padding: 16px;
  border-radius: 18px;
  border: 1px solid var(--border-color);
  background: var(--panel-bg);
}

.route-name {
  color: var(--text-primary);
  font-size: 15px;
  font-weight: 700;
}

.route-meta {
  margin-top: 4px;
  font-family: var(--font-mono);
  font-size: 12px;
}

.route-active {
  margin-top: 12px;
}

.route-active strong {
  color: var(--text-primary);
}

.session-chip-row {
  margin-top: 12px;
  flex-wrap: wrap;
}

.session-chip.active {
  border-color: rgba(var(--accent-rgb), 0.22);
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--text-primary);
}

.field-group {
  margin-top: 16px;
}

.field-label {
  display: block;
  margin-bottom: 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.field-input {
  width: 100%;
  min-height: 42px;
  padding: 0 14px;
  border-radius: 14px;
  border: 1px solid var(--border-color);
  background: var(--control-bg);
  color: var(--text-primary);
}

.field-input.multiline {
  min-height: 108px;
  padding: 12px 14px;
  resize: vertical;
}

.card-actions {
  align-items: center;
  flex-wrap: wrap;
  margin-top: 16px;
}

.card-actions.compact {
  margin-top: 12px;
}

.primary-btn {
  padding: 9px 14px;
  border-radius: 12px;
  border: 1px solid rgba(var(--accent-rgb), 0.26);
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--text-primary);
  font: inherit;
  cursor: pointer;
}

.primary-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.qr-preview {
  display: flex;
  justify-content: center;
  margin-top: 10px;
}

.qr-stage {
  width: min(320px, 100%);
  padding: 18px;
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.98);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.12);
}

.qr-svg :deep(svg) {
  width: 100%;
  height: auto;
  display: block;
}

.refresh-msg {
  margin-top: 14px;
  padding: 10px 12px;
  border-radius: 14px;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
}

.empty-state {
  margin-top: 12px;
}

@media (max-width: 960px) {
  .wechat-view {
    padding: 20px 18px 24px;
  }

  .wechat-grid {
    grid-template-columns: 1fr;
  }

  .section-head,
  .panel-head,
  .route-top {
    flex-direction: column;
  }

  .header-side {
    justify-content: flex-start;
  }
}
</style>
