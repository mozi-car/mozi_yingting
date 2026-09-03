import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const nativeDir = path.join(root, 'out', 'sidecar', 'native')
const require = createRequire(import.meta.url)
const results = []
const record = (module, operation, value) => results.push({ module, operation, result: value === undefined ? { ok: true } : value })
const call = (module, operation, fn) => {
  try {
    const result = fn()
    record(module, operation, result)
    return result
  } catch (error) {
    record(module, operation, { error: String(error?.message ?? error) })
    return undefined
  }
}
function load(module, envName) {
  const dll = process.env[envName]
  if (!dll) return null
  const value = require(path.join(nativeDir, `${module}.node`))
  value.LoadDll(dll)
  record(module, 'LoadDll/IsLoaded', typeof value.IsLoaded === 'function' ? value.IsLoaded() : true)
  return value
}

const candle = require(path.join(nativeDir, 'candle.node'))
call('candle', 'scanDevices', () => candle.scanDevices())
const peak = load('peak', 'YT_VENDOR_DLL_PEAK')
if (peak) {
  call('peak', 'GetValue(channel-condition)', () => peak.CANTP_GetValue_2016(0, peak.PCANTP_PARAMETER_CHANNEL_CONDITION, Buffer.alloc(1)))
  call('peak', 'Uninitialize(invalid-or-unused-channel)', () => peak.CANTP_Uninitialize_2016(0))
}
const kvaser = load('kvaser', 'YT_VENDOR_DLL_KVASER')
if (kvaser) {
  call('kvaser', 'canInitializeLibrary', () => kvaser.canInitializeLibrary())
  call('kvaser', 'canGetNumberOfChannels', () => kvaser.canGetNumberOfChannels())
  call('kvaser', 'canUnloadLibrary', () => kvaser.canUnloadLibrary())
}
const zlg = load('zlg', 'YT_VENDOR_DLL_ZLG')
if (zlg) {
  call('zlg', 'ZCAN_OpenDevice(invalid-safe)', () => zlg.ZCAN_OpenDevice(0, 0, 0))
}
const vector = load('vector', 'YT_VENDOR_DLL_VECTOR')
if (vector) {
  call('vector', 'xlOpenDriver', () => vector.xlOpenDriver())
  const config = new vector.XL_DRIVER_CONFIG()
  call('vector', 'xlGetDriverConfig', () => vector.xlGetDriverConfig(config))
  if (config.channelCount > 0) {
    const channels = []
    for (let i = 0; i < config.channelCount; i++) {
      const channel = config.getitem(i)
      channels.push({ name: channel.name, hwType: channel.hwType, hwIndex: channel.hwIndex, hwChannel: channel.hwChannel, channelIndex: channel.channelIndex, channelMask: channel.channelMask, serialNumber: channel.serialNumber, channelCapabilities: channel.channelCapabilities, channelBusCapabilities: channel.channelBusCapabilities })
    }
    record('vector', 'xlGetDriverConfig/channels', channels)
    call('vector', 'listEthernetChannels', () => vector.listEthernetChannels().map((item) => ({ name: item.name, channelIndex: item.channelIndex, channelMask: item.channelMask })))
    const channel = Array.from({ length: config.channelCount }, (_, index) => config.getitem(index))
      .find((item) => (item.channelBusCapabilities & 0x00010001) === 0x00010001) ?? config.getitem(0)
    const port = new vector.XLPORTHANDLE()
    const permission = new vector.XLACCESS()
    permission.assign(channel.channelMask)
    const openStatus = call('vector', `xlOpenPort(${channel.name})`, () => vector.xlOpenPort(port, 'yingting-hardware-smoke', channel.channelMask, permission, 4096, 3, 1))
    if (openStatus === 0 && port.value >= 0) {
      call('vector', 'xlCanSetChannelParams(500k)', () => {
        const params = new vector.XLchipParams()
        params.bitRate = 500000
        params.sjw = 1
        params.tseg1 = 6
        params.tseg2 = 1
        params.sam = 1
        return vector.xlCanSetChannelParams(port.value, channel.channelMask, params)
      })
      call('vector', 'xlActivateChannel', () => vector.xlActivateChannel(port.value, channel.channelMask))
      if (process.env.YT_VECTOR_E2E === '1') {
        call('vector', 'xlCanTransmitEx(test-frame)', () => {
          const frame = new vector.XLcanTxEvent()
          frame.tag = 0x0440
          frame.canId = 0x123
          frame.msgFlags = 0
          frame.dlc = 8
          frame.data = [0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0]
          return vector.xlCanTransmitEx(port.value, channel.channelMask, 1, 1, frame)
        })
      }
      call('vector', 'xlCanReceive(no-frame-safe)', () => vector.xlCanReceive(port.value))
      call('vector', 'xlDeactivateChannel', () => vector.xlDeactivateChannel(port.value, channel.channelMask))
      call('vector', 'xlClosePort', () => vector.xlClosePort(port.value))
    }
  }
  call('vector', 'xlCloseDriver', () => vector.xlCloseDriver())
}
const toomoss = load('toomoss', 'YT_VENDOR_DLL_TOOMOSS')
if (toomoss) {
  const handles = new toomoss.I32Array(16)
  call('toomoss', 'USB_ScanDevice', () => toomoss.USB_ScanDevice(handles))
}
const kvaserLin = load('kvaserLin', 'YT_VENDOR_DLL_KVASERLIN')
if (kvaserLin) {
  call('kvaserLin', 'linInitializeLibrary', () => kvaserLin.linInitializeLibrary())
  call('kvaserLin', 'linUnloadLibrary', () => kvaserLin.linUnloadLibrary())
}
const peakLin = load('peakLin', 'YT_VENDOR_DLL_PEAKLIN')
if (peakLin) call('peakLin', 'LIN_GetAvailableHardware', () => peakLin.LIN_GetAvailableHardware())
const toomossLin = load('toomossLin', 'YT_VENDOR_DLL_TOOMOSSLIN')
if (toomossLin) {
  const handles = new toomossLin.I32Array(16)
  call('toomossLin', 'USB_ScanDevice', () => toomossLin.USB_ScanDevice(handles))
}

const logPath = process.env.YT_VENDOR_SMOKE_LOG || path.join(root, 'out', 'vendor-smoke.json')
fs.mkdirSync(path.dirname(logPath), { recursive: true })
fs.writeFileSync(logPath, JSON.stringify({ generatedAt: new Date().toISOString(), platform: process.platform, results }, null, 2))
console.log(`Vendor smoke log written: ${logPath}`)
for (const item of results) console.log(`${item.module}.${item.operation}: ${JSON.stringify(item.result)}`)
