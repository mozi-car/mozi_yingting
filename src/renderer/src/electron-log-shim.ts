/**
 * renderer 端 electron-log 兼容 —— 输出到 console，关键错误可选转发
 */
const consoleMap: Record<string, (...args: any[]) => void> = {
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  verbose: console.debug,
  silly: console.debug
}

const level = (lvl: string) => (...args: any[]) => {
  const fn = consoleMap[lvl] ?? console.log
  fn(`[${lvl.toUpperCase()}]`, ...args)
}

const log = {
  error: level('error'),
  warn: level('warn'),
  info: level('info'),
  debug: level('debug'),
  verbose: level('verbose'),
  silly: level('silly'),
  sendEvent: (name: string, data?: any) => {
    console.info('[event]', name, data ?? '')
  },
  transports: {
    console: { level: 'debug' }
  }
}

export default log
export { log }
export const error = log.error
export const warn = log.warn
export const info = log.info
export const debug = log.debug
export const verbose = log.verbose
export const silly = log.silly
export const sendEvent = log.sendEvent
export const transports = log.transports
export const initialize = () => {}
