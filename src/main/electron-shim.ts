/**
 * Electron API shim —— sidecar 模式下被 esbuild alias 到 'electron'
 * 保持原 API 签名，底层走 rpc.ts
 */
import { rpc, sendToRenderer, invoke } from './rpc'
import path from 'path'
import { platform, versions, env } from 'process'

export const app = {
  getVersion: () => env.YT_VERSION || '0.1.0',
  getPath: (name: string) => {
    const cwd = process.cwd()
    switch (name) {
      case 'userData':
        return path.join(cwd, 'userData')
      case 'userCache':
      case 'cache':
        return path.join(cwd, 'cache')
      case 'temp':
        return require('os').tmpdir()
      case 'documents':
      case 'downloads':
        return require('os').homedir()
      default:
        return cwd
    }
  },
  getGPUFeatureStatus: () => ({}),
  getLocale: () => 'zh-CN',
  quit: () => process.exit(0),
  exit: () => process.exit(0),
  whenReady: () => Promise.resolve(),
  on: () => {},
  once: () => {},
  isPackaged: false,
  setAppUserModelId: () => {},
  requestSingleInstanceLock: () => true,
  setAsDefaultProtocolClient: () => false,
  removeAsDefaultProtocolClient: () => false,
  isDefaultProtocolClient: () => false
}

export const shell = {
  openExternal: async (url: string) => {
    console.log('[shell] openExternal:', url)
  },
  openPath: async (p: string) => {
    console.log('[shell] openPath:', p)
    return ''
  },
  showItemInFolder: () => {},
  trashItem: async () => {}
}

export const dialog = {
  showOpenDialog: async () => ({ canceled: true, filePaths: [] }),
  showSaveDialog: async () => ({ canceled: true, filePath: undefined }),
  showMessageBox: async () => ({ response: 0, checkboxChecked: false }),
  showErrorBox: () => {}
}

export class BrowserWindow {
  static getAllWindows() {
    return []
  }
  constructor() {}
  loadURL() {}
  loadFile() {}
  show() {}
  hide() {}
  close() {}
  destroy() {}
  minimize() {}
  maximize() {}
  unmaximize() {}
  isMaximized() {
    return false
  }
  isMinimized() {
    return false
  }
  on() {
    return this
  }
  once() {
    return this
  }
  webContents = {
    send: (_ch: string, ..._args: any[]) => {},
    postMessage: () => {},
    on: () => {}
  }
  getBounds() {
    return { x: 0, y: 0, width: 1440, height: 900 }
  }
  setBounds() {}
  setMenuBarVisibility() {}
  setTitle() {}
  getTitle() {
    return ''
  }
  static getFocusedWindow() {
    return null
  }
}

export class WebContents {
  static getAllWebContents() {
    return []
  }
}

export const ipcMain = {
  handle: rpc.handle.bind(rpc),
  on: rpc.on.bind(rpc),
  once: rpc.once.bind(rpc),
  removeHandler: rpc.removeHandler.bind(rpc),
  removeAllListeners: () => {},
  emit: (_ch: string, ..._args: any[]) => {}
}

export const ipcRenderer = {
  invoke,
  send: (_ch: string, ..._args: any[]) => {},
  on: () => {},
  once: () => {},
  removeListener: () => {},
  removeAllListeners: () => {}
}

export const contextBridge = {
  exposeInMainWorld: () => {}
}

export const protocol = {
  registerSchemesAsPrivileged: () => {},
  handle: () => {},
  unhandle: () => {}
}

export const net = {
  fetch: (url: string, opts?: any) => fetch(url, opts)
}

export const clipboard = {
  writeText: () => {},
  readText: () => ''
}

export const nativeTheme = {
  shouldUseDarkColors: false,
  on: () => {},
  themeSource: 'system'
}

export const screen = {
  getPrimaryDisplay: () => ({
    workArea: { x: 0, y: 0, width: 1920, height: 1080 },
    size: { width: 1920, height: 1080 }
  }),
  getAllDisplays: () => []
}

export const globalShortcut = {
  register: () => true,
  unregister: () => {}
}

export class MessageChannelMain {
  port1: any
  port2: any
  constructor() {
    this.port1 = { on: () => {}, start: () => {}, postMessage: () => {}, close: () => {} }
    this.port2 = { on: () => {}, start: () => {}, postMessage: () => {}, close: () => {} }
  }
}

export const MessagePortMain = class {
  on() {}
  start() {}
  postMessage() {}
  close() {}
}

export const systemPreferences = {
  getAccentColor: () => '#409eff'
}

export const safeStorage = {
  isEncryptionAvailable: () => true,
  encryptString: (plain: string) => Buffer.from(plain, 'utf8').toString('base64'),
  decryptString: (cipher: string) => Buffer.from(cipher, 'base64').toString('utf8')
}

export const session = {
  defaultSession: {
    setPermissionRequestHandler: () => {},
    on: () => {},
    cookies: { set: async () => {}, get: async () => [] }
  }
}

export const powerMonitor = {
  on: () => {}
}

export const tray = null

export const Notification = class {}

export const webContents = {
  send: sendToRenderer
}

export const ipcRendererModule = {
  sendToRenderer
}

export default {
  app,
  shell,
  dialog,
  BrowserWindow,
  ipcMain,
  contextBridge,
  protocol,
  net,
  clipboard,
  nativeTheme,
  screen,
  globalShortcut,
  MessageChannelMain,
  session,
  powerMonitor,
  versions,
  platform
}
