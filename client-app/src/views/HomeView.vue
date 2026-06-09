<script setup lang="ts">
import { useServerStore } from '@/stores/server'
import { useUiStore } from '@/stores/ui'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import type { ServiceItem } from '@/stores/server'
import { httpFetch } from '@/composables/useHttp'
import { friendlyErrorMessage } from '@/utils/error'
import { AlertTriangle, Inbox, PlugZap, RefreshCw } from '@lucide/vue'
import Skeleton from '@/components/Skeleton.vue'
import ServiceCard from '@/components/ServiceCard.vue'
import StatusBadge from '@/components/StatusBadge.vue'

const serverStore = useServerStore()
const uiStore = useUiStore()
const router = useRouter()

interface ServiceLoad {
  capacity?: number
  current_load?: number
  available_slots?: number
  pending_tasks?: number
  running_tasks?: number
}

const hasServers = computed(() => serverStore.serverCount > 0)
const cachedServices = computed(() => serverStore.getCachedServices())
const loads = ref<Record<string, ServiceLoad | null>>({})
const fetchError = ref<string | null>(null)
const isRefreshing = ref(false)

const seedServiceIds = new Set([
  'image-processing',
  'code-review',
  'doc-proofreading',
  'data-analysis',
])

const supplementalDemoServices: ServiceItem[] = [
  {
    id: 'demo-online-agent',
    name: '实时问答代理',
    description: '在线 Agent 示例，可公开访问并提供多个可用任务槽位。',
    agentStatus: 'online',
    registrationStatus: 'active',
    accessType: 'public',
    hasPermission: true,
    agentLastHeartbeat: new Date().toISOString(),
  },
  {
    id: 'demo-busy-agent',
    name: '高负载渲染代理',
    description: '忙碌 Agent 示例，用于观察 busy 状态和满负载数据卡片。',
    agentStatus: 'busy',
    registrationStatus: 'active',
    accessType: 'public',
    hasPermission: true,
    agentLastHeartbeat: new Date().toISOString(),
  },
  {
    id: 'demo-restricted-granted',
    name: '受限合规审查',
    description: '已授权访问的受限服务示例，适合合规审查和敏感内容处理。',
    agentStatus: 'online',
    registrationStatus: 'active',
    accessType: 'restricted',
    hasPermission: true,
    agentLastHeartbeat: new Date().toISOString(),
  },
  {
    id: 'demo-revoked-agent',
    name: '离线旧版代理',
    description: '旧版 Agent 当前不可用，展示服务暂时无法使用时的视觉效果。',
    agentStatus: 'offline',
    registrationStatus: 'revoked',
    accessType: 'public',
    hasPermission: true,
  },
]

const supplementalDemoLoads: Record<string, ServiceLoad | null> = {
  'demo-online-agent': {
    capacity: 4,
    current_load: 1,
    available_slots: 3,
    pending_tasks: 0,
    running_tasks: 1,
  },
  'demo-busy-agent': {
    capacity: 3,
    current_load: 3,
    available_slots: 0,
    pending_tasks: 5,
    running_tasks: 3,
  },
  'demo-restricted-granted': {
    capacity: 2,
    current_load: 1,
    available_slots: 1,
    pending_tasks: 1,
    running_tasks: 1,
  },
  'demo-revoked-agent': null,
}

const isSeedServiceList = computed(() => {
  const services = cachedServices.value
  return import.meta.env.DEV
    && !!services?.length
    && services.every((service) => seedServiceIds.has(service.id))
})

const displayServices = computed(() => {
  const services = cachedServices.value
  if (!services) return services
  if (isSeedServiceList.value) return [...services, ...supplementalDemoServices]
  return services
})

async function retryFetch() {
  if (isRefreshing.value) return
  if (!hasServers.value || !serverStore.defaultServer) return
  try {
    isRefreshing.value = true
    uiStore.setLoading(true)
    fetchError.value = null
    await serverStore.fetchServices()
    await fetchLoads()
  } catch (err) {
    fetchError.value = err instanceof Error ? friendlyErrorMessage(err.message) : friendlyErrorMessage(String(err))
  } finally {
    uiStore.setLoading(false)
    isRefreshing.value = false
  }
}

async function fetchLoads() {
  const server = serverStore.defaultServer
  if (!server?.apiKey) return
  const services = cachedServices.value
  if (!services) return
  await Promise.allSettled(services.map(async (svc: ServiceItem) => {
    try {
      const baseUrl = server.serverUrl.replace(/\/$/, '')
      const url = `${baseUrl}/api/v1/client/services/${encodeURIComponent(svc.id)}/load`
      const res = await httpFetch(url, {
        headers: { Authorization: `Bearer ${server.apiKey}` },
      })
      if (res.ok) {
        loads.value[svc.id] = await res.json()
      } else {
        loads.value[svc.id] = null
      }
    } catch (err) {
      loads.value[svc.id] = null
      uiStore.addToast(friendlyErrorMessage(err instanceof Error ? err.message : String(err)), 'error')
    }
  }))
}

