import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const dll = process.env.YT_SECURE_ACCESS_DLL || path.join(root, 'resources', 'examples', 'secure_access_dll', 'dll', 'GenerateKeyEx.dll')
if (!fs.existsSync(dll)) throw new Error(`Secure Access test DLL missing: ${dll}`)
const require = createRequire(import.meta.url)
const api = require(path.join(root, 'out', 'sidecar', 'native', 'sa.node'))
const seedKey = new api.SeedKey()
seedKey.LoadDLL(dll)
if (!seedKey.IsLoaded()) throw new Error('Secure Access DLL did not load')
const key = seedKey.GenerateKeyEx(
  Buffer.from([1, 2, 3, 4, 5]),
  1,
  Buffer.from([1, 2, 3]),
  Buffer.alloc(64)
)
const actual = key.toString('hex')
const expected = 'fefdfcfbfa'
if (actual !== expected) throw new Error(`Secure Access output mismatch: expected ${expected}, got ${actual}`)
seedKey.Unload()
if (seedKey.IsLoaded()) throw new Error('Secure Access Unload left the DLL loaded')
seedKey.LoadDLL(dll)
if (!seedKey.IsLoaded()) throw new Error('Secure Access reload failed')
console.log(`Secure Access DLL regression passed: GenerateKeyEx -> ${actual}; unload/reload passed`)
