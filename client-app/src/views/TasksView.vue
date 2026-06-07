<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  AlertTriangle,
  CheckCircle,
  Clock,
  FileText,
  PlayCircle,
  Server,
  Sparkles,
  XCircle,
} from '@lucide/vue'
import { useTaskStore, type Task, type TaskStatus } from '@/stores/task'
import StatusBadge from '@/components/StatusBadge.vue'

type FilterKey = 'all' | 'queued' | 'running' | 'completed' | 'failed'
type TaskDisplayState = 'queued' | 'running' | 'completed' | 'failed'
type BadgeTone = InstanceType<typeof StatusBadge>['$props']['tone']
type DisplayTask = Task & {
  isDemo?: boolean
  summary?: string
}

const router = useRouter()
const taskStore = useTaskStore()

const filter = ref<FilterKey>('all')

function isoMinutesAgo(minutes: number): string {
  return new Date(Date.now() - minutes * 60 * 1000).toISOString()
}

const demoTasks: DisplayTask[] = [
  {
    id: 'demo-pending-data-report',
    serverAlias: 'local-dev',
    serviceId: 'data-analysis',
    serviceName: '数据分析助手',
    title: '生成本周运营数据摘要',
    taskPrompt: '分析 CSV 数据并生成一页周报摘要。',
    outputPrompt: '结果写入 report.md，并附带关键指标表。',
    status: 'pending',
    createdAt: isoMinutesAgo(7),
    files: [],
    isPolling: true,
    summary: '任务已进入队列，等待服务接收并发送到执行端。',
    isDemo: true,
  },
  {
    id: 'demo-running-image-batch',
    serverAlias: 'local-dev',
    serviceId: 'image-processing',
    serviceName: '图像处理工作站',
    title: '批量转换产品截图为 WebP',
    taskPrompt: '将 28 张产品截图统一压缩为 1440px 宽度的 WebP 文件。',
    outputPrompt: '输出处理后文件和压缩比统计。',
    status: 'running',
    createdAt: isoMinutesAgo(23),
    startedAt: isoMinutesAgo(21),
    files: [],
    isPolling: true,
    summary: '任务已发送到服务器，正在等待服务端返回最终状态。',
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
    createdAt: isoMinutesAgo(78),
    startedAt: isoMinutesAgo(76),
    completedAt: isoMinutesAgo(64),
    result: '完成',
    files: [],
    isPolling: false,
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
    createdAt: isoMinutesAgo(96),
    startedAt: isoMinutesAgo(94),
    completedAt: isoMinutesAgo(91),
    files: [],
    errorMessage: '服务权限不足，无法读取上传附件。',
    isPolling: false,
    summary: '服务端返回失败状态：附件读取权限不足。',
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
  if (filter.value !== 'all') return sortedTasks.value.filter((t) => displayState(t.status) === filter.value)
  return sortedTasks.value
})

const filterItems = computed(() => {
  const tasks = sortedTasks.value
  return [
    { key: 'all' as const, label: '全部', count: tasks.length },
    { key: 'queued' as const, label: '排队中', count: tasks.filter((t) => displayState(t.status) === 'queued').length },
    { key: 'running' as const, label: '进行中', count: tasks.filter((t) => displayState(t.status) === 'running').length },
    { key: 'completed' as const, label: '已完成', count: tasks.filter((t) => displayState(t.status) === 'completed').length },
    { key: 'failed' as const, label: '失败', count: tasks.filter((t) => displayState(t.status) === 'failed').length },
  ]
})

const statusSummary = computed(() => ({
  queued: sortedTasks.value.filter((t) => displayState(t.status) === 'queued').length,
  running: sortedTasks.value.filter((t) => displayState(t.status) === 'running').length,
  completed: sortedTasks.value.filter((t) => displayState(t.status) === 'completed').length,
  failed: sortedTasks.value.filter((t) => displayState(t.status) === 'failed').length,
}))

function displayState(status: TaskStatus): TaskDisplayState {
  if (status === 'pending') return 'queued'
  if (status === 'running') return 'running'
  if (status === 'completed') return 'completed'
  return 'failed'
}

