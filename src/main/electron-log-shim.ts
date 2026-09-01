/**
 * sidecar 端 electron-log 兼容 —— console + 文件输出
 */
import fs from 'fs'
import path from 'path'
import os from 'os'

const logRoot = process.env.LOCALAPPDATA || process.env.XDG_STATE_HOME || os.homedir()
const logDir = path.join(logRoot, 'yingting', 'logs')
let logFile: fs.WriteStream | null = null

try {
  fs.mkdirSync(logDir, { recursive: true })
  logFile = fs.createWriteStream(path.join(logDir, 'yingting.log'), { flags: 'a' })
} catch {
  logFile = null
}

const ts = () => new Date().toISOString()

function write(lvl: string, args: any[]) {
  const msg = args
    .map((a) => (typeof a === 'string' ? a : JSON.stringify(a) ?? String(a)))
    .join(' ')
  const line = `${ts()} [${lvl.toUpperCase()}] ${msg}`
  if (logFile) logFile.write(line + '\n')
  console.error(`[${lvl.toUpperCase()}]`, ...args)
}

const level = (lvl: string) => (...args: any[]) => write(lvl, args)

const log: any = {
  error: level('error'),
  warn: level('warn'),
  info: level('info'),
  debug: level('debug'),
  verbose: level('verbose'),
  silly: level('silly'),
  initialize: () => {},
  getLevel: () => 'info',
  transports: {
    file: { level: 'info' },
    console: { level: 'debug' }
  },
  main: null
}

log.default = log
log.main = log

export default log
export { log }
export const error = log.error
export const warn = log.warn
export const info = log.info
export const debug = log.debug
export const verbose = log.verbose
export const silly = log.silly
export const initialize = log.initialize
