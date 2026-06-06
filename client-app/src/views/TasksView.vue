<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  AlertTriangle,
  Ban,
  CheckCircle,
  Clock,
  FileText,
  LoaderCircle,
  PlayCircle,
  RotateCcw,
  Server,
  Sparkles,
  XCircle,
} from '@lucide/vue'
import { useTaskStore, type Task, type TaskStatus } from '@/stores/task'
import StatusBadge from '@/components/StatusBadge.vue'

type FilterKey = 'all' | 'active' | 'completed' | 'failed'
type BadgeTone = InstanceType<typeof StatusBadge>['$props']['tone']
type DisplayTask = Task & {
  isDemo?: boolean
  progress?: number
  summary?: string
}

const router = useRouter()
const taskStore = useTaskStore()

const filter = ref<FilterKey>('all')

const demoTasks: DisplayTask[] = [
  {
    id: 'demo-running-image-batch',
    serverAlias: 'local-dev',
    serviceId: 'image-processing',
    serviceName: '图像处理工作站',
    title: '批量转换产品截图为 WebP',
    taskPrompt: '将 28 张产品截图统一压缩为 1440px 宽度的 WebP 文件。',
    outputPrompt: '输出处理后文件和压缩比统计。',
    status: 'running',
    createdAt: '2026-06-06T11:46:00Z',
    startedAt: '2026-06-06T11:48:00Z',
    files: [],
    isPolling: true,
    progress: 68,
    summary: '正在处理第 19 / 28 个文件，预计数分钟内完成。',
    isDemo: true,
  },
  {
    id: 'demo-pending-data-report',
    serverAlias: 'local-dev',
    serviceId: 'data-analysis',
    serviceName: '数据分析助手',
    title: '生成本周运营数据摘要',
    taskPrompt: '分析 CSV 数据并生成一页周报摘要。',
    outputPrompt: '结果写入 report.md，并附带关键指标表。',
    status: 'pending',
    createdAt: '2026-06-06T11:55:00Z',
    files: [],
    isPolling: true,
    progress: 12,
    summary: '任务已入队，等待可用 Agent 槽位。',
    isDemo: true,
  },
  {
    id: 'demo-completed-proofread',
    serverAlias: 'local-dev',
    serviceId: 'doc-proofreading',
    serviceName: '文档审校专家',
    title: '审校 OpenAaaS 客户端说明文档',
    taskPrompt: '检查中文 README 的术语一致性和表达清晰度。',
    outputPrompt: '列出修改建议并生成修订版 Markdown。',
    status: 'completed',
    createdAt: '2026-06-06T09:10:00Z',
    startedAt: '2026-06-06T09:12:00Z',
    completedAt: '2026-06-06T09:24:00Z',
    result: '完成',
    files: [],
    isPolling: false,
    progress: 100,
    summary: '已生成审校建议、术语统一表和修订版文档。',
    isDemo: true,
  },
  {
    id: 'demo-failed-review',
    serverAlias: 'local-dev',
    serviceId: 'code-review',
    serviceName: '代码审查助手',
    title: '审查上传的 TypeScript 组件',
    taskPrompt: '检查组件状态管理和可访问性问题。',
    outputPrompt: '输出问题列表和修复建议。',
    status: 'failed',
    createdAt: '2026-06-06T08:40:00Z',
    startedAt: '2026-06-06T08:42:00Z',
    completedAt: '2026-06-06T08:45:00Z',
    files: [],
    errorMessage: '服务权限不足，无法读取上传附件。',
    isPolling: false,
    progress: 38,
    summary: '执行中断：附件读取权限不足。',
    isDemo: true,
  },
  {
    id: 'demo-cancelling-ocr',
    serverAlias: 'local-dev',
    serviceId: 'image-processing',
    serviceName: '图像处理工作站',
    title: '取消 OCR 批处理任务',
    taskPrompt: '识别多语言截图中的文字。',
    outputPrompt: '输出识别文本和置信度。',
    status: 'cancelling',
    createdAt: '2026-06-06T11:20:00Z',
    startedAt: '2026-06-06T11:21:00Z',
    files: [],
    isPolling: true,
    progress: 54,
    summary: '正在通知 Agent 停止当前批处理。',
    isDemo: true,
  },
  {
    id: 'demo-cancelled-chart',
    serverAlias: 'local-dev',
    serviceId: 'data-analysis',
    serviceName: '数据分析助手',
    title: '取消季度趋势图生成',
    taskPrompt: '根据 Excel 文件生成季度趋势图。',
    outputPrompt: '输出图表和摘要。',
    status: 'cancelled',
    createdAt: '2026-06-06T07:30:00Z',
    startedAt: '2026-06-06T07:33:00Z',
    completedAt: '2026-06-06T07:36:00Z',
    files: [],
    isPolling: false,
    progress: 22,
    summary: '用户主动取消，未生成最终输出。',
    isDemo: true,
  },
]

