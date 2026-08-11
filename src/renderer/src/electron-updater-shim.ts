/**
 * renderer 端 electron-updater 兼容 —— 类型占位，无真实更新逻辑
 */
export const autoUpdater = {
  checkForUpdates: async () => ({}),
  checkForUpdatesAndNotify: async () => ({}),
  downloadUpdate: async () => ({}),
  quitAndInstall: () => {},
  on: () => {},
  once: () => {},
  removeListener: () => {},
  removeAllListeners: () => {},
  event: () => {}
}
export const ProgressInfo = class {}
export const UpdateDownloadedEvent = class {}
export const UpdateInfo = class {}

export default { autoUpdater }