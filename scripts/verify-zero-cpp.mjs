import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
// Scan source and generated product artifacts. Cargo target is a build cache,
// but `out` is shipped and must include no C/C++ dependency tree. Replaced
// modules have no migration archive; do not ignore out/sidecar/node_modules:
// stale bindings-cpp there would still be delivered to users.
const ignored = new Set(['.git', 'target'])
const extensions = new Set(['.c', '.cc', '.cpp', '.cxx', '.h', '.hh', '.hxx', '.h++', '.hpp', '.ipp', '.inl', '.tcc', '.i', '.ii', '.ixx', '.m', '.mm', '.node', '.lib', '.a', '.so', '.dylib', '.gyp', '.gypi'])
const forbidden = []
const inspected = []
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(full)
    else if (extensions.has(path.extname(entry.name).toLowerCase()) || ['binding.gyp', 'CMakeLists.txt', 'Makefile'].includes(entry.name) || entry.name.endsWith('.bat')) {
      inspected.push(path.relative(root, full))
      const extension = path.extname(entry.name).toLowerCase()
      if (extension === '.node') {
        // Rust N-API release artifacts are allowed; reject known C++ addon markers.
        const bytes = fs.readFileSync(full).toString('latin1')
        if (/bindings-cpp|node-gyp|node-addon-api|serialport-bindings|c\+\+ exception/i.test(bytes)) {
          forbidden.push(path.relative(root, full))
        }
      } else if (['.lib', '.a', '.so', '.dylib'].includes(extension)) {
        forbidden.push(path.relative(root, full))
      } else {
        forbidden.push(path.relative(root, full))
      }
    }
  }
}
// The TypeScript driver classes under docan/dolin are still the public
// orchestration layer, but every native implementation must come from native/.
// Scan those source roots too so a removed Rust migration cannot silently be
// replaced by an old binding.gyp/SWIG artifact.
const activeRoots = [
  path.join(root, 'native'),
  // All application source is active. This catches legacy vsomeip and
  // SecureAccess bridges that do not live under docan/dolin.
  path.join(root, 'src'),
  path.join(root, 'out'),
  path.join(root, 'resources', 'lib', 'native'),
  path.join(root, 'resources', 'lib', 'js')
]
for (const activeRoot of activeRoots) {
  if (fs.existsSync(activeRoot)) walk(activeRoot)
}
if (forbidden.length) {
  console.error('Zero-C++ verification failed:\n' + forbidden.join('\n'))
  process.exit(1)
}
console.log(`Zero-C++ verification passed: scanned ${inspected.length} active source/product artifacts; all active native driver implementations are Rust`)
