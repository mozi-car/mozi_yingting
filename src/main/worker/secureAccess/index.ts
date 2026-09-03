// Lazy-load sa.node so importing this module does not crash when the native
// binary is absent (e.g. published main-plugin-sdk has no sa.node).
import { loadNative } from '../../native'

let saNode: any

function getStubSaNode() {
  return {
    SeedKey: class {
      IsLoaded() { return false }
      LoadDLL() { throw new Error('SecureAccessDll is only available on Windows platform') }
      GenerateKeyExOpt() { throw new Error('SecureAccessDll is only available on Windows platform') }
      GenerateKeyEx() { throw new Error('SecureAccessDll is only available on Windows platform') }
    }
  }
}

function getSaNode() {
  if (saNode) {
    return saNode
  }
  if (process.platform == 'win32') {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    saNode = loadNative('sa')
  } else {
    saNode = getStubSaNode()
  }
  return saNode
}

/**
 * @category UDS
 */
export class SecureAccessDll {
  _ref: any
  constructor(dllPath: string) {
    const native = getSaNode()
    this._ref = new native.SeedKey()
    // console.log(dllPath)
    this.loadDll(dllPath)
    if (!this._ref.IsLoaded()) {
      throw new Error('Failed to load DLL')
    }
  }

  /**
   * Generates a key with extended options.
   *
   * @param ipSeedArray - A buffer containing the seed array, for c: = ipSeedArray + iSeedArraySize
   * @param iSecurityLevel - The security level to be used.
   * @param ipVariant - A buffer containing the variant. for c: = ipVariant, size decide by vendor self
   * @param ipOptions - A buffer containing the options. for c: = ipOptions, size decide by vendor self
   * @param key - A buffer containing the input key.for c: = iopKeyArray + iMaxKeyArraySize
   * @returns A buffer containing the generated key. Return is Buffer, for c: = iopKeyArray, length = oActualKeyArraySize
   * @throws Will throw an error if the key generation fails.
   *
   * @example
   * ```typescript
   *
   * const dllPath=path.join(__dirname,'GenerateKeyExOpt.dll')
   * const sa=new SecureAccessDll(dllPath)
   *
   * const seed=sa.GenerateKeyExOpt(Buffer.from([1,2,3,4,5]),1,Buffer.from([1,2,3,4,5]),Buffer.from([1,2,3,4,5]),Buffer.from([1,2,3,4,5]))
   * ```
   *
   */
  GenerateKeyExOpt(
    ipSeedArray: Buffer,
    iSecurityLevel: number,
    ipVariant: Buffer,
    ipOptions: Buffer,
    key: Buffer
  ): Buffer {
    const native = getSaNode()
    return this._ref.GenerateKeyExOpt(ipSeedArray, iSecurityLevel, ipVariant, ipOptions, key)
  }
  /**
   * Generates a key with extended options.
   *
   * @param ipSeedArray - A buffer containing the seed array, for c: = ipSeedArray + iSeedArraySize
   * @param iSecurityLevel - The security level to be used.
   * @param ipVariant - A buffer containing the variant. for c: = ipVariant, size decide by vendor self
   * @param key - A buffer containing the input key.for c: = ioKeyArray + iKeyArraySize
   * @returns A buffer containing the generated key. Return is Buffer, for c: = ioKeyArray, length = oSize
   * @throws Will throw an error if the key generation fails.
   *
   * @example
   * ```typescript
   *
   * const dllPath=path.join(__dirname,'GenerateKeyEx.dll')
   * const sa=new SecureAccessDll(dllPath)
   *
   *const seed=sa.GenerateKeyEx(Buffer.from([1,2,3,4,5]),1,Buffer.from([1,2,3,4,5]),Buffer.from([1,2,3,4,5]))
   * ```
   *
   */
  GenerateKeyEx(
    ipSeedArray: Buffer,
    iSecurityLevel: number,
    ipVariant: Buffer,
    key: Buffer
  ): Buffer {
    const native = getSaNode()
    return this._ref.GenerateKeyEx(ipSeedArray, iSecurityLevel, ipVariant, key)
  }
  private loadDll(dllPath: string) {
    this._ref.LoadDLL(dllPath)
  }
}
