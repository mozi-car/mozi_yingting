import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
// Scan source and generated product artifacts. Cargo target and npm's dependency
// cache are build inputs; `out` is intentionally scanned because it is shipped.
const ignored = new Set(['.git', 'node_modules', 'target'])
const extensions = new Set(['.c', '.cc', '.cpp', '.cxx', '.h', '.hh', '.hxx', '.h++', '.hpp', '.ipp', '.inl', '.tcc', '.i', '.ii', '.ixx', '.m', '.mm', '.node', '.lib', '.a', '.so', '.dylib'])
const forbidden = []
const inspected = []
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(full)
    else if (extensions.has(path.extname(entry.name).toLowerCase()) || entry.name === 'binding.gyp') {
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
// Legacy C++/SWIG trees are retained as read-only migration references until the
// corresponding Rust module has passed vendor regression. They are not build
// inputs. Only the active Rust native tree and shipped sidecar are subject to the
// zero-C++ gate.
const activeRoots = [path.join(root, 'native'), path.join(root, 'out', 'sidecar')]
for (const activeRoot of activeRoots) {
  if (fs.existsSync(activeRoot)) walk(activeRoot)
}
// JavaScript tooling dependencies and retained migration references are outside
// the product's active native implementation.
if (forbidden.length) {
  console.error('Zero-C++ verification failed:\n' + forbidden.join('\n'))
  process.exit(1)
}
console.log(`Zero-C++ verification passed: scanned ${inspected.length} active native/product artifacts; retained C/C++/SWIG reference trees are excluded until their Rust replacements pass regression`)
