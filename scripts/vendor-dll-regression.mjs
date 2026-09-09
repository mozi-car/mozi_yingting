import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const resources = path.join(root, 'resources', 'lib')
const vendor = {
  YT_VENDOR_DLL_PEAK: path.join(resources, 'PCAN-ISO-TP.dll'),
  YT_VENDOR_DLL_KVASER: path.join(resources, 'canlib32.dll'),
  YT_VENDOR_DLL_ZLG: path.join(resources, 'zlgcan.dll'),
  YT_VENDOR_DLL_VECTOR: path.join(resources, 'vxlapi64.dll'),
  YT_VENDOR_DLL_TOOMOSS: path.join(resources, 'USB2XXX.dll'),
  YT_VENDOR_DLL_KVASERLIN: path.join(resources, 'linlib.dll'),
  YT_VENDOR_DLL_PEAKLIN: path.join(resources, 'PLinApi.dll'),
  YT_VENDOR_DLL_TOOMOSSLIN: path.join(resources, 'USB2XXX.dll')
}
const missing = Object.entries(vendor).filter(([, file]) => !fs.existsSync(file))
if (missing.length) throw new Error(`vendor DLL regression inputs missing: ${missing.map(([name, file]) => `${name}=${file}`).join(', ')}`)

const env = { ...process.env, ...vendor }
const run = (script) => {
  const result = spawnSync(process.execPath, [path.join(root, 'scripts', script)], {
    cwd: root,
    env,
    stdio: 'inherit'
  })
  if (result.status !== 0) process.exit(result.status ?? 1)
}

// native-regression checks every Rust CAN/LIN LoadDll/IsLoaded path and also
// exercises missing-DLL/error semantics. SecureAccess is checked separately
// because its vendor ABI is a SeedKey class rather than a module-level loader.
run('native-regression.mjs')
run('secure-access-regression.mjs')
console.log(`Vendor DLL load regression passed for ${Object.keys(vendor).length} CAN/LIN runtime inputs plus SecureAccess`)
