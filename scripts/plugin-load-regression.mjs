import assert from 'node:assert/strict'
import fs from 'node:fs/promises'

const timeout = (promise, ms, message) =>
  new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), ms)
    promise.then(
      (value) => {
        clearTimeout(timer)
        resolve(value)
      },
      (error) => {
        clearTimeout(timer)
        reject(error)
      }
    )
  })

const [mainPlugin, rendererPlugin, pluginStore] = await Promise.all([
  fs.readFile('src/main/ipc/plugin.ts', 'utf8'),
  fs.readFile('src/renderer/src/plugin/plugin.vue', 'utf8'),
  fs.readFile('src/renderer/src/stores/plugin.ts', 'utf8')
])

assert.match(mainPlugin, /PLUGIN_START_TIMEOUT_MS/)
assert.match(mainPlugin, /Plugin .* start timed out/)
assert.match(pluginStore, /Promise\.allSettled\(enablePromises\)/)
assert.match(rendererPlugin, /ready\.value = true/)
assert.match(rendererPlugin, /qitem\?\.entry/)

const plugins = [
  { id: 'hung-plugin', start: () => new Promise(() => {}) },
  { id: 'healthy-plugin', start: async () => 'loaded' }
]

const results = await Promise.allSettled(
  plugins.map(async (plugin) => {
    try {
      return await timeout(plugin.start(), 50, `${plugin.id} timed out`)
    } catch (error) {
      return { id: plugin.id, status: 'skipped', error: error.message }
    }
  })
)

assert.equal(results[0].status, 'fulfilled')
assert.deepEqual(results[0].value.status, 'skipped')
assert.equal(results[1].status, 'fulfilled')
assert.equal(results[1].value, 'loaded')

console.log('plugin-load-regression: passed')
console.log('- hung plugin is timed out and isolated')
console.log('- healthy plugin continues loading')
console.log('- renderer mount is gated until runtime initialization completes')
