/**
 * 婴听 renderer shim —— 在 WebView 里模拟 electron preload 暴露的全局对象
 * window.electron / window.api / window.store / window.path
 * 底层全部转发到 Tauri → Rust 桥 → Node sidecar
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import path from 'path-browserify'

declare global {
  interface Window {
    electron: any
    api: any
    store: any
    path: any
    serviceDetail: any
  }
}

// ---------- 事件分发（sidecar emit → renderer on） ----------
const listeners = new Map<string, Set<Function>>()
let tauriListenStarted = false

function startTauriListen() {
  if (tauriListenStarted) return
  tauriListenStarted = true
  listen('yt-sidecar-event', (event) => {
    const { channel, payload } = (event.payload as any) ?? {}
    if (typeof channel !== 'string') return
    const cbs = listeners.get(channel)
    if (!cbs) return
    const syntheticEvent = {
      channel,
      payload,
      data: payload,
      sender: undefined,
      returnValue: undefined
    }
    for (const cb of [...cbs]) {
      try {
        cb(syntheticEvent)
      } catch (e) {
        console.error('[shim:event]', channel, e)
      }
    }
  }).catch((e) => console.error('[shim] listen failed:', e))
}

function registerListener(channel: string, cb: Function): () => void {
  startTauriListen()
  if (!listeners.has(channel)) listeners.set(channel, new Set())
  listeners.get(channel)!.add(cb)
  return () => listeners.get(channel)?.delete(cb)
}

// ---------- sendSync 兼容（仅少量初始化调用） ----------
const SYNC_STUBS: Record<string, (...args: any[]) => any> = {
  'ipc-get-casdoor-config': () => null,
  'ipc-plugin-lib-path': () => '',
  'ipc-service-detail': () => null
}

function syncFallback(channel: string, args: any[]) {
  if (channel === 'electron-store-get') {
    const key = args[0]
    return storeGet(key)
  }
  if (channel === 'electron-store-set') {
    const [key, val] = args
    storeSet(key, val)
    return undefined
  }
  const stub = SYNC_STUBS[channel]
  if (stub) return stub(...args)
  console.warn('[shim:sendSync] unhandled:', channel, args)
  return null
}

// ---------- store（localStorage 实现，兼容 electron-store 语义） ----------
const storePrefix = 'yt-store:'
function storeGet(key: string) {
  const raw = localStorage.getItem(storePrefix + key)
  if (raw === null) return undefined
  try {
    return JSON.parse(raw)
  } catch {
    return raw
  }
}
function storeSet(key: string, val: any) {
  localStorage.setItem(storePrefix + key, JSON.stringify(val))
}

// ---------- ipcRenderer ----------
const ipcRenderer = {
  invoke: (channel: string, ...args: any[]) =>
    invoke('rpc_invoke', { channel, args }).then((r) => (r ?? undefined) as any),
  send: (channel: string, ...args: any[]) => {
    invoke('rpc_invoke', { channel, args }).catch((e) =>
      console.error('[shim:send]', channel, e)
    )
  },
  sendSync: (channel: string, ...args: any[]) => syncFallback(channel, args),
  on: (channel: string, cb: Function) => registerListener(channel, cb),
  once: (channel: string, cb: Function) => {
    const un = registerListener(channel, (e: any) => {
      un()
      cb(e)
    })
    return un
  },
  removeListener: (channel: string, cb: Function) => {
    listeners.get(channel)?.delete(cb)
  },
  removeAllListeners: (channel?: string) => {
    if (channel) listeners.delete(channel)
    else listeners.clear()
  }
}

// ---------- api ----------
const api = {
  glob: (pattern: string | string[], options?: any) =>
    invoke('rpc_invoke', { channel: 'ipc-glob', args: [pattern, options] }),
  readdir: (p: string) => invoke('rpc_invoke', { channel: 'ipc-fs-readdir', args: [p] }),
  state: (p: string) => invoke('rpc_invoke', { channel: 'ipc-fs-stat', args: [p] }),
  getPort: (_id: string) => {
    console.warn('[shim:getPort] MessagePort 通道在 Tauri 下暂不支持')
  }
}

// ---------- store ----------
const store = {
  get: (key: string) => storeGet(key),
  set: (property: string, val: any) => storeSet(property, val)
}

// ---------- 注入全局 ----------
if (!window.electron) {
  window.electron = {
    ipcRenderer,
    webFrame: {
      setZoomFactor: (f: number) => {
        document.documentElement.style.setProperty('zoom', String(f / 100))
      },
      setZoomLevel: () => {}
    },
    webUtils: {
      getPathForFile: () => ''
    },
    process: {
      versions: { node: '', chrome: '', electron: '0.0.0' },
      platform: navigator.platform.toLowerCase().includes('win')
        ? 'win32'
        : navigator.platform.toLowerCase().includes('mac')
          ? 'darwin'
          : 'linux',
      env: {}
    }
  }
}
if (!window.api) window.api = api
if (!window.store) window.store = store
if (!window.path) window.path = path

export default { ipcRenderer, api, store, path }
