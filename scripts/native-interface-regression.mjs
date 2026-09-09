import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import manifest from './native-interface-manifest.json' with { type: 'json' }

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const nativeDir = path.join(root, 'out', 'sidecar', 'native')
const require = createRequire(import.meta.url)
let checked = 0

function resolveInterface(addon, name) {
  const prototypeParts = name.split('.prototype.')
  if (prototypeParts.length === 2) {
    const owner = addon[prototypeParts[0]]
    if (typeof owner !== 'function') throw new Error(`${prototypeParts[0]} is not a constructor`)
    const descriptor = Object.getOwnPropertyDescriptor(owner.prototype, prototypeParts[1])
    if (!descriptor) throw new Error(`${name} is not present on prototype`)
    return { descriptor, name }
  }
  const staticParts = name.split('.')
  if (staticParts.length === 2) {
    const owner = addon[staticParts[0]]
    if (typeof owner !== 'function') throw new Error(`${staticParts[0]} is not a constructor`)
    const descriptor = Object.getOwnPropertyDescriptor(owner, staticParts[1])
    if (!descriptor) throw new Error(`${name} is not present as a static member`)
    return { descriptor, name }
  }
  const descriptor = Object.getOwnPropertyDescriptor(addon, name)
  if (!descriptor) throw new Error(`${name} is not an addon export`)
  return { descriptor, name }
}

for (const [file, interfaces] of Object.entries(manifest)) {
  const addonPath = path.join(nativeDir, file)
  if (!fs.existsSync(addonPath)) throw new Error(`${file}: built addon missing`)
  const addon = require(addonPath)
  for (const name of interfaces) {
    const { descriptor } = resolveInterface(addon, name)
    if (typeof descriptor.value === 'undefined' && !descriptor.get && !descriptor.set) {
      throw new Error(`${file}: interface ${name} is undefined`)
    }
    checked++
  }
  console.log(`${file}: interface boundary verified (${interfaces.length})`)
}
console.log(`Native complete interface regression passed: ${checked} interfaces across ${Object.keys(manifest).length} addons`)