function statusMeta(status: TaskDisplayState): {
  label: string
  tone: BadgeTone
  icon: typeof Clock
  accentClass: string
  textClass: string
  description: string
} {
  const map: Record<TaskDisplayState, {
    label: string
    tone: BadgeTone
    icon: typeof Clock
    accentClass: string
    textClass: string
    description: string
  }> = {
    queued: {
      label: '排队中',
      tone: 'accent',
      icon: Clock,
      accentClass: 'bg-accent-soft text-accent border-accent/20',
      textClass: 'text-accent',
      description: '任务正在队列中等待服务接收。',
    },
    running: {
      label: '进行中',
      tone: 'info',
      icon: PlayCircle,
      accentClass: 'bg-info-soft text-info border-info/20',
      textClass: 'text-info',
      description: '任务已发送到服务器，正在等待最终结果。',
    },
    completed: {
      label: '已完成',
      tone: 'success',
      icon: CheckCircle,
      accentClass: 'bg-success/10 text-success border-success/20',
      textClass: 'text-success',
      description: '服务端已返回完成状态，可以查看输出。',
    },
    failed: {
      label: '失败',
      tone: 'danger',
      icon: XCircle,
      accentClass: 'bg-danger/10 text-danger border-danger/20',
      textClass: 'text-danger',
      description: '服务端返回失败状态，或任务无法继续执行。',
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
          按排队中、进行中、已完成和失败状态查看任务，快速判断下一步操作。
        </p>
      </div>

      <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">排队中</p>
          <p class="mt-1 text-xl font-bold text-accent">{{ statusSummary.queued }}</p>
        </div>
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">进行中</p>
          <p class="mt-1 text-xl font-bold text-info">{{ statusSummary.running }}</p>
        </div>
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">已完成</p>
          <p class="mt-1 text-xl font-bold text-success">{{ statusSummary.completed }}</p>
        </div>
        <div class="rounded-lg border border-border bg-bg-card px-4 py-3 shadow-sm">
          <p class="text-[11px] font-semibold text-text-muted">失败</p>
          <p class="mt-1 text-xl font-bold text-danger">{{ statusSummary.failed }}</p>
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
          'border-danger/20 bg-danger/5': displayState(task.status) === 'failed',
        }"
        @click="openTask(task)"
      >
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div class="flex min-w-0 flex-1 gap-3">
            <div
              class="flex flex-shrink-0 items-center justify-center rounded-lg border"
              :class="statusMeta(displayState(task.status)).accentClass"
              style="height: 44px; width: 44px;"
            >
              <component
                :is="statusMeta(displayState(task.status)).icon"
                class="h-5 w-5"
                :stroke-width="2.25"
                aria-hidden="true"
              />
            </div>

            <div class="min-w-0 flex-1">
              <div class="mb-2 flex flex-wrap items-center gap-2">
                <h3 class="truncate text-base font-bold text-info">{{ task.title }}</h3>
                <StatusBadge
                  :label="statusMeta(displayState(task.status)).label"
                  :tone="statusMeta(displayState(task.status)).tone"
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
          <div class="min-w-[220px] flex-1 rounded-lg border border-border bg-bg-inset px-3 py-2">
            <p class="text-[11px] font-semibold text-text-muted">当前状态</p>
            <p class="mt-1 text-sm font-semibold" :class="statusMeta(displayState(task.status)).textClass">
              {{ statusMeta(displayState(task.status)).description }}
            </p>
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
          v-else-if="displayState(task.status) === 'queued' || displayState(task.status) === 'running'"
          class="mt-3 flex items-center gap-2 rounded-lg border border-accent/15 bg-accent-soft px-3 py-2 text-xs font-semibold text-accent"
        >
          <Clock class="h-4 w-4 flex-shrink-0" aria-hidden="true" />
          {{ task.isPolling ? '正在同步任务状态' : '等待下一次状态同步' }}
        </div>
      </button>
    </div>
  </div>
</template>
