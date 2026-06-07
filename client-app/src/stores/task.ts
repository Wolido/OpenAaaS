import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { httpFetch, httpFetchWithRedirect, parseServerError, uploadWithFiles } from '@/composables/useHttp'
import { friendlyErrorMessage } from '@/utils/error'
import { loadState, saveState } from './persist'

export interface TaskFile {
  id: string
  filename: string
  mimeType?: string
  sizeBytes: number
  storagePath: string
  createdAt: string
  isPlaceholder?: boolean
}

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'cancelling'

export interface Task {
  id: string
  serverAlias: string
  serviceId: string
  serviceName: string
  title: string
  taskPrompt: string
  outputPrompt: string
  status: TaskStatus
  createdAt: string
  startedAt?: string
  completedAt?: string
  result?: string
  files: TaskFile[]
  errorMessage?: string
  isPolling: boolean
  pollFailCount?: number
  resultFetched?: boolean
  fetchError?: string
  isFetchingResult?: boolean
  pollError?: string
}

const FIRST_POLL_MS = 5000
const POLL_INTERVAL_MS = 30000
const MAX_POLL_FAIL = 20

function isActiveStatus(status: TaskStatus): boolean {
  return ['pending', 'running'].includes(status)
}

function isTerminalStatus(status: TaskStatus): boolean {
  return ['completed', 'failed', 'cancelled'].includes(status)
}

function loadPersistedTasks(): Task[] {
  const state = loadState()
  return (state.tasks || []) as Task[]
}

