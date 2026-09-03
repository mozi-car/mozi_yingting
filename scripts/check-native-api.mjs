import fs from 'node:fs'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
console.log('Rebuilding all Rust native addons before API verification')
execFileSync(process.execPath, [path.join(root, 'scripts', 'build-native.mjs')], { cwd: root, stdio: 'inherit' })
const nativeDir = path.join(root, 'out', 'sidecar', 'native')
const addons = {
  sa: ['SeedKey'],
  candle: ['scanDevices', 'candle_dev_open', 'candle_dev_close', 'candle_channel_start', 'candle_channel_stop', 'SendCANMsg', 'CreateTSFN', 'FreeTSFN'],
  peak: ['LoadDll', 'CANTP_Initialize_2016', 'CANTP_Read_2016', 'CANTP_Write_2016', 'CreateTSFN', 'FreeTSFN'],
  kvaser: ['LoadDll', 'canOpenChannel', 'canRead', 'canWrite', 'CreateTSFN', 'FreeTSFN'],
  zlg: ['LoadDll', 'ZCAN_OpenDevice', 'ZCAN_Transmit', 'ZCAN_Receive', 'CreateTSFN', 'FreeTSFN'],
  vector: ['LoadDll', 'xlOpenPort', 'xlCanTransmitEx', 'xlCanReceive', 'CreateTSFN', 'FreeTSFN'],
  toomoss: ['LoadDll', 'USB_ScanDevice', 'CAN_Init', 'CAN_SendMsg', 'CAN_GetMsg', 'CreateTSFN', 'FreeTSFN'],
  kvaserLin: ['LoadDll', 'linOpenChannel', 'linReadMessageWait', 'linWriteMessage', 'CreateTSFN', 'FreeTSFN'],
  peakLin: ['LoadDll', 'LIN_Read', 'LIN_Write', 'LIN_RegisterClient'],
  toomossLin: ['LoadDll', 'USB_ScanDevice', 'LIN_EX_Init', 'LIN_EX_GetMsg', 'SendLinMsg', 'CreateTSFN', 'FreeTSFN'],
  vsomeip: ['Runtime', 'SomeipMessage', 'RegisterCallback', 'UnregisterCallback', 'Send', 'VsomeipCallbackWrapper', 'Application', 'offerService', 'stopOfferService', 'releaseService', 'requestService', 'subscribe', 'unsubscribe', 'clearAllHandler', 'registerMessageHandler', 'registerAvailabilityHandler', 'start', 'stop', 'sendMessage', 'startPeriodicMessage', 'stopPeriodicMessage', 'updatePeriodicMessage']
}

// Infer the complete runtime surface from the actual TypeScript call sites as
// well as the small smoke-test list above. This prevents a test from passing
// while a case-sensitive export used by production code is missing.
const sourceAliases = {
  peak: ['src/main/docan/peak/index.ts', 'peak'],
  candle: ['src/main/docan/candle/index.ts', 'Candle'],
  kvaser: ['src/main/docan/kvaser/index.ts', 'KV'],
  zlg: ['src/main/docan/zlg/index.ts', 'ZLG'],
  vector: ['src/main/docan/vector/index.ts', 'VECTOR'],
  toomoss: ['src/main/docan/toomoss/index.ts', 'TOOMOSS'],
  kvaserLin: ['src/main/dolin/kvaser/index.ts', 'LIN'],
  peakLin: ['src/main/dolin/peak/index.ts', 'LIN'],
  toomossLin: ['src/main/dolin/toomoss/index.ts', 'LIN'],
  vsomeip: ['src/main/vsomeip/client.ts', 'vsomeip']
}
for (const [name, [relative, alias]] of Object.entries(sourceAliases)) {
  const source = fs.readFileSync(path.join(root, relative), 'utf8')
    .replace(/\/\/.*$/gm, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
  const inferred = [...source.matchAll(new RegExp(`${alias}\\.([A-Za-z_][A-Za-z0-9_]*)`, 'g'))].map((match) => match[1])
  addons[name] = [...new Set([...addons[name], ...inferred])]
}
for (const relative of ['src/main/dolin/vector/index.ts']) {
  const source = fs.readFileSync(path.join(root, relative), 'utf8')
    .replace(/\/\/.*$/gm, '').replace(/\/\*[\s\S]*?\*\//g, '')
  addons.vector = [...new Set([...addons.vector, ...[...source.matchAll(/VECTOR\.([A-Za-z_][A-Za-z0-9_]*)/g)].map((match) => match[1])])]
}

if (!fs.existsSync(nativeDir)) throw new Error(`native addon directory missing: ${nativeDir}; run npm run build:native first`)
const require = createRequire(import.meta.url)
for (const [name, expected] of Object.entries(addons)) {
  const file = path.join(nativeDir, `${name}.node`)
  if (!fs.existsSync(file)) throw new Error(`${name}: release addon missing: ${file}`)
  const addon = require(file)
  const exported = new Set(Object.keys(addon))
  for (const value of Object.values(addon)) {
    if (typeof value === 'function') for (const key of Object.getOwnPropertyNames(value.prototype ?? {})) exported.add(key)
  }
  const missing = expected.filter((key) => !exported.has(key))
  if (missing.length) throw new Error(`${name}: missing runtime exports: ${missing.join(', ')}`)
  console.log(`${name}: loaded ${expected.length} required runtime exports`)
}
console.log('Native runtime API verification passed')
