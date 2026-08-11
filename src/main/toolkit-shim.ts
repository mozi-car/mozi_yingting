/** @electron-toolkit/utils shim */
export const is = {
  dev: !!process.env.VITE_DEV_SERVER_URL || process.env.NODE_ENV !== 'production',
  linux: process.platform === 'linux',
  mac: process.platform === 'darwin',
  win: process.platform === 'win32',
  main: true,
  preload: false,
  renderer: false,
  macAppStore: false,
  windowsStore: false
}

export const electronApp = {
  setAppUserModelId: () => {},
  setAppUserModelIdForProcessByEnv: () => {},
  isElectronAppReady: () => {}
}

export const optimizer = {
  watchWindowShortcuts: () => {},
  disableF12InProduction: () => {}
}

export default { is, electronApp, optimizer }
