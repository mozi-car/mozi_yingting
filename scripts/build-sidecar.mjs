/**
 * 婴听 sidecar 构建脚本 (esbuild)
 * - 处理 electron-vite 的 ?asset 语法（资源路径字符串化）
 * - alias electron / @electron-toolkit/utils 到 shim
 */
import esbuild from 'esbuild'
import path from 'path'
import fs from 'fs'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')
const nativeFiles = new Set()

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
        contents: `export default require('process').env.YT_RESOURCES
  ? require('path').join(require('process').env.YT_RESOURCES, ${JSON.stringify(rel)})
  : require('path').join(require('path').resolve(__dirname, '..', '..'), ${JSON.stringify(rel)})`,
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
    build.onResolve({ filter: /\.node$/ }, (args) => {
      throw new Error(`Legacy .node import forbidden; migrate ${args.path} to Rust native loader`)
    })
    build.onLoad({ filter: /.*/, namespace: 'yt-native' }, (args) => {
      const abs = args.path
      const filename = path.basename(abs)
      return {
        contents: `
let mod;
const nativePath = require('path').join(__dirname, 'native', ${JSON.stringify(filename)});
try {
  const nativeRequire = require('module').createRequire(__filename);
  mod = nativeRequire(nativePath);
} catch (e) {
  mod = new Proxy({}, {
    get: () => () => {
      throw new Error('native addon unavailable: ' + nativePath + ' -> ' + (e && e.message))
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
  entryPoints: [path.join(root, 'src/main/index.ts')],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node22',
  outfile: path.join(root, 'out/sidecar/index.cjs'),
  sourcemap: true,
  // 全量打包依赖（release 无 node_modules），仅外置原生/平台相关
  external: ['electron-updater'],
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

const nativeDst = path.join(root, 'out/sidecar/native')
fs.mkdirSync(nativeDst, { recursive: true })
for (const nativeFile of nativeFiles) {
  if (!fs.existsSync(nativeFile)) {
    console.warn(`[sidecar] native addon missing, skipped: ${nativeFile}`)
    continue
  }
  const destination = path.join(nativeDst, path.basename(nativeFile))
  if (path.resolve(nativeFile) !== path.resolve(destination)) fs.copyFileSync(nativeFile, destination)
}

// No legacy native serial closure is copied into the sidecar.
for (const dep of ['ms', 'debug']) {
  fs.cpSync(path.join(root, 'node_modules', dep), path.join(root, 'out/sidecar/node_modules', dep), {
    recursive: true
  })
}

console.log('[sidecar] built -> out/sidecar/index.cjs')
