import EventEmitter from 'events'
import { loadNative } from '../native'

type NativeSerial = { new (): { open(path: string, baud: number, dataBits?: number, stopBits?: number, parity?: string, callback?: (data: Buffer) => void): void; write(data: Buffer): number; drain(): void; flush(): void; close(): void; isOpen(): boolean }; list(): string[] }
const api = loadNative<NativeSerial>('serial')

export type SerialPortOpenOptions = { path: string; baudRate: number; dataBits?: number; stopBits?: number; parity?: string; autoOpen?: boolean }
export class SerialPort extends EventEmitter {
  private native = new api()
  isOpen = false
  constructor(private options: SerialPortOpenOptions, callback?: (error?: Error) => void) {
    super()
    if (options.autoOpen !== false) this.open(callback)
  }
  open(callback?: (error?: Error) => void) { try { this.native.open(this.options.path, this.options.baudRate, this.options.dataBits, this.options.stopBits, this.options.parity, (data: Buffer) => this.emit('data', Buffer.from(data))); this.isOpen = true; callback?.(); this.emit('open') } catch (e) { const error = e instanceof Error ? e : new Error(String(e)); callback?.(error); this.emit('error', error) } }
  write(data: Buffer | string, callback?: (error?: Error) => void) { try { this.native.write(Buffer.isBuffer(data) ? data : Buffer.from(data)); callback?.(); return true } catch (e) { const error = e instanceof Error ? e : new Error(String(e)); callback?.(error); this.emit('error', error); return false } }
  drain(callback?: (error?: Error) => void) { try { this.native.drain(); callback?.() } catch (e) { callback?.(e as Error) } }
  flush(callback?: (error?: Error) => void) { try { this.native.flush(); callback?.() } catch (e) { callback?.(e as Error) } }
  close(callback?: (error?: Error) => void) { try { this.native.close(); this.isOpen = false; callback?.(); this.emit('close') } catch (e) { callback?.(e as Error) } }
  static list() { return Promise.resolve(api.list().map(path => ({ path, friendlyName: path, serialNumber: undefined }))) }
}