const isDemoMode = computed(() => import.meta.env.DEV && taskStore.tasks.length === 0)

const sourceTasks = computed<DisplayTask[]>(() => {
  if (isDemoMode.value) return demoTasks
  return taskStore.tasks
})

const sortedTasks = computed(() =>
  [...sourceTasks.value].sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()),
)

const filteredTasks = computed(() => {
  if (filter.value === 'active') {
    return sortedTasks.value.filter((t) => ['pending', 'running', 'cancelling'].includes(t.status))
  }
  if (filter.value === 'completed') {
    return sortedTasks.value.filter((t) => t.status === 'completed')
  }
  if (filter.value === 'failed') {
    return sortedTasks.value.filter((t) => t.status === 'failed' || t.status === 'cancelled')
  }
  return sortedTasks.value
})

const filterItems = computed(() => {
  const tasks = sortedTasks.value
  return [
    { key: 'all' as const, label: '全部', count: tasks.length },
    { key: 'active' as const, label: '进行中', count: tasks.filter((t) => ['pending', 'running', 'cancelling'].includes(t.status)).length },
    { key: 'completed' as const, label: '已完成', count: tasks.filter((t) => t.status === 'completed').length },
    { key: 'failed' as const, label: '异常/取消', count: tasks.filter((t) => t.status === 'failed' || t.status === 'cancelled').length },
  ]
})

const statusSummary = computed(() => ({
  active: sortedTasks.value.filter((t) => ['pending', 'running', 'cancelling'].includes(t.status)).length,
  completed: sortedTasks.value.filter((t) => t.status === 'completed').length,
  problem: sortedTasks.value.filter((t) => t.status === 'failed' || t.status === 'cancelled').length,
}))

function statusMeta(status: TaskStatus): {
  label: string
  tone: BadgeTone
  icon: typeof Clock
  accentClass: string
  progressClass: string
} {
  const map: Record<TaskStatus, {
    label: string
    tone: BadgeTone
    icon: typeof Clock
    accentClass: string
    progressClass: string
  }> = {
    pending: {
      label: '待处理',
      tone: 'accent',
      icon: Clock,
      accentClass: 'bg-accent-soft text-accent border-accent/20',
      progressClass: 'bg-accent',
    },
    running: {
      label: '运行中',
      tone: 'info',
      icon: PlayCircle,
      accentClass: 'bg-info-soft text-info border-info/20',
      progressClass: 'bg-info',
    },
    completed: {
      label: '已完成',
      tone: 'success',
      icon: CheckCircle,
      accentClass: 'bg-success/10 text-success border-success/20',
      progressClass: 'bg-success',
    },
    failed: {
      label: '失败',
      tone: 'danger',
      icon: XCircle,
      accentClass: 'bg-danger/10 text-danger border-danger/20',
      progressClass: 'bg-danger',
    },
    cancelled: {
      label: '已取消',
      tone: 'neutral',
      icon: Ban,
      accentClass: 'bg-bg-secondary text-text-muted border-border',
      progressClass: 'bg-text-muted',
    },
    cancelling: {
      label: '取消中',
      tone: 'secondary',
      icon: LoaderCircle,
      accentClass: 'bg-secondary-soft text-secondary border-secondary/25',
      progressClass: 'bg-secondary',
    },
  }
  return map[status]
}

