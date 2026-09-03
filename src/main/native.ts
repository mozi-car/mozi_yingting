import path from 'node:path'
import { createRequire } from 'node:module'

/**
 * Loads a Rust N-API addon from the sidecar's single native directory.
 * Keeping this in one place prevents source-tree/build-layout imports from
 * leaking into runtime modules.
 */
export function loadNative<T = any>(name: string): T {
  const filename = name.endsWith('.node') ? name : `${name}.node`
  const nativeRoot = process.env.YT_NATIVE_DIR || path.join(__dirname, 'native')
  const nativePath = path.join(nativeRoot, filename)
  const requireNative = createRequire(__filename)
  try {
    return requireNative(nativePath) as T
  } catch (error: any) {
    throw new Error(`Unable to load Rust native addon ${filename} from ${nativePath}: ${error?.message ?? error}`)
  }
}
