import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { httpFetch, httpFetchWithRedirect, uploadWithFiles } from '../useHttp'

vi.mock('@tauri-apps/plugin-http', () => ({
  fetch: vi.fn(),
}))

// Re-import after mocking to get the mocked module
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
const _mockedTauriFetchModule = vi.mocked(tauriFetch)

describe('httpFetch', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('should use tauri fetch when available', async () => {
    const response = new Response('ok', { status: 200 })
    _mockedTauriFetchModule.mockResolvedValue(response)

    const res = await httpFetch('http://example.com/api')
    expect(res.status).toBe(200)
    expect(_mockedTauriFetchModule).toHaveBeenCalledWith('http://example.com/api', undefined)
  })

  it('should fallback to native fetch when tauri fetch fails', async () => {
    _mockedTauriFetchModule.mockRejectedValue(new Error('not in tauri'))
    const nativeResponse = new Response('fallback', { status: 200 })
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(nativeResponse)

    const res = await httpFetch('http://example.com/api')
    expect(res.status).toBe(200)
    expect(globalThis.fetch).toHaveBeenCalledWith('http://example.com/api', undefined)
  })

  it('should rethrow scope rejection errors instead of falling back', async () => {
    const scopeError = new Error('url not allowed on the configured scope')
    _mockedTauriFetchModule.mockRejectedValue(scopeError)

    await expect(httpFetch('http://example.com/api')).rejects.toThrow('url not allowed on the configured scope')
    expect(globalThis.fetch).not.toHaveBeenCalled()
  })

  it('should rethrow command not allowed errors instead of falling back', async () => {
    const permissionError = new Error('command not allowed')
    _mockedTauriFetchModule.mockRejectedValue(permissionError)

    await expect(httpFetch('http://example.com/api')).rejects.toThrow('command not allowed')
    expect(globalThis.fetch).not.toHaveBeenCalled()
  })

  it('should still fallback for non-scope tauri errors', async () => {
    _mockedTauriFetchModule.mockRejectedValue(new Error('window.__TAURI_INTERNALS__ is undefined'))
    const nativeResponse = new Response('fallback', { status: 200 })
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(nativeResponse)

    const res = await httpFetch('http://example.com/api')
    expect(res.status).toBe(200)
    expect(globalThis.fetch).toHaveBeenCalledWith('http://example.com/api', undefined)
  })
})

describe('httpFetchWithRedirect', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('should follow single redirect and strip Authorization on cross-domain', async () => {
    const redirectResponse = new Response(null, {
      status: 302,
      headers: { Location: 'https://other.com/api' },
    })
    const finalResponse = new Response('ok', { status: 200 })

    _mockedTauriFetchModule
      .mockResolvedValueOnce(redirectResponse)
      .mockResolvedValueOnce(finalResponse)

    const res = await httpFetchWithRedirect('https://example.com/api', {
      method: 'POST',
      headers: { Authorization: 'Bearer token', 'Content-Type': 'application/json' },
      redirect: 'manual' as any,
    })

    expect(res.status).toBe(200)
    // Second call should not have Authorization because it's cross-domain
    const secondCall = _mockedTauriFetchModule.mock.calls[1]
    expect((secondCall[1] as any)?.headers?.has('Authorization') ?? true).toBe(false)
  })

  it('should preserve Authorization on same-domain redirect', async () => {
    const redirectResponse = new Response(null, {
      status: 301,
      headers: { Location: 'https://example.com/api/v2' },
    })
    const finalResponse = new Response('ok', { status: 200 })

    _mockedTauriFetchModule
      .mockResolvedValueOnce(redirectResponse)
      .mockResolvedValueOnce(finalResponse)

    const res = await httpFetchWithRedirect('https://example.com/api', {
      method: 'POST',
      headers: { Authorization: 'Bearer token' },
      redirect: 'manual' as any,
    })

    expect(res.status).toBe(200)
    const secondCall = _mockedTauriFetchModule.mock.calls[1]
    expect((secondCall[1] as any)?.headers?.get('Authorization')).toBe('Bearer token')
  })

  it('should throw on too many redirects', async () => {
    const redirectResponse = new Response(null, {
      status: 302,
      headers: { Location: 'https://example.com/1' },
    })

    _mockedTauriFetchModule.mockResolvedValue(redirectResponse)

    await expect(
      httpFetchWithRedirect('https://example.com/api', { redirect: 'manual' as any }),
    ).rejects.toThrow('多次重定向')
  })

  it('should fallback to httpFetch when manual redirect is not supported', async () => {
    _mockedTauriFetchModule.mockRejectedValue(new Error('unsupported redirect option'))
    const nativeResponse = new Response('fallback', { status: 200 })
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(nativeResponse)

    const res = await httpFetchWithRedirect('http://example.com/api')
    expect(res.status).toBe(200)
  })

  it('should fallback to currentUrl when redirect fails mid-chain', async () => {
    const redirectResponse = new Response(null, {
      status: 302,
      headers: { Location: 'https://redirected.com/api' },
    })
    _mockedTauriFetchModule.mockResolvedValueOnce(redirectResponse)
    // All subsequent tauriFetch calls fail
    _mockedTauriFetchModule.mockRejectedValue(new Error('plugin not available'))

    const nativeResponse = new Response('fallback', { status: 200 })
    ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(nativeResponse)

    const res = await httpFetchWithRedirect('http://example.com/api')
    expect(res.status).toBe(200)
    // With current code, native fetch is called with original 'http://example.com/api'
    // After fix, it should be called with the redirected URL
    expect(globalThis.fetch).toHaveBeenCalledWith('https://redirected.com/api', expect.anything())
  })

  it('should rethrow scope rejection even after a redirect', async () => {
    const redirectResponse = new Response(null, {
      status: 302,
      headers: { Location: 'https://redirected.com/api' },
    })
    _mockedTauriFetchModule.mockResolvedValueOnce(redirectResponse)
    _mockedTauriFetchModule.mockRejectedValueOnce(new Error('url not allowed on the configured scope'))

    await expect(
      httpFetchWithRedirect('http://example.com/api'),
    ).rejects.toThrow('url not allowed on the configured scope')

    // Should NOT fall back to native fetch
    expect(globalThis.fetch).not.toHaveBeenCalled()
  })
})

describe('uploadWithFiles', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('should construct multipart body and post', async () => {
    _mockedTauriFetchModule.mockResolvedValue(new Response('created', { status: 201 }))

    const file = new File(['content'], 'test.txt', { type: 'text/plain' })
    const res = await uploadWithFiles(
      'http://example.com/upload',
      { field1: 'value1' },
      [file],
      { Authorization: 'Bearer tok' },
    )

    expect(res.status).toBe(201)
    const call = _mockedTauriFetchModule.mock.calls[0]
    expect(call[0]).toBe('http://example.com/upload')
    expect((call[1] as any).method).toBe('POST')
    const hdrs = (call[1] as any).headers
    if (hdrs instanceof Headers) {
      expect(hdrs.get('Authorization')).toBe('Bearer tok')
      expect(hdrs.get('Content-Type')).toContain('multipart/form-data')
    } else {
      expect(hdrs['Authorization']).toBe('Bearer tok')
      expect(hdrs['Content-Type']).toContain('multipart/form-data')
    }
    expect((call[1] as any).body).toBeInstanceOf(Uint8Array)
  })
})
