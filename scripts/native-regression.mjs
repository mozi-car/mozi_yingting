import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const nativeDir = path.join(root, 'out', 'sidecar', 'native')
const require = createRequire(import.meta.url)
const missing = path.join(root, 'out', 'does-not-exist', 'vendor.dll')
const loaded = new Map()
function addon(name) {
  const value = require(path.join(nativeDir, `${name}.node`))
  loaded.set(name, value)
  return value
}
function expectThrow(label, fn) {
  let thrown = false
  try { fn() } catch { thrown = true }
  if (!thrown) throw new Error(`${label}: expected an error`)
}
function checkMissingDll(name) {
  const value = addon(name)
  if (typeof value.LoadDll === 'function') expectThrow(`${name}.LoadDll`, () => value.LoadDll(missing))
}

if (!fs.existsSync(nativeDir)) throw new Error(`native output missing: ${nativeDir}`)
for (const name of ['candle', 'peak', 'kvaser', 'zlg', 'vector', 'kvaserLin', 'peakLin', 'toomossLin', 'toomoss']) checkMissingDll(name)
const sa = addon('sa')
const seedKey = new sa.SeedKey()
expectThrow('sa.LoadDLL', () => seedKey.LoadDLL(missing))
if (seedKey.IsLoaded()) throw new Error('sa: failed LoadDLL must not leave loaded state')

// Wrapper-level checks exercise the JS/Rust boundary without pretending that a
// missing vendor device is present.
const peak = loaded.get('peak') ?? addon('peak')
const msg = new peak.cantp_msg()
msg.can_info = { can_id: 0x123, can_msgtype: 0, dlc: 8 }
if (msg.can_info.can_id !== 0x123) throw new Error('peak: can_info round-trip failed')
if (!peak.CANTP_StatusIsOk_2016(0) || peak.CANTP_StatusIsOk_2016(7)) throw new Error('peak: status semantics failed')
expectThrow('peak.StartPeriodSend without DLL', () => peak.StartPeriodSend('test', { handle: 0, id: 1, data: [] }, 1))

const candle = loaded.get('candle') ?? addon('candle')
const candleBytes = new candle.Uint8Array(2)
candleBytes.setitem(0, 0xab)
if (candleBytes.getitem(0) !== 0xab) throw new Error('candle: byte wrapper round-trip failed')

const kvaser = loaded.get('kvaser') ?? addon('kvaser')
const kvaserBytes = new kvaser.ByteArray(2)
kvaserBytes.setitem(0, 0xcd)
if (kvaserBytes.getitem(0) !== 0xcd) throw new Error('kvaser: byte wrapper round-trip failed')
expectThrow('kvaser.StartPeriodSend without DLL', () => kvaser.StartPeriodSend('0', { id: 1, data: [] }, 1, 0))

const vector = loaded.get('vector') ?? addon('vector')
const vectorEvent = new vector.s_xl_event()
vectorEvent.canId = 0x123
vectorEvent.data = [1, 2, 3]
if (vectorEvent.canId !== 0x123 || vectorEvent.data.length !== 3) throw new Error('vector: event round-trip failed')
expectThrow('vector.StartPeriodSend without DLL', () => vector.StartPeriodSend('test', { port: 0, mask: 0, data: [] }, 1, 0))

const peakLin = loaded.get('peakLin') ?? addon('peakLin')
const linClient = new peakLin.HLINCLIENT_JS()
if (linClient.value() !== 0 || peakLin.hwpName !== 1) throw new Error('peakLin: output wrapper/constants failed')

const serial = addon('serial')
if (typeof serial.Serial !== 'function') throw new Error('serial: class export missing')

// A vendor DLL can be supplied by a hardware CI runner. In that mode we at
// least verify LoadDll/IsLoaded and make failures fatal; normal CI deliberately
// does not invent a DLL or a device.
for (const name of ['candle', 'peak', 'kvaser', 'zlg', 'vector', 'kvaserLin', 'peakLin', 'toomossLin', 'toomoss']) {
  const dll = process.env[`YT_VENDOR_DLL_${name.toUpperCase()}`]
  if (!dll) continue
  const value = loaded.get(name) ?? addon(name)
  value.LoadDll(dll)
  if (typeof value.IsLoaded === 'function' && !value.IsLoaded()) throw new Error(`${name}: vendor DLL did not report loaded`)
}
console.log(`Native behavior regression passed for ${loaded.size} addons (vendor DLL checks run when YT_VENDOR_DLL_* is set)`)
