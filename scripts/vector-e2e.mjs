import path from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const require = createRequire(import.meta.url)
const vector = require(path.join(root, 'out', 'sidecar', 'native', 'vector.node'))
const dll = process.env.YT_VENDOR_DLL_VECTOR || path.join(root, 'resources', 'lib', 'vxlapi64.dll')
const result = { dll, steps: [], loopback: null }
const step = (name, value) => {
  result.steps.push({ name, value })
  console.log(`${name}: ${JSON.stringify(value)}`)
  return value
}
const must = (name, value) => {
  step(name, value)
  if (value !== 0) throw new Error(`${name} failed with status ${value}`)
}

vector.LoadDll(dll)
must('xlOpenDriver', vector.xlOpenDriver())
try {
  const config = new vector.XL_DRIVER_CONFIG()
  must('xlGetDriverConfig', vector.xlGetDriverConfig(config))
  const channels = Array.from({ length: config.channelCount }, (_, i) => config.getitem(i))
  step('channels', channels.map((c) => ({ name: c.name, channelIndex: c.channelIndex, channelMask: c.channelMask, hwType: c.hwType, busType: c.busParams.busType })))

  // Prefer a real CAN channel for the physical connection proof.
  const physical = channels.find((c) => c.hwType !== 1 && c.busParams.busType === 1)
  if (physical) {
    const port = new vector.XLPORTHANDLE()
    const access = new vector.XLACCESS()
    access.assign(physical.channelMask)
    must(`physical.xlOpenPort(${physical.name})`, vector.xlOpenPort(port, 'yingting-rust-e2e', physical.channelMask, access, 4096, 3, 1))
    try {
      const params = new vector.XLchipParams()
      params.bitRate = 500000
      params.sjw = 1
      params.tseg1 = 6
      params.tseg2 = 1
      params.sam = 1
      must('physical.xlCanSetChannelParams', vector.xlCanSetChannelParams(port.value, physical.channelMask, params))
      must('physical.xlActivateChannel', vector.xlActivateChannel(port.value, physical.channelMask))
      step('physical.permissionMask', access.value)
      step('physical.status', 'open/configured/activated')
      must('physical.xlDeactivateChannel', vector.xlDeactivateChannel(port.value, physical.channelMask))
    } finally {
      must('physical.xlClosePort', vector.xlClosePort(port.value))
    }
  }

  // Virtual Channel 1 provides a deterministic vendor-driver loopback without
  // pretending that a physical CAN bus peer is present.
  const virtual = channels.find((c) => c.hwType === 1 && c.busParams.busType === 1)
  if (!virtual) throw new Error('No Vector virtual CAN channel available for deterministic loopback')
  const port = new vector.XLPORTHANDLE()
  const access = new vector.XLACCESS()
  access.assign(virtual.channelMask)
  must(`loopback.xlOpenPort(${virtual.name})`, vector.xlOpenPort(port, 'yingting-rust-loopback', virtual.channelMask, access, 4096, 3, 1))
  try {
    must('loopback.xlActivateChannel', vector.xlActivateChannel(port.value, virtual.channelMask))
    const tx = new vector.s_xl_event()
    tx.tag = 10
    tx.canId = 0x5a5
    tx.dlc = 8
    tx.flags = 0
    tx.data = [0xde, 0xad, 0xbe, 0xef, 1, 2, 3, 4]
    must('loopback.xlCanTransmit', vector.xlCanTransmit(port.value, virtual.channelMask, 1, tx))

    let received = null
    let lastError = null
    for (let i = 0; i < 20 && !received; i++) {
      try {
        const frame = vector.receiveClassicEvent(port.value)
        if (frame.tag === 1 && frame.canId === 0x5a5) received = frame
      } catch (error) {
        lastError = String(error?.message ?? error)
      }
    }
    if (!received) throw new Error(`loopback receive failed: ${lastError ?? 'no matching frame'}`)
    result.loopback = { tag: received.tag, canId: received.canId, dlc: received.dlc, data: received.data }
    step('loopback.received', result.loopback)
  } finally {
    must('loopback.xlDeactivateChannel', vector.xlDeactivateChannel(port.value, virtual.channelMask))
    must('loopback.xlClosePort', vector.xlClosePort(port.value))
  }
} finally {
  must('xlCloseDriver', vector.xlCloseDriver())
}

console.log('Vector Rust driver physical lifecycle and deterministic virtual loopback passed')