function formatTime(iso?: string): string {
  if (!iso) return '-'
  const d = new Date(iso)
  if (isNaN(d.getTime())) return '-'
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

function durationText(task: DisplayTask): string {
  const startIso = task.startedAt || task.createdAt
  if (!startIso) return '-'
  const start = new Date(startIso).getTime()
  if (isNaN(start)) return '-'
  const end = task.completedAt ? new Date(task.completedAt).getTime() : Date.now()
  const sec = Math.max(0, Math.floor((end - start) / 1000))
  if (sec < 60) return `${sec}秒`
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min}分${sec % 60}秒`
  const h = Math.floor(min / 60)
  return `${h}小时${min % 60}分`
}

function progressValue(task: DisplayTask): number {
  if (task.progress != null) return task.progress
  if (task.status === 'completed') return 100
  if (task.status === 'pending') return 10
  if (task.status === 'running') return 62
  if (task.status === 'cancelling') return 48
  if (task.status === 'failed' || task.status === 'cancelled') return 36
  return 0
}

function openTask(task: DisplayTask) {
  if (task.isDemo) return
  router.push(`/task/${task.id}`)
}
</script>

<template>
  <div class="max-w-6xl mx-auto">
    <div class="mb-6 flex flex-wrap items-start justify-between gap-4">
      <div>
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-sm font-semibold text-accent">OpenAaaS Tasks</p>
          <StatusBadge v-if="isDemoMode" label="示例数据" tone="secondary" compact />
        </div>
        <h1 class="mt-1 text-3xl font-bold text-info">任务列表</h1>
        <p class="mt-2 text-sm text-text-secondary">
          跟踪任务排队、运行、完成和异常状态，快速判断下一步操作。
        </p>
      </div>

      <div class="grid grid-cols-3 gap-2">
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">进行中</p>
          <p class="mt-1 text-xl font-bold text-info">{{ statusSummary.active }}</p>
        </div>
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">已完成</p>
          <p class="mt-1 text-xl font-bold text-success">{{ statusSummary.completed }}</p>
        </div>
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">异常/取消</p>
          <p class="mt-1 text-xl font-bold text-secondary">{{ statusSummary.problem }}</p>
        </div>
      </div>
    </div>

    <div class="mb-6 inline-flex rounded-lg border border-border bg-bg-card p-1 shadow-sm">
      <button
        v-for="f in filterItems"
        :key="f.key"
        type="button"
        class="inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-semibold transition-colors"
        :class="
          filter === f.key
            ? 'bg-accent text-white shadow-sm'
            : 'text-text-secondary hover:bg-accent-soft hover:text-accent'
        "
        @click="filter = f.key"
      >
        {{ f.label }}
        <span
          class="rounded-full px-1.5 py-0.5 text-[10px] leading-none"
          :class="filter === f.key ? 'bg-white/20 text-white' : 'bg-bg-secondary text-text-muted'"
        >
          {{ f.count }}
        </span>
      </button>
    </div>

    <div v-if="filteredTasks.length === 0" class="rounded-lg border border-border bg-bg-card py-20 text-center text-text-muted shadow-sm">
      <FileText class="mx-auto mb-3 h-8 w-8 text-text-muted" aria-hidden="true" />
      <p class="text-sm">暂无任务</p>
    </div>

    <div v-else class="space-y-3">
      <button
        v-for="task in filteredTasks"
        :key="task.id"
        type="button"
        class="group w-full rounded-lg border border-border bg-bg-card p-4 text-left shadow-sm transition-all hover:-translate-y-0.5 hover:border-accent/35 hover:shadow-soft"
        :class="{
          'cursor-default hover:translate-y-0': task.isDemo,
          'border-secondary/25 bg-secondary-soft/60': task.status === 'failed' || task.status === 'cancelling',
        }"
        @click="openTask(task)"
      >
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div class="flex min-w-0 flex-1 gap-3">
            <div
              class="flex flex-shrink-0 items-center justify-center rounded-lg border"
              :class="statusMeta(task.status).accentClass"
              style="height: 44px; width: 44px;"
            >
              <component
                :is="statusMeta(task.status).icon"
                class="h-5 w-5"
                :class="{ 'animate-spin': task.status === 'cancelling' }"
                :stroke-width="2.25"
                aria-hidden="true"
              />
            </div>

            <div class="min-w-0 flex-1">
              <div class="mb-2 flex flex-wrap items-center gap-2">
                <h3 class="truncate text-base font-bold text-info">{{ task.title }}</h3>
                <StatusBadge
                  :label="statusMeta(task.status).label"
                  :tone="statusMeta(task.status).tone"
                  compact
                  show-icon
                />
              </div>
              <p class="mb-3 text-sm leading-6 text-text-secondary">
                {{ task.summary || task.taskPrompt }}
              </p>

              <div class="flex flex-wrap gap-2 text-xs text-text-muted">
                <div class="inline-flex max-w-full items-center gap-1.5 rounded-lg border border-border bg-bg-inset px-2.5 py-2">
                  <Server class="h-3.5 w-3.5 flex-shrink-0" aria-hidden="true" />
                  <span class="truncate">{{ task.serviceName }}</span>
                </div>
                <div class="rounded-lg border border-border bg-bg-inset px-2.5 py-2">
                  创建: {{ formatTime(task.createdAt) }}
                </div>
                <div class="rounded-lg border border-border bg-bg-inset px-2.5 py-2">
                  耗时: {{ durationText(task) }}
                </div>
                <div class="rounded-lg border border-border bg-bg-inset px-2.5 py-2">
                  文件: {{ task.files.length }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap items-end justify-between gap-3">
          <div class="min-w-[220px] flex-1">
            <div class="mb-2 flex items-center justify-between text-xs font-semibold text-text-muted">
              <span>进度</span>
              <span>{{ progressValue(task) }}%</span>
            </div>
            <div class="h-2 overflow-hidden rounded-full bg-bg-tertiary">
              <div
                class="h-full rounded-full transition-all"
                :class="statusMeta(task.status).progressClass"
                :style="{ width: `${progressValue(task)}%` }"
              />
            </div>
          </div>
          <div class="flex items-center justify-end gap-2 text-xs font-semibold">
            <StatusBadge v-if="task.isDemo" label="Demo" tone="neutral" compact />
            <span
              v-if="!task.isDemo"
              class="inline-flex items-center gap-1 text-accent opacity-0 transition-opacity group-hover:opacity-100"
            >
              查看详情
              <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
            </span>
          </div>
        </div>

        <div
          v-if="task.errorMessage || task.pollError"
          class="mt-3 flex items-center gap-2 rounded-lg border border-danger/20 bg-danger/5 px-3 py-2 text-xs font-semibold text-danger"
        >
          <AlertTriangle class="h-4 w-4 flex-shrink-0" aria-hidden="true" />
          {{ task.errorMessage || task.pollError }}
        </div>
        <div
          v-else-if="task.status === 'pending' || task.status === 'running' || task.status === 'cancelling'"
          class="mt-3 flex items-center gap-2 rounded-lg border border-accent/15 bg-accent-soft px-3 py-2 text-xs font-semibold text-accent"
        >
          <RotateCcw class="h-4 w-4 flex-shrink-0" aria-hidden="true" />
          {{ task.isPolling ? '正在同步任务状态' : '等待下一次状态同步' }}
        </div>
      </button>
    </div>
  </div>
</template>
