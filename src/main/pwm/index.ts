import { PwmBaseInfo, PwmDevice } from '../share/uds'
import PwmBase from './base'
import { yingtingPwm } from './ecubus'

export { PwmBase, yingtingPwm }
export type { PwmBaseInfo, PwmDevice }

/**
 * Create a PWM device based on the provided info
 */
export function createPwmDevice(info: PwmBaseInfo): PwmBase {
  switch (info.vendor) {
    case 'ecubus':
      return new yingtingPwm(info)
    default:
      throw new Error(`Unsupported PWM vendor: ${info.vendor}`)
  }
}

/**
 * Get all available PWM devices
 */
export async function getValidPwmDevices(vendor: string): Promise<PwmDevice[]> {
  vendor = vendor.toUpperCase()
  if (vendor === 'ECUBUS') {
    return yingtingPwm.getValidDevices()
  } else {
    return []
  }
}
