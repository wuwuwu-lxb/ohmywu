export type ToolRisk = "ReadOnly" | "ControlledWrite" | "HighRisk"
export type ToolStatus = "running" | "success" | "failed" | "denied" | "needs_confirm"

export interface CapabilityInfo {
  name: string
  description: string
  risk_level: ToolRisk
}

export interface ToolMeta {
  label: string
  short: string
  detail: string
  example?: string
}

const TOOL_META: Record<string, ToolMeta> = {
  bash: {
    label: "终端命令",
    short: "执行 shell 命令",
    detail: "可运行构建、测试、查询系统状态等命令，能力最强，风险也最高。",
    example: "ls -la /mnt/data/workspace/project/ohmywu",
  },
  read: {
    label: "读取文件",
    short: "读取本地文件内容",
    detail: "只读工具，用于查看代码、配置、文档，不会改动文件。",
    example: "/mnt/data/workspace/project/ohmywu/README.md",
  },
  write: {
    label: "写入文件",
    short: "创建或覆盖文件",
    detail: "直接写入完整文件内容，适合生成新文件或整体替换内容。",
    example: "/mnt/data/workspace/project/ohmywu/plan.md",
  },
  edit: {
    label: "精确编辑",
    short: "按片段替换文件内容",
    detail: "基于唯一匹配文本做局部修改，适合小范围修复和增量更新。",
    example: "/mnt/data/workspace/project/ohmywu/src/views/SettingsView.vue",
  },
  glob: {
    label: "文件搜索",
    short: "按模式查找文件",
    detail: "适合快速找文件、目录和匹配模式的资源。",
    example: "**/*.vue",
  },
  grep: {
    label: "内容搜索",
    short: "在文件里查文本",
    detail: "适合找变量、函数、文案和配置片段。",
    example: "reasoning_content",
  },
  web_fetch: {
    label: "网页读取",
    short: "抓取指定 URL 内容",
    detail: "用于读取外部网页或文档内容，本地不写入文件。",
    example: "https://example.com",
  },
  thinking: {
    label: "思考记录",
    short: "写出中间推理",
    detail: "只记录计划和推理，不会执行外部操作。",
    example: "拆解下一步任务",
  },
  checklist_write: {
    label: "任务清单",
    short: "生成当前执行清单",
    detail: "把当前回合的步骤写成 checklist，便于前端展示过程。",
    example: "前端改造步骤",
  },
  wiki_read: {
    label: "知识库读取",
    short: "读取指定 wiki 条目",
    detail: "从本地知识库中读取一篇笔记或文档。",
    example: "project-roadmap",
  },
  wiki_write: {
    label: "知识库写入",
    short: "新建或更新 wiki 条目",
    detail: "把阶段结论、经验和结构化知识写入本地知识库。",
    example: "agent-memory-design",
  },
  wiki_search: {
    label: "知识库搜索",
    short: "按关键词搜索 wiki",
    detail: "从本地知识库里找相关条目和上下文。",
    example: "permission",
  },
  wiki_list: {
    label: "知识库列表",
    short: "列出全部 wiki 条目",
    detail: "查看当前知识库里已有的所有笔记。",
    example: "全部条目",
  },
  wiki_graph: {
    label: "知识图谱",
    short: "查看 wiki 关联图",
    detail: "返回知识节点和关联边，适合后续做记忆与关系视图。",
    example: "条目关联图",
  },
}

export function getToolMeta(name: string): ToolMeta {
  return (
    TOOL_META[name] || {
      label: name,
      short: "工具调用",
      detail: "当前工具尚未补充前端说明。",
    }
  )
}

export function toolRiskLabel(risk: ToolRisk): string {
  switch (risk) {
    case "ReadOnly":
      return "只读"
    case "ControlledWrite":
      return "受控写入"
    case "HighRisk":
      return "高风险"
    default:
      return risk
  }
}

export function toolStatusLabel(status: ToolStatus): string {
  switch (status) {
    case "running":
      return "进行中"
    case "success":
      return "成功"
    case "failed":
      return "失败"
    case "denied":
      return "已拒绝"
    case "needs_confirm":
      return "待确认"
    default:
      return status
  }
}
