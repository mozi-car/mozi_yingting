/**
 * 婴听 sidecar 入口 —— 替代 Electron main 进程
 * 仅做业务初始化，不创建窗口（窗口由 Tauri 壳负责）
 */
import './ipc'
import 'src/renderer/src/helper'
import { ipcMain } from 'electron'
import { store } from './store'
import { startRpc } from './rpc'
import { initMainI18n } from './i18n'
import { setupCasdoor } from './ipc/casdoor'
import log from 'electron-log/main'
import { createLogs } from './log'
import Transport from 'winston-transport'
import fs from 'fs'
import path from 'path'

log.initialize()

// winston sysLog/scriptLog 文件传输器
const logDir = path.join(process.cwd(), 'logs')
try {
  fs.mkdirSync(logDir, { recursive: true })
} catch {
  /* ignore */
}

class SidecarTransport extends Transport {
  private stream: fs.WriteStream | null = null
  constructor(opts?: Transport.TransportStreamOptions) {
    super(opts)
    try {
      this.stream = fs.createWriteStream(path.join(logDir, 'syslog.jsonl'), { flags: 'a' })
    } catch {
      this.stream = null
    }
  }
  log(info: any, callback: () => void) {
    if (this.stream) {
      this.stream.write(JSON.stringify(info) + '\n')
    }
    callback()
  }
}

createLogs(
  [
    () =>
      new SidecarTransport({
        level: 'debug'
      })
  ],
  []
)

try {
  initMainI18n()
} catch (e) {
  log.error('[initMainI18n]', e)
}

try {
  setupCasdoor()
} catch (e) {
  log.error('[setupCasdoor]', e)
}

// electron-store 兼容通道（renderer 的 window.store）
ipcMain.on('electron-store-get', (event: any, key: string) => {
  event.returnValue = store.get(key)
})
ipcMain.on('electron-store-set', (event: any, key: string, val: any) => {
  store.set(key, val)
})

startRpc()