function isSupplementalDemoService(serviceId: string): boolean {
  return serviceId in supplementalDemoLoads
}

function getServiceLoad(serviceId: string): ServiceLoad | null | undefined {
  if (serviceId in loads.value) return loads.value[serviceId]
  if (isSupplementalDemoService(serviceId)) return supplementalDemoLoads[serviceId]
  return undefined
}

function openService(serviceId: string) {
  if (isSupplementalDemoService(serviceId)) return
  router.push(`/service/${serviceId}`)
}

onMounted(async () => {
  if (hasServers.value && serverStore.defaultServer) {
    try {
      uiStore.setLoading(true)
      fetchError.value = null
      await serverStore.fetchServices()
      await fetchLoads()
    } catch (err) {
      fetchError.value = err instanceof Error ? friendlyErrorMessage(err.message) : friendlyErrorMessage(String(err))
    } finally {
      uiStore.setLoading(false)
    }
  }
})
</script>

<template>
  <div class="max-w-6xl mx-auto">
    <div class="mb-6 flex items-start justify-between gap-4">
      <div>
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-sm font-semibold text-accent">OpenAaaS Services</p>
          <StatusBadge v-if="isSeedServiceList" label="示例服务" tone="secondary" compact />
        </div>
        <h1 class="mt-1 text-3xl font-bold text-info">服务市场</h1>
        <p class="mt-2 text-sm text-text-secondary">
          浏览可用 Agent 服务，查看访问权限、在线状态和实时负载。
        </p>
      </div>
      <button
        class="inline-flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-bg-card text-text-secondary shadow-sm transition-colors hover:border-accent/35 hover:text-accent"
        :class="{ 'animate-spin': isRefreshing }"
        title="刷新"
        aria-label="刷新服务列表"
        @click="retryFetch"
      >
        <RefreshCw class="h-5 w-5" :stroke-width="2.25" aria-hidden="true" />
      </button>
    </div>

    <!-- Empty state: no servers -->
    <div
      v-if="!hasServers"
      class="flex flex-col items-center justify-center rounded-lg border border-border bg-bg-card py-24 text-text-muted shadow-sm"
    >
      <div class="mb-4 flex h-14 w-14 items-center justify-center rounded-lg bg-accent-soft text-accent">
        <PlugZap class="h-7 w-7" :stroke-width="2.25" aria-hidden="true" />
      </div>
      <p class="mb-2 text-lg font-bold text-info">暂无服务器</p>
      <p class="text-sm text-text-secondary">请先添加服务器以浏览可用服务</p>
      <router-link
        to="/settings"
        class="mt-4 inline-flex items-center rounded-md bg-accent px-4 py-2 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-accent-hover"
      >
        前往设置添加服务器 →
      </router-link>
    </div>

    <!-- Error state -->
    <div
      v-else-if="fetchError"
      class="flex flex-col items-center justify-center rounded-lg border border-secondary/20 bg-secondary-soft py-24 text-text-muted shadow-sm"
    >
      <div class="mb-4 flex h-14 w-14 items-center justify-center rounded-lg bg-bg-card text-secondary">
        <AlertTriangle class="h-7 w-7" :stroke-width="2.25" aria-hidden="true" />
      </div>
      <p class="mb-2 text-lg font-bold text-info">加载失败</p>
      <p class="text-sm text-text-secondary">{{ fetchError }}</p>
      <button
        class="mt-4 rounded-md bg-accent px-4 py-2 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-accent-hover"
        @click="retryFetch"
      >
        重试
      </button>
    </div>

    <!-- Skeleton grid: fetching or no cache -->
    <div
      v-else-if="serverStore.isFetching || cachedServices === null"
      class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
    >
      <div
        v-for="n in 6"
        :key="`sk-${n}`"
        class="bg-bg-card border border-border rounded-lg p-4 space-y-3 shadow-sm"
      >
        <div class="flex items-start justify-between">
          <Skeleton width="60%" height="18px" />
          <Skeleton width="40px" height="16px" rounded="9999px" />
        </div>
        <Skeleton :rows="2" height="14px" />
        <Skeleton width="40%" height="14px" />
        <Skeleton :rows="2" height="12px" width="70%" />
      </div>
    </div>

    <!-- Empty state: no services available -->
    <div
      v-else-if="displayServices?.length === 0"
      class="flex flex-col items-center justify-center rounded-lg border border-border bg-bg-card py-24 text-text-muted shadow-sm"
    >
      <div class="mb-4 flex h-14 w-14 items-center justify-center rounded-lg bg-bg-secondary text-text-secondary">
        <Inbox class="h-7 w-7" :stroke-width="2.25" aria-hidden="true" />
      </div>
      <p class="text-lg font-bold text-info">当前服务器没有可用服务</p>
    </div>

    <!-- Service grid placeholder -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <ServiceCard
        v-for="service in displayServices"
        :key="service.id"
        :service="service"
        :load="getServiceLoad(service.id)"
        :demo="isSupplementalDemoService(service.id)"
        @open="openService"
      />
    </div>
  </div>
</template>
