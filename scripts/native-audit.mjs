import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import driverManifest from './native-driver-manifest.json' with { type: 'json' }

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const errors = []
const checked = []
const exists = (relative) => fs.existsSync(path.join(root, relative))
const addError = (message) => errors.push(message)
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8')
const sha256 = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex')

// Every production native entrypoint must be a Rust crate and must be wired
// through the shared loadNative() loader. This is the call-chain check that a
// directory move alone cannot satisfy.
const nativeCallSites = new Set()
for (const [name, entry] of Object.entries(driverManifest)) {
  const crateToml = path.join(root, entry.crate, 'Cargo.toml')
  if (!fs.existsSync(crateToml)) addError(`${name}: missing Rust crate ${entry.crate}/Cargo.toml`)
  let hasLoaderCall = false
  for (const source of entry.typescript) {
    const file = path.join(root, source)
    if (!fs.existsSync(file)) {
      addError(`${name}: missing TypeScript call site ${source}`)
      continue
    }
    const sourceText = fs.readFileSync(file, 'utf8')
    const matcher = new RegExp(`loadNative(?:<[^>]+>)?\\(\\s*['"]${name}['"]\\s*\\)`)
    if (matcher.test(sourceText)) {
      hasLoaderCall = true
      nativeCallSites.add(source)
    }
  }
  if (!hasLoaderCall) addError(`${name}: no TypeScript entrypoint calls loadNative('${name}')`)
}

// The sidecar and CLI may only consume the canonical Rust artifacts. A stale
// resources/lib/js/sa.node or a second addon directory would make the result
// dependent on the old C++ build layout.
const addonNames = Object.keys(driverManifest).map((name) => `${name}.node`).sort()
const canonicalDirs = ['out/sidecar/native', 'out/cli/native', 'resources/lib/native']
for (const relative of canonicalDirs) {
  const dir = path.join(root, relative)
  if (!fs.existsSync(dir)) continue
  const files = fs.readdirSync(dir).filter((file) => file.endsWith('.node')).sort()
  if (relative === 'out/sidecar/native' && files.join('|') !== addonNames.join('|')) {
    addError(`${relative}: expected ${addonNames.join(', ')}, found ${files.join(', ')}`)
  }
  for (const file of files) checked.push(`${relative}/${file}`)
}
for (const stale of ['resources/lib/js/sa.node', 'resources/lib/js/candle.node', 'resources/lib/js/peak.node']) {
  if (exists(stale)) addError(`legacy addon still present: ${stale}`)
}

const sidecarDir = path.join(root, 'out/sidecar/native')
if (exists('out/sidecar/index.cjs') && fs.existsSync(path.join(root, 'src/main/vsomeip/worker.ts')) && !exists('out/sidecar/vsomeip.js')) {
  addError('vsomeip: Rust-backed worker artifact missing: out/sidecar/vsomeip.js')
}
const vsomeipWorker = exists('src/main/vsomeip/worker.ts') ? read('src/main/vsomeip/worker.ts') : ''
for (const legacyMethod of ['notify_event', 'request_event_one_group', 'release_event_simple', 'offer_event_with_groups', 'stop_offer_event']) {
  if (vsomeipWorker.includes(`.${legacyMethod}`)) addError(`vsomeip worker still calls legacy SWIG method: ${legacyMethod}`)
}
for (const file of addonNames) {
  const sidecarFile = path.join(sidecarDir, file)
  if (!fs.existsSync(sidecarFile)) continue
  const sidecarHash = sha256(sidecarFile)
  for (const relative of ['out/cli/native', 'resources/lib/native']) {
    const copy = path.join(root, relative, file)
    if (fs.existsSync(copy) && sha256(copy) !== sidecarHash) {
      addError(`${relative}/${file}: differs from out/sidecar/native/${file}`)
    }
  }
}

// No active source may contain the old C/C++ build inputs. Replaced modules
// deliberately have no migration archive: the Rust crate is the sole source.
const sourceRoots = ['native', 'src', 'scripts', 'out/sidecar']
const forbiddenNames = new Set(['binding.gyp', 'CMakeLists.txt', 'Makefile'])
const forbiddenExtensions = new Set(['.c', '.cc', '.cpp', '.cxx', '.h', '.hh', '.hxx', '.hpp', '.ipp', '.inl', '.tcc', '.i', '.ii', '.ixx', '.gyp', '.gypi'])
function scan(dir, relative = dir) {
  if (!fs.existsSync(dir)) return
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (['target', 'node_modules', '.git', 'migration-archive'].includes(entry.name)) continue
    const full = path.join(dir, entry.name)
    const rel = path.relative(root, full)
    if (entry.isDirectory()) scan(full, rel)
    else if (forbiddenNames.has(entry.name) || forbiddenExtensions.has(path.extname(entry.name).toLowerCase())) addError(`legacy build input in active tree: ${rel}`)
    else if (entry.name.endsWith('.node')) {
      const normalized = rel.split(path.sep).join('/')
      if (!normalized.startsWith('out/sidecar/native/') && !normalized.startsWith('out/cli/native/') && !normalized.startsWith('resources/lib/native/')) addError(`non-canonical native addon: ${rel}`)
    }
  }
}
for (const sourceRoot of sourceRoots) scan(path.join(root, sourceRoot))

// Vendor DLL paths are release inputs, not C++ source inputs. Check all
// concrete paths in the manifest and the retained import-library directory.
for (const [name, entry] of Object.entries(driverManifest)) {
  for (const vendor of entry.vendor ?? []) {
    if (vendor.startsWith('resources/') && !exists(vendor)) addError(`${name}: vendor runtime missing: ${vendor}`)
  }
}
const importLibDir = path.join(root, 'resources/lib/vendor-import-libs')
if (!fs.existsSync(importLibDir) || !fs.readdirSync(importLibDir).some((file) => file.endsWith('.lib'))) {
  addError('vendor import-library compatibility directory is empty: resources/lib/vendor-import-libs')
}

if (errors.length) {
  console.error('Native Rust call-chain audit failed:\n' + errors.map((error) => `- ${error}`).join('\n'))
  process.exit(1)
}
console.log(`Native Rust call-chain audit passed: ${Object.keys(driverManifest).length} modules, ${checked.length} canonical artifacts, ${nativeCallSites.size} TypeScript entrypoints, vendor runtimes retained`)
