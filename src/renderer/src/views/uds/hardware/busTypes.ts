import type { CanVendor } from 'nodeCan/can'
import type { UdsDevice } from 'nodeCan/uds'

export type BusType = 'can' | 'lin' | 'eth' | 'pwm' | 'serial'

export const BUS_TYPES: BusType[] = ['can', 'lin', 'eth', 'pwm', 'serial']

export const BUS_TYPE_PREFIX: Record<BusType, string> = {
  can: 'CAN',
  lin: 'LIN',
  eth: 'ETH',
  pwm: 'PWM',
  serial: 'SER'
}

/**
 * Which vendors are eligible for which bus type.
 * Mirrors the eligibility rules previously hard-coded in the legacy `addSubTree()`
 * function of hardware/index.vue, so behavior does not regress.
 */
export function vendorSupportsType(vendor: CanVendor, type: BusType): boolean {
  switch (type) {
    case 'can':
      return vendor !== 'ecubus'
    case 'lin':
      return (
        vendor == 'peak' || vendor == 'toomoss' || vendor == 'kvaser' || vendor == 'vector' || vendor == 'ecubus'
      )
    case 'eth':
      return vendor === 'simulate'
    case 'pwm':
      return vendor === 'ecubus'
    case 'serial':
      return vendor === 'ecubus'
    default:
      return false
  }
}

/**
 * IA (interface) child nodes are supported on every bus type. can/lin/pwm/eth/serial each have a
 * matching `Inter` variant and editor window (cani/lini/pwmi/ethi/seriali).
 */
export const IA_SUPPORTED_TYPES: BusType[] = ['can', 'lin', 'pwm', 'eth', 'serial']

export interface ChannelSummary {
  vendor?: string
  model?: string
  param1?: string
  param2?: string
}

function formatFreq(hz: number): string {
  if (hz >= 1000000 && hz % 1000000 === 0) return `${hz / 1000000}Mbps`
  if (hz >= 1000) return `${hz / 1000}kbps`
  return `${hz}bps`
}

/** Extracts the vendor/model/baudrate/resistor summary shown on a channel card. */
export function getDeviceSummary(device?: UdsDevice): ChannelSummary {
  if (!device) return {}
  if (device.type === 'can' && device.canDevice) {
    const d = device.canDevice
    const res =
      d.vendor === 'toomoss'
        ? d.toomossRes
        : d.vendor === 'zlg'
          ? d.zlgRes
          : d.vendor === 'candle'
            ? d.candleRes
            : undefined
    return {
      vendor: d.vendor,
      model: d.name,
      param1: formatFreq(d.bitrate.freq),
      param2: res === undefined ? undefined : res ? '120Ω 已启用' : '120Ω 未启用'
    }
  } else if (device.type === 'lin' && device.linDevice) {
    const d = device.linDevice
    return { vendor: d.vendor, model: d.name, param1: `${d.baudRate}bps` }
  } else if (device.type === 'eth' && device.ethDevice) {
    const d = device.ethDevice
    return { vendor: d.vendor, model: d.name }
  } else if (device.type === 'pwm' && device.pwmDevice) {
    const d = device.pwmDevice
    return { vendor: d.vendor, model: d.name, param1: `${d.freq}Hz` }
  } else if (device.type === 'serial' && device.serialDevice) {
    const d = device.serialDevice
    return { vendor: d.vendor, model: d.name, param1: `${d.baudRate}bps` }
  }
  return {}
}
