import { UdsDevice } from '../share/uds'
import { PEAK_TP } from './peak'
import path from 'path'
import { ZLG_CAN } from './zlg'
import { KVASER_CAN } from './kvaser'
import { SIMULATE_CAN } from './simulate'
import { TOOMOSS_CAN } from './toomoss'
import { VECTOR_CAN } from './vector'
import { CanBaseInfo } from '../share/can'
import { CanBase } from './base'
import { SLCAN_CAN } from './slcan'
import { Candle_CAN } from './candle'

const libPath = process.env.YT_RESOURCES
  ? path.join(process.env.YT_RESOURCES, 'resources', 'lib')
  : path.resolve(process.cwd(), 'resources', 'lib')
for (const loader of [PEAK_TP.loadDllPath, ZLG_CAN.loadDllPath, KVASER_CAN.loadDllPath, TOOMOSS_CAN.loadDllPath, VECTOR_CAN.loadDllPath]) {
  try { loader(libPath) } catch (e) { console.error('[can] driver unavailable:', e) }
}

export function openCanDevice(canDevice: CanBaseInfo) {
  let canBase: CanBase | undefined

  if (canDevice.vendor == 'peak') {
    canBase = new PEAK_TP(canDevice)
  } else if (canDevice.vendor == 'zlg') {
    canBase = new ZLG_CAN(canDevice)
  } else if (canDevice.vendor == 'kvaser') {
    canBase = new KVASER_CAN(canDevice)
  } else if (canDevice.vendor == 'simulate') {
    canBase = new SIMULATE_CAN(canDevice)
  } else if (canDevice.vendor == 'toomoss') {
    canBase = new TOOMOSS_CAN(canDevice)
  } else if (canDevice.vendor == 'vector') {
    canBase = new VECTOR_CAN(canDevice)
  } else if (canDevice.vendor == 'slcan') {
    canBase = new SLCAN_CAN(canDevice)
  } else if (canDevice.vendor == 'candle') {
    canBase = new Candle_CAN(canDevice)
  }

  return canBase
}

export function getCanVersion(vendor: string) {
  vendor = vendor.toUpperCase()
  if (vendor === 'PEAK') {
    return PEAK_TP.getLibVersion()
  } else if (vendor === 'ZLG') {
    return ZLG_CAN.getLibVersion()
  } else if (vendor === 'KVASER') {
    return KVASER_CAN.getLibVersion()
  } else if (vendor === 'SIMULATE') {
    return SIMULATE_CAN.getLibVersion()
  } else if (vendor === 'TOOMOSS') {
    return TOOMOSS_CAN.getLibVersion()
  } else if (vendor === 'VECTOR') {
    return VECTOR_CAN.getLibVersion()
  } else if (vendor === 'SLCAN') {
    return SLCAN_CAN.getLibVersion()
  } else if (vendor === 'CANDLE') {
    return Candle_CAN.getLibVersion()
  } else {
    return 'Not supported'
  }
}

export function getCanDevices(vendor: string) {
  vendor = vendor.toUpperCase()
  if (vendor === 'PEAK') {
    return PEAK_TP.getValidDevices()
  } else if (vendor === 'ZLG') {
    return ZLG_CAN.getValidDevices()
  } else if (vendor === 'KVASER') {
    return KVASER_CAN.getValidDevices()
  } else if (vendor === 'SIMULATE') {
    return SIMULATE_CAN.getValidDevices()
  } else if (vendor === 'TOOMOSS') {
    return TOOMOSS_CAN.getValidDevices()
  } else if (vendor === 'VECTOR') {
    return VECTOR_CAN.getValidDevices()
  } else if (vendor === 'SLCAN') {
    return SLCAN_CAN.getValidDevices()
  } else if (vendor === 'CANDLE') {
    return Candle_CAN.getValidDevices()
  } else {
    return []
  }
}

export function canClean() {
  KVASER_CAN.unloadDll()
}
