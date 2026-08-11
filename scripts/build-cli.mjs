/**
 * 婴听 CLI 构建脚本 (esbuild)
 * - 与 build-sidecar 同构：alias electron -> shim，全量打包依赖
 * - 输出 out/cli/myt.cjs（与 EcuBus-Pro cli/out/ecb_cli.js 对齐）
 */
import esbuild from 'esbuild'
import path from 'path'
import fs from 'fs'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')

const assetPlugin = {
  name: 'yt-asset',
  setup(build) {
    build.onResolve({ filter: /[?&]asset|modulePath/ }, (args) => {
      const cleanPath = args.path.replace(/[?&](asset(?:&asarUnpack)?|modulePath)$/, '')
      const absPath = path.resolve(args.resolveDir, cleanPath)
      return { path: absPath, namespace: 'yt-asset' }
    })
    build.onLoad({ filter: /.*/, namespace: 'yt-asset' }, (args) => {
      const abs = args.path
      const rel = path.relative(root, abs)
      return {
        contents: `export default require('path').join(require('process').env.YT_RESOURCES || __dirname.replace(/out[\\\\/](sidecar|cli)$/, ''), ${JSON.stringify(rel)})`,
        loader: 'js'
      }
    })
  }
}

const rawPlugin = {
  name: 'yt-raw',
  setup(build) {
    build.onResolve({ filter: /\?raw$/ }, (args) => {
      const cleanPath = args.path.replace(/\?raw$/, '')
      const absPath = path.resolve(args.resolveDir, cleanPath)
      return { path: absPath, namespace: 'yt-raw' }
    })
    build.onLoad({ filter: /.*/, namespace: 'yt-raw' }, (args) => {
      try {
        const content = fs.readFileSync(args.path, 'utf8')
        return { contents: `export default ${JSON.stringify(content)}`, loader: 'js' }
      } catch {
        return { contents: 'export default ""', loader: 'js' }
      }
    })
  }
}

const nativePlugin = {
  name: 'yt-native',
  setup(build) {
    build.onResolve({ filter: /\.node$/ }, (args) => ({
      path: args.path,
      namespace: 'yt-native'
    }))
    build.onLoad({ filter: /.*/, namespace: 'yt-native' }, (args) => {
      const abs = args.path
      return {
        contents: `
let mod;
try {
  mod = require(${JSON.stringify(abs)});
} catch (e) {
  mod = new Proxy({}, {
    get: () => () => {
      throw new Error('native addon unavailable: ' + ${JSON.stringify(abs)} + ' -> ' + (e && e.message))
    }
  });
}
module.exports = mod;
`,
        loader: 'js'
      }
    })
  }
}

await esbuild.build({
  entryPoints: [path.join(root, 'src/cli/index.ts')],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node22',
  outfile: path.join(root, 'out/cli/myt.cjs'),
  sourcemap: true,
  banner: { js: '#!/usr/bin/env node' },
  external: ['electron-updater', '@serialport/bindings-cpp'],
  alias: {
    electron: path.join(root, 'src/main/electron-shim.ts'),
    '@electron-toolkit/utils': path.join(root, 'src/main/toolkit-shim.ts'),
    'electron-log': path.join(root, 'src/main/electron-log-shim.ts'),
    'electron-log/main': path.join(root, 'src/main/electron-log-shim.ts'),
    src: path.join(root, 'src')
  },
  plugins: [assetPlugin, rawPlugin, nativePlugin],
  define: {
    'import.meta.env': '{}'
  },
  logLevel: 'warning'
})

// serialport 原生 addon 无法 bundle，随包携带 node_modules 供运行时 require
const nmDst = path.join(root, 'out/cli/node_modules')
fs.rmSync(nmDst, { recursive: true, force: true })
fs.cpSync(path.join(root, 'node_modules/@serialport'), path.join(nmDst, '@serialport'), {
  recursive: true
})
for (const dep of ['ms', 'debug', 'node-gyp-build']) {
  fs.cpSync(path.join(root, 'node_modules', dep), path.join(nmDst, dep), { recursive: true })
}

// 部署到 resources/lib/myt（随 resources 打包，sidecar pnpm 等调用）
const deployPath = path.join(root, 'resources/lib/myt')
fs.copyFileSync(path.join(root, 'out/cli/myt.cjs'), deployPath)
fs.chmodSync(deployPath, 0o755)

console.log('[cli] built -> out/cli/myt.cjs -> resources/lib/myt')
