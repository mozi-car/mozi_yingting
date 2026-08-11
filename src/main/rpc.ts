/**
 * 婴听 sidecar RPC —— 替代 Electron ipcMain 的注册器
 *
 * 协议（stdout/stdin，行分隔 JSON）：
 *   req : {"id":1,"method":"invoke","params":["channel",[args...]]}
 *   resp: {"id":1,"result":<json>} | {"id":1,"error":"..."}
 *   emit: {"method":"emit","params":["channel",payload]}
 *   ctl : {"method":"shutdown"}
 */
import { EventEmitter } from 'events'
import { Readline } from 'node:readline'
import * as readline from 'node:readline'
import { platform } from 'process'

type Handler = (event: any, ...args: any[]) => any

const handlers = new Map<string, Handler>()
const eventEmitter = new EventEmitter()

export const rpc = {
  handle(channel: string, handler: Handler) {
    handlers.set(channel, handler)
  },
  on(channel: string, handler: Handler) {
    handlers.set(channel, handler)
  },
  once(channel: string, handler: Handler) {
    handlers.set(channel, (event, ...args) => {
      handlers.delete(channel)
      return handler(event, ...args)
    })
  },
  removeHandler(channel: string) {
    handlers.delete(channel)
  },
  emit(channel: string, ...args: any[]) {
    write({ method: 'emit', params: [channel, ...args] })
  }
}

/** 主进程(TS) -> renderer 的调用（原 webContents.send 语义） */
export const sendToRenderer = rpc.emit

export function getHandler(channel: string): Handler | undefined {
  return handlers.get(channel)
}

function write(obj: any) {
  process.stdout.write(JSON.stringify(obj) + '\n')
}

// 退出前 flush stdout（防止大响应被 process.exit 截断）
function gracefulExit() {
  process.stdout.end(() => process.exit(0))
}

let sequence = 0
const inflight = new Map<number, { resolve: (v: any) => void; reject: (e: any) => void }>()

/** invoke 另一个 rpc 通道（sidecar 内部互调，第一版很少用） */
export function invoke(channel: string, ...args: any[]): Promise<any> {
  return new Promise((resolve, reject) => {
    const id = ++sequence
    inflight.set(id, { resolve, reject })
    write({ id, method: 'invoke', params: [channel, args] })
  })
}

/** 伪造的 event 对象，兼容原 ipcMain.handle 的 event.sender.send */
export function createFakeEvent(channel: string) {
  return {
    channel,
    sender: {
      send: (ch: string, ...args: any[]) => rpc.emit(ch, ...args),
      id: 1
    },
    returnValue: null,
    reply: (ch: string, ...args: any[]) => rpc.emit(ch, ...args)
  }
}

export function startRpc(): void {
  const rl = readline.createInterface({
    input: process.stdin,
    terminal: false
  }) as Readline

  rl.on('line', (line: string) => {
    if (!line.trim()) return
    let msg: any
    try {
      msg = JSON.parse(line)
    } catch {
      return
    }

    if (msg.method === 'shutdown') {
      gracefulExit()
      return
    }

    if (msg.method === 'invoke') {
      const [channel, argv] = msg.params as [string, any]
      // 兼容 bridge 拍平单参（params:[ch,arg]）与数组（params:[ch,[a,b]]）两种形态
      const rest: any[] = Array.isArray(argv) ? argv : [argv]
      const handler = handlers.get(channel)
      const reply = (result: any) => write({ id: msg.id, result })
      const replyErr = (err: any) => write({ id: msg.id, error: String(err?.message ?? err) })
      if (!handler) {
        replyErr(`No handler registered for channel: ${channel}`)
        return
      }
      Promise.resolve()
        .then(() => handler(createFakeEvent(channel), ...rest))
        .then(reply)
        .catch(replyErr)
    }
  })

  rl.on('close', () => {
    gracefulExit()
  })

  write({ method: 'ready', params: [platform] })
}

// 拦截 renderer 可见的平台信息（原 process.versions）
export const platformInfo = {
  platform
}
