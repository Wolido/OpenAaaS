<script setup lang="ts">
import { computed } from 'vue'
import { ArrowRight, Gauge, LockKeyhole, Server, Sparkles } from '@lucide/vue'
import type { ServiceItem } from '@/stores/server'
import StatusBadge from '@/components/StatusBadge.vue'
import Skeleton from '@/components/Skeleton.vue'

interface ServiceLoad {
  capacity?: number
  current_load?: number
  available_slots?: number
  pending_tasks?: number
  running_tasks?: number
}

const props = defineProps<{
  service: ServiceItem
  load?: ServiceLoad | null
  demo?: boolean
}>()

const emit = defineEmits<{
  open: [serviceId: string]
}>()

const statusMeta = computed(() => {
  const map = {
    online: { label: 'ONLINE', tone: 'success' as const },
    offline: { label: 'OFFLINE', tone: 'secondary' as const },
    busy: { label: 'BUSY', tone: 'warning' as const },
  }
  return map[props.service.agentStatus] ?? { label: props.service.agentStatus, tone: 'neutral' as const }
})

const accessMeta = computed(() => {
  if (props.service.accessType === 'public') return { label: '公开', tone: 'accent' as const }
  return { label: '受限', tone: 'info' as const }
})

const loadMetrics = computed(() => {
  if (!props.load) return []
  return [
    { label: '容量', value: props.load.capacity },
    { label: '负载', value: props.load.current_load },
    { label: '可用', value: props.load.available_slots },
    { label: '排队', value: props.load.pending_tasks },
    { label: '运行', value: props.load.running_tasks },
  ].filter((item) => item.value != null)
})
</script>

<template>
  <button
    type="button"
    class="group flex min-h-[196px] w-full flex-col rounded-lg border border-border bg-bg-card p-4 text-left shadow-sm transition-all hover:-translate-y-0.5 hover:border-accent/35 hover:shadow-soft focus-visible:shadow-focus"
    :class="{
      'border-secondary/30 bg-secondary-soft/60': !service.hasPermission,
      'cursor-default hover:translate-y-0': demo,
    }"
    @click="emit('open', service.id)"
  >
    <div class="mb-3 flex items-start justify-between gap-3">
      <div class="flex min-w-0 items-start gap-3">
        <div
          class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border"
          :class="service.hasPermission
            ? 'border-accent/20 bg-accent-soft text-accent'
            : 'border-secondary/25 bg-bg-card text-secondary'"
        >
          <Sparkles v-if="service.hasPermission" class="h-5 w-5" :stroke-width="2.25" aria-hidden="true" />
          <LockKeyhole v-else class="h-5 w-5" :stroke-width="2.25" aria-hidden="true" />
        </div>
        <div class="min-w-0">
          <h3 class="truncate text-base font-bold text-info">{{ service.name }}</h3>
          <div class="mt-1 flex flex-wrap items-center gap-1.5">
            <StatusBadge v-if="demo" label="示例" tone="neutral" compact />
            <StatusBadge :label="accessMeta.label" :tone="accessMeta.tone" compact />
            <StatusBadge
              v-if="!service.hasPermission"
              label="无权限"
              tone="secondary"
              compact
              show-icon
            />
          </div>
        </div>
      </div>
      <StatusBadge :label="statusMeta.label" :tone="statusMeta.tone" compact />
    </div>

    <p class="mb-4 min-h-[42px] text-sm leading-6 text-text-secondary line-clamp-2">
      {{ service.description || '暂无描述' }}
    </p>

    <div v-if="loadMetrics.length > 0" class="mt-auto grid grid-cols-3 gap-2">
      <div
        v-for="metric in loadMetrics.slice(0, 3)"
        :key="metric.label"
        class="rounded-lg border border-border bg-bg-inset px-2 py-2"
      >
        <p class="text-[10px] font-semibold text-text-muted">{{ metric.label }}</p>
        <p class="mt-1 text-sm font-bold text-text-primary">{{ metric.value }}</p>
      </div>
    </div>
    <div v-else-if="load === null" class="mt-auto flex items-center gap-2 rounded-lg border border-secondary/20 bg-secondary-soft px-3 py-2 text-xs font-semibold text-secondary">
      <Gauge class="h-4 w-4" aria-hidden="true" />
      无法获取负载信息
    </div>
    <div v-else-if="service.hasPermission" class="mt-auto space-y-2 rounded-lg border border-border bg-bg-inset px-3 py-3">
      <Skeleton height="12px" width="72%" />
      <Skeleton height="12px" width="52%" />
    </div>
    <div v-else class="mt-auto flex items-center gap-2 rounded-lg border border-border bg-bg-card px-3 py-2 text-xs font-medium text-text-muted">
      <Server class="h-4 w-4" aria-hidden="true" />
      需要授权后查看负载
    </div>

    <div class="mt-4 flex items-center justify-between border-t border-border pt-3 text-xs font-semibold text-text-muted">
      <span>注册状态: {{ service.registrationStatus }}</span>
      <span
        v-if="!demo"
        class="inline-flex items-center gap-1 text-accent opacity-0 transition-opacity group-hover:opacity-100"
      >
        查看详情
        <ArrowRight class="h-3.5 w-3.5" aria-hidden="true" />
      </span>
      <span v-else class="text-text-muted">视觉预览</span>
    </div>
  </button>
</template>
