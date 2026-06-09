<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, onMounted, ref, watch } from 'vue'
import { useServerStore } from '@/stores/server'
import { httpFetch } from '@/composables/useHttp'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import Skeleton from '@/components/Skeleton.vue'

const route = useRoute()
const serverStore = useServerStore()

const serviceId = computed(() => String(route.params.id))
const service = computed(() => {
  for (const cache of Object.values(serverStore.services)) {
    const found = cache.items.find((s) => s.id === serviceId.value)
    if (found) return found
  }
  return undefined
})

const load = ref<{
  capacity?: number
  current_load?: number
  available_slots?: number
  pending_tasks?: number
  running_tasks?: number
} | null | undefined>(undefined)

const usage = ref<string | null | undefined>(undefined)
const usageHtml = computed(() => {
  if (!usage.value) return ''
  const html = marked.parse(usage.value, { async: false, breaks: true }) as string
  return DOMPurify.sanitize(html)
})

async function fetchServiceData() {
  if (!serviceId.value) return
  const server = serverStore.defaultServer
  if (!server?.apiKey) return
  const baseUrl = server.serverUrl.replace(/\/$/, '')

  try {
    const loadUrl = `${baseUrl}/api/v1/client/services/${encodeURIComponent(serviceId.value)}/load`
    const loadRes = await httpFetch(loadUrl, {
      headers: { Authorization: `Bearer ${server.apiKey}` },
    })
    if (loadRes.ok) {
      load.value = await loadRes.json()
    } else {
      load.value = null
    }
  } catch {
    load.value = null
  }

  try {
    const usageUrl = `${baseUrl}/api/v1/client/services/${encodeURIComponent(serviceId.value)}/usage`
    const usageRes = await httpFetch(usageUrl, {
      headers: { Authorization: `Bearer ${server.apiKey}` },
    })
    if (usageRes.ok) {
      const data = await usageRes.json()
      usage.value = data.usage ?? null
    } else {
      usage.value = null
    }
  } catch {
    usage.value = null
  }
}

onMounted(() => {
  fetchServiceData()

  const server = serverStore.defaultServer
  if (server?.apiKey) {
    serverStore.fetchServices().catch((err) => {
      console.warn('刷新服务列表失败', err)
    })
  }
})

watch(() => route.params.id, () => {
  load.value = undefined
  usage.value = undefined
  fetchServiceData()
  serverStore.fetchServices().catch((err) => {
    console.warn('刷新服务列表失败', err)
  })
})
</script>

<template>
  <div class="max-w-3xl mx-auto">
    <div class="mb-6">
      <router-link to="/" class="text-sm text-text-secondary hover:text-text-primary mb-2 inline-block">
        ← 返回服务市场
      </router-link>
      <h1 class="text-2xl font-bold mt-2">服务详情</h1>
    </div>

    <div v-if="service" class="bg-bg-secondary border border-border rounded-lg p-6">
      <div class="flex items-start justify-between mb-4">
        <h2 class="text-xl font-semibold">{{ service.name }}</h2>
        <span
          class="text-[11px] font-semibold px-2 py-1 rounded-full uppercase"
          :class="{
            'bg-success/10 text-success': service.agentStatus === 'online',
            'bg-danger/10 text-danger': service.agentStatus === 'offline',
            'bg-warning/10 text-warning': service.agentStatus === 'busy',
          }"
        >
          {{ service.agentStatus }}
        </span>
      </div>
      <p class="text-text-secondary mb-4">{{ service.description || '暂无描述' }}</p>

      <div class="flex items-center gap-3 mb-6">
        <span
          class="text-xs px-2 py-1 rounded border"
          :class="service.accessType === 'public'
            ? 'border-success/30 text-success'
            : 'border-warning/30 text-warning'"
        >
          {{ service.accessType === 'public' ? '公开访问' : '需要授权' }}
        </span>
      </div>

      <div v-if="load !== undefined && load !== null" class="mt-4 p-3 bg-bg-primary border border-border rounded-md">
        <p class="text-sm font-medium mb-1">负载信息</p>
        <div class="flex flex-wrap gap-3 text-xs text-text-muted">
          <span v-if="load.capacity != null">容量: {{ load.capacity }}</span>
          <span v-if="load.current_load != null">当前负载: {{ load.current_load }}</span>
          <span v-if="load.available_slots != null">可用槽位: {{ load.available_slots }}</span>
          <span v-if="load.pending_tasks != null">排队任务: {{ load.pending_tasks }}</span>
          <span v-if="load.running_tasks != null">运行中任务: {{ load.running_tasks }}</span>
        </div>
      </div>
      <div v-else-if="load === null" class="mt-4 p-3 bg-bg-primary border border-border rounded-md">
        <p class="text-sm font-medium mb-1">负载信息</p>
        <p class="text-xs text-danger">获取失败</p>
      </div>
      <div v-else class="mt-4 p-3 bg-bg-primary border border-border rounded-md space-y-2">
        <p class="text-sm font-medium mb-1">负载信息</p>
        <Skeleton :rows="3" height="14px" />
      </div>

      <div v-if="usage !== undefined && usage !== null" class="mt-4 p-3 bg-bg-primary border border-border rounded-md">
        <p class="text-sm font-medium mb-1">使用说明</p>
        <div class="text-sm leading-relaxed prose max-w-none" v-html="usageHtml" />
      </div>
      <div v-else-if="usage === null" class="mt-4 p-3 bg-bg-primary border border-border rounded-md">
        <p class="text-sm font-medium mb-1">使用说明</p>
        <p class="text-xs text-danger">获取失败</p>
      </div>
      <div v-else class="mt-4 p-3 bg-bg-primary border border-border rounded-md space-y-2">
        <p class="text-sm font-medium mb-1">使用说明</p>
        <Skeleton :rows="8" height="14px" />
      </div>

      <router-link
        v-if="service.hasPermission"
        :to="`/submit/${service.id}`"
        class="inline-flex items-center px-4 py-2 bg-accent text-white rounded-md text-sm font-medium hover:bg-accent-hover transition-colors mt-4"
      >
        提交任务 →
      </router-link>
      <span v-else class="text-sm text-danger mt-4 inline-block">您没有权限使用此服务</span>
    </div>

    <div v-else class="text-text-muted">
      服务信息未找到
    </div>
  </div>
</template>
