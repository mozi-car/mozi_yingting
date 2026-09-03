/** Build Rust Native-API modules used by the Node sidecar. Zero-C++ runtime build. */
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

const modules = [
  { name: 'sa', directory: 'native/secure-access', packageName: 'secure_access' },
  { name: 'candle', directory: 'native/candle', packageName: 'candle' },
  { name: 'peak', directory: 'native/peak', packageName: 'peak' },
  { name: 'kvaser', directory: 'native/kvaser', packageName: 'kvaser' },
  { name: 'zlg', directory: 'native/zlg', packageName: 'zlg' },
  { name: 'vector', directory: 'native/vector', packageName: 'vector' },
  { name: 'kvaserLin', directory: 'native/kvaserLin', packageName: 'kvaser_lin' },
  { name: 'peakLin', directory: 'native/peakLin', packageName: 'peak_lin' },
  { name: 'toomossLin', directory: 'native/toomossLin', packageName: 'toomoss_lin' },
  { name: 'toomoss', directory: 'native/toomoss', packageName: 'toomoss' },
  { name: 'vsomeip', directory: 'native/vsomeip', packageName: 'vsomeip' },
  { name: 'serial', directory: 'native/serial', packageName: 'serial' }
]
const target = 'x86_64-pc-windows-gnu'
const nodeLib = path.join(process.env.TEMP ?? process.env.TMP ?? root, 'yingting-rust-node-lib')
const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo'

function ensureNodeImportLibrary() {
  if (process.platform !== 'win32' || fs.existsSync(path.join(nodeLib, 'libnode.a'))) return
  fs.mkdirSync(nodeLib, { recursive: true })
  const nodeExe = process.execPath
  const nodeDll = path.join(nodeLib, 'libnode.dll')
  const nodeDef = path.join(nodeLib, 'libnode.def')
  fs.copyFileSync(nodeExe, nodeDll)
  try {
    execFileSync('gendef.exe', [nodeDll], { cwd: nodeLib, stdio: 'inherit' })
    execFileSync('dlltool.exe', ['-d', nodeDef, '-l', path.join(nodeLib, 'libnode.a'), '-D', 'libnode.dll'], {
      cwd: nodeLib,
      stdio: 'inherit'
    })
  } catch (error) {
    throw new Error(`[native] unable to create libnode import library; Rust native build is required: ${error}`)
  }
}

ensureNodeImportLibrary()
if (process.platform === 'win32' && !fs.existsSync(path.join(nodeLib, 'libnode.a'))) {
  throw new Error('[native] libnode.a is missing; refusing to skip Rust native addon compilation')
}
const output = path.join(root, 'out/sidecar/native')
fs.mkdirSync(output, { recursive: true })

for (const module of modules) {
  const moduleRoot = path.join(root, module.directory)
  console.log(`[native] building Rust ${module.name} addon`)
  execFileSync(cargo, ['build', '--manifest-path', path.join(moduleRoot, 'Cargo.toml'), '--target', target, '--release'], {
    cwd: root,
    env: {
      ...process.env,
      LIBNODE_PATH: nodeLib,
      RUSTFLAGS: `${process.env.RUSTFLAGS ?? ''} -L native=${nodeLib}`.trim()
    },
    stdio: 'inherit'
  })
  const built = path.join(moduleRoot, `target/${target}/release/${module.packageName}.dll`)
  const destination = path.join(output, `${module.name}.node`)
  if (!fs.existsSync(built)) throw new Error(`[native] Rust addon was not produced: ${built}`)
  fs.copyFileSync(built, destination)
  console.log(`[native] Rust addon copied -> ${destination}`)
}
