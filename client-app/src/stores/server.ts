import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { httpFetch, httpFetchWithRedirect, parseServerError } from '@/composables/useHttp'
import { friendlyErrorMessage } from '@/utils/error'
import { loadState, saveState } from './persist'

export interface Server {
  alias: string
  serverUrl: string
  apiKey?: string
  clientId?: string
  clientName?: string
  isDefault: boolean
}

export interface ServiceItem {
  id: string
  name: string
  description?: string
  agentStatus: 'online' | 'offline' | 'busy'
  registrationStatus: string
  accessType: 'public' | 'restricted'
  hasPermission: boolean
  agentLastHeartbeat?: string
}

export interface ServiceCache {
  fetchedAt: number
  items: ServiceItem[]
}

const DEV_SERVER_ALIAS = import.meta.env.VITE_DEV_SERVER_ALIAS?.trim() || 'local-dev'
const DEV_SERVER_URL = import.meta.env.VITE_DEV_SERVER_URL?.trim()
const DEV_SERVER_API_KEY = import.meta.env.VITE_DEV_SERVER_API_KEY?.trim()
const DEV_SERVER_CLIENT_ID = import.meta.env.VITE_DEV_SERVER_CLIENT_ID?.trim()
const DEV_SERVER_CLIENT_NAME = import.meta.env.VITE_DEV_SERVER_CLIENT_NAME?.trim() || 'desktop-client-dev'
const shouldSeedDevServer =
  import.meta.env.DEV
  && import.meta.env.MODE !== 'test'
  && !!DEV_SERVER_URL
  && !!DEV_SERVER_API_KEY

export const useServerStore = defineStore('server', () => {
  const persisted = loadState()

  // Auto-add a default local dev server if none exists (development only)
  const initialServers: Server[] = (persisted.servers || []) as Server[]
  if (shouldSeedDevServer && initialServers.length === 0) {
    initialServers.push({
      alias: DEV_SERVER_ALIAS,
      serverUrl: DEV_SERVER_URL!,
      apiKey: DEV_SERVER_API_KEY!,
      clientId: DEV_SERVER_CLIENT_ID,
      clientName: DEV_SERVER_CLIENT_NAME,
      isDefault: true,
    })
  }

  const servers = ref<Server[]>(initialServers)
  const defaultAlias = ref<string | undefined>(persisted.defaultAlias || (initialServers[0]?.alias))
  const services = ref<Record<string, ServiceCache>>((persisted.services || {}) as Record<string, ServiceCache>)
  const isFetching = ref(false)
  const fetchError = ref<string | null>(null)

  const defaultServer = computed<Server | undefined>(() =>
    servers.value.find((s) => s.isDefault) || servers.value[0],
  )

  const serverCount = computed(() => servers.value.length)

  function persist() {
    saveState({ servers: servers.value, defaultAlias: defaultAlias.value, services: services.value })
  }

  function addServer(server: Omit<Server, 'isDefault'>) {
    if (servers.value.some((s) => s.alias === server.alias)) {
      throw new Error(`服务器别名 "${server.alias}" 已存在`)
    }
    const newServer: Server = { ...server, isDefault: servers.value.length === 0 }
    servers.value.push(newServer)
    if (newServer.isDefault) {
      defaultAlias.value = newServer.alias
    }
    persist()
  }

  function removeServer(alias: string) {
    const idx = servers.value.findIndex((s) => s.alias === alias)
    if (idx === -1) return
    const wasDefault = servers.value[idx].isDefault
    servers.value.splice(idx, 1)
    if (wasDefault && servers.value.length > 0) {
      servers.value[0].isDefault = true
      defaultAlias.value = servers.value[0].alias
    } else if (servers.value.length === 0) {
      defaultAlias.value = undefined
    }
    persist()
  }

  function setDefault(alias: string) {
    servers.value.forEach((s) => {
      s.isDefault = s.alias === alias
    })
    defaultAlias.value = alias
    persist()
  }

  function updateServer(alias: string, patch: Partial<Server>) {
    const idx = servers.value.findIndex((s) => s.alias === alias)
    if (idx !== -1) {
      servers.value[idx] = { ...servers.value[idx], ...patch }
      persist()
    }
  }

  async function register(alias: string, name: string): Promise<{ apiKey: string; clientId: string }> {
    const server = servers.value.find((s) => s.alias === alias)
    if (!server) throw new Error('服务器不存在')

    const url = `${server.serverUrl.replace(/\/$/, '')}/api/v1/client/auth/register`
    const res = await httpFetchWithRedirect(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    })

    if (!res.ok) {
      const message = await parseServerError(res)
      throw new Error(`注册失败: ${res.status} ${message}`)
    }

    const data = await res.json()
    if (!data.api_key || !data.id) {
      throw new Error('注册响应格式错误')
    }

    updateServer(alias, {
      apiKey: data.api_key,
      clientId: data.id,
      clientName: name,
    })

    return { apiKey: data.api_key, clientId: data.id }
  }

  async function fetchServices(alias?: string): Promise<ServiceItem[]> {
    const target = alias ?? defaultAlias.value
    if (!target) throw new Error('没有配置默认服务器')

    const server = servers.value.find((s) => s.alias === target)
    if (!server) throw new Error('服务器不存在')
    if (!server.apiKey) throw new Error('服务器未注册，请先获取 API Key')

    isFetching.value = true
    fetchError.value = null

    try {
      const url = `${server.serverUrl.replace(/\/$/, '')}/api/v1/client/services`
      const res = await httpFetch(url, {
        headers: { Authorization: `Bearer ${server.apiKey}` },
      })

      if (!res.ok) {
        const message = await parseServerError(res)
        throw new Error(`获取服务列表失败: ${res.status} ${message}`)
      }

      const data = await res.json()
      const rawItems: any[] = Array.isArray(data) ? data : data.services || []
      const items: ServiceItem[] = rawItems.map((s: any) => ({
        id: s.id,
        name: s.name,
        description: s.description,
        agentStatus: s.agent_status,
        registrationStatus: s.registration_status,
        agentLastHeartbeat: s.agent_last_heartbeat,
        accessType: s.access_type,
        hasPermission: s.has_permission,
      }))

      services.value[target] = {
        fetchedAt: Date.now(),
        items,
      }
      persist()

      return items
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err)
      const message = friendlyErrorMessage(raw)
      fetchError.value = message
      throw err
    } finally {
      isFetching.value = false
    }
  }

  function getCachedServices(alias?: string): ServiceItem[] | null {
    const target = alias ?? defaultAlias.value
    if (!target) return null
    const cache = services.value[target]
    if (!cache) return null
    // Cache valid for 5 minutes
    if (Date.now() - cache.fetchedAt > 5 * 60 * 1000) return null
    return cache.items
  }

  return {
    servers,
    defaultAlias,
    defaultServer,
    serverCount,
    services,
    isFetching,
    fetchError,
    addServer,
    removeServer,
    setDefault,
    updateServer,
    register,
    fetchServices,
    getCachedServices,
  }
})