function saveTasks(tasks: Task[]) {
  saveState({ tasks })
}

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>(loadPersistedTasks())
  const isSubmitting = ref(false)
  const submitError = ref<string | null>(null)

  const activeTasks = computed(() => tasks.value.filter((t) => isActiveStatus(t.status)))
  const activeTaskCount = computed(() => activeTasks.value.length)

  const pollTimers = new Map<string, number>()

  function persist() {
    saveTasks(tasks.value)
  }

  function getTask(id: string): Task | undefined {
    return tasks.value.find((t) => t.id === id)
  }

  function updateTask(id: string, patch: Partial<Task>) {
    const idx = tasks.value.findIndex((t) => t.id === id)
    if (idx !== -1) {
      tasks.value[idx] = { ...tasks.value[idx], ...patch }
      persist()
    }
  }

  function getServerConfig(alias: string) {
    const state = loadState()
    const servers = (state.servers || []) as { alias: string; serverUrl: string; apiKey?: string; clientId?: string; clientName?: string }[]
    return servers.find((s) => s.alias === alias)
  }

  async function submitTask(params: {
    serverAlias: string
    serviceId: string
    serviceName: string
    title: string
    taskPrompt: string
    outputPrompt: string
    files?: File[]
  }): Promise<string> {
    isSubmitting.value = true
    submitError.value = null

    try {
      const server = getServerConfig(params.serverAlias)
      if (!server) throw new Error('服务器不存在')
      if (!server.apiKey) throw new Error('服务器未注册')

      const baseUrl = server.serverUrl.replace(/\/$/, '')
      const url = `${baseUrl}/api/v1/client/tasks`

      const res = await uploadWithFiles(
        url,
        {
          service_id: params.serviceId,
          task_prompt: params.taskPrompt,
          output_prompt: params.outputPrompt,
        },
        params.files || [],
        { Authorization: `Bearer ${server.apiKey}` },
      )

      if (!res.ok) {
        const message = await parseServerError(res)
        throw new Error(`提交任务失败: ${res.status} ${message}`)
      }

      const data = await res.json()
      const taskId: string = data.id

      const newTask: Task = {
        id: taskId,
        serverAlias: params.serverAlias,
        serviceId: params.serviceId,
        serviceName: params.serviceName,
        title: params.title,
        taskPrompt: params.taskPrompt,
        outputPrompt: params.outputPrompt,
        status: (data.status as TaskStatus) || 'pending',
        createdAt: data.created_at || new Date().toISOString(),
        startedAt: data.started_at,
        completedAt: data.completed_at,
        files: [],
        errorMessage: data.error_message,
        isPolling: true,
        pollFailCount: 0,
      }

      tasks.value.unshift(newTask)
      persist()
      startPolling(taskId)

      return taskId
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err)
      const message = friendlyErrorMessage(raw)
      submitError.value = message
      throw err
    } finally {
      isSubmitting.value = false
    }
  }

  async function doPoll(taskId: string, isFirst: boolean) {
    const task = getTask(taskId)
    if (!task) return
    if (isTerminalStatus(task.status)) {
      stopPolling(taskId)
      return
    }

    const server = getServerConfig(task.serverAlias)
    if (!server || !server.apiKey) {
      stopPolling(taskId)
      return
    }

    try {
      if (!task.isPolling) return
      const baseUrl = server.serverUrl.replace(/\/$/, '')
      const url = `${baseUrl}/api/v1/client/tasks/${encodeURIComponent(taskId)}`
      const res = await httpFetch(url, {
        headers: { Authorization: `Bearer ${server.apiKey}` },
      })

      if (!res.ok) {
        const message = await parseServerError(res)
        throw new Error(`查询失败: ${res.status} ${message}`)
      }

      const data = await res.json()
      const newStatus: TaskStatus = data.status || task.status

      const patch: Partial<Task> = {
        status: newStatus,
        pollFailCount: 0,
        pollError: undefined,
        isPolling: !isTerminalStatus(newStatus),
      }
      if (data.started_at) patch.startedAt = data.started_at
      if (data.completed_at) patch.completedAt = data.completed_at
      patch.errorMessage = data.error_message ?? undefined

      if (data.output) {
        const output = data.output as Record<string, unknown>
        if (output.file_ids && Array.isArray(output.file_ids)) {
          patch.files = (output.file_ids as string[]).map((id) => ({
            id,
            filename: '',
            mimeType: undefined,
            sizeBytes: 0,
            storagePath: '',
            createdAt: '',
            isPlaceholder: true,
          }))
        }
        const resultKeys = Object.keys(output).filter((k) => k !== 'file_ids')
        if (resultKeys.length > 0) {
          patch.result = JSON.stringify(output, null, 2)
        }
      }

      updateTask(taskId, patch)

      if (isTerminalStatus(newStatus)) {
        stopPolling(taskId)
        if (newStatus === 'completed') {
          try {
            await fetchResult(taskId)
          } catch {
            // fetchResult 内部已处理错误，此处静默忽略
          }
        }
      } else {
        const delay = isFirst ? FIRST_POLL_MS : POLL_INTERVAL_MS
        const timer = window.setTimeout(() => doPoll(taskId, false), delay)
        pollTimers.set(taskId, timer)
      }
    } catch (err) {
      const currentTask = getTask(taskId)
      if (!currentTask) return
      if (isTerminalStatus(currentTask.status) || !currentTask.isPolling) return
      const failCount = (currentTask.pollFailCount || 0) + 1
      updateTask(taskId, { pollFailCount: failCount })
      if (failCount >= MAX_POLL_FAIL) {
        stopPolling(taskId)
        const raw = err instanceof Error ? err.message : String(err)
        const friendly = friendlyErrorMessage(raw)
        const pollMsg = `轮询失败次数过多，已停止。原因：${friendly}`
        updateTask(taskId, { pollError: pollMsg })
      } else {
        const delay = Math.min(POLL_INTERVAL_MS * 2 ** (failCount - 1), 300000)
        const timer = window.setTimeout(() => doPoll(taskId, false), delay)
        pollTimers.set(taskId, timer)
      }
    }
  }

  function startPolling(taskId: string) {
    stopPolling(taskId)
    updateTask(taskId, { isPolling: true, pollFailCount: 0, pollError: undefined })
    const timer = window.setTimeout(() => doPoll(taskId, true), FIRST_POLL_MS)
    pollTimers.set(taskId, timer)
  }

  function stopPolling(taskId: string) {
    const timer = pollTimers.get(taskId)
    if (timer !== undefined) {
      clearTimeout(timer)
      pollTimers.delete(taskId)
    }
    const task = getTask(taskId)
    if (task && task.isPolling) {
      updateTask(taskId, { isPolling: false })
    }
  }

  async function cancelTask(taskId: string) {
    const task = getTask(taskId)
    if (!task) throw new Error('任务不存在')

    const server = getServerConfig(task.serverAlias)
    if (!server || !server.apiKey) throw new Error('服务器配置不存在')

    const baseUrl = server.serverUrl.replace(/\/$/, '')
    const url = `${baseUrl}/api/v1/client/tasks/${encodeURIComponent(taskId)}/cancel`
    const res = await httpFetchWithRedirect(url, {
      method: 'POST',
      headers: { Authorization: `Bearer ${server.apiKey}` },
    })

    if (!res.ok) {
      const message = await parseServerError(res)
      throw new Error(`取消任务失败: ${res.status} ${message}`)
    }

    const data = await res.json()
    updateTask(taskId, {
      status: (data.status as TaskStatus) || 'cancelled',
      completedAt: data.completed_at,
      isPolling: false,
    })
    stopPolling(taskId)
  }

  async function fetchResult(taskId: string) {
    const task = getTask(taskId)
    if (!task) return

    const server = getServerConfig(task.serverAlias)
    if (!server || !server.apiKey) return

    updateTask(taskId, { isFetchingResult: true })
    try {
      const baseUrl = server.serverUrl.replace(/\/$/, '')
      const listUrl = `${baseUrl}/api/v1/client/files/list/${encodeURIComponent(taskId)}`
      const res = await httpFetch(listUrl, {
        headers: { Authorization: `Bearer ${server.apiKey}` },
      })

      if (!res.ok) {
        const message = await parseServerError(res)
        throw new Error(`获取文件列表失败: ${res.status} ${message}`)
      }

      const data = await res.json()
      const files: TaskFile[] = (data.files || []).map((f: Record<string, unknown>) => ({
        id: String(f.id || ''),
        filename: String(f.filename || ''),
        mimeType: f.mime_type ? String(f.mime_type) : undefined,
        sizeBytes: Number(f.size_bytes || 0),
        storagePath: String(f.storage_path || ''),
        createdAt: String(f.created_at || ''),
      }))

      updateTask(taskId, { files, resultFetched: true, fetchError: undefined })

      const mdFile = files.find((f) => f.filename.endsWith('.md'))
      if (mdFile) {
        const downloadUrl = `${baseUrl}/api/v1/client/files/${encodeURIComponent(mdFile.id)}/download`
        const blobRes = await httpFetch(downloadUrl, {
          headers: { Authorization: `Bearer ${server.apiKey}` },
        })
        if (blobRes.ok) {
          const text = await blobRes.text()
          updateTask(taskId, { result: text })
        }
      }
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err)
      const message = friendlyErrorMessage(raw)
      updateTask(taskId, { resultFetched: true, fetchError: message })
    } finally {
      updateTask(taskId, { isFetchingResult: false })
    }
  }

  function removeTask(taskId: string) {
    stopPolling(taskId)
    const idx = tasks.value.findIndex((t) => t.id === taskId)
    if (idx !== -1) {
      tasks.value.splice(idx, 1)
      persist()
    }
  }

  function resumePolling() {
    for (const task of tasks.value) {
      if (isActiveStatus(task.status)) {
        startPolling(task.id)
      }
    }
  }

  function resumePollingForTask(taskId: string) {
    startPolling(taskId)
  }

  return {
    tasks,
    activeTasks,
    activeTaskCount,
    isSubmitting,
    submitError,
    getTask,
    submitTask,
    startPolling,
    stopPolling,
    cancelTask,
    fetchResult,
    removeTask,
    updateTask,
    resumePolling,
    resumePollingForTask,
  }
})
