import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

const rendererRoot = resolve(__dirname, 'src/renderer')
const logShim = resolve(__dirname, 'src/renderer/src/electron-log-shim.ts')

export default defineConfig({
  root: rendererRoot,
  base: './',
  resolve: {
    alias: {
      src: resolve(__dirname, 'src'),
      '@r': resolve(__dirname, 'src/renderer/src'),
      nodeCan: resolve(__dirname, 'src/main/share'),
      // 先匹配子路径，避免 'electron-log/renderer' 被前缀吞成 'shim.ts/renderer'
      'electron-log/renderer': logShim,
      'electron-log': logShim,
      'electron-updater': resolve(__dirname, 'src/renderer/src/electron-updater-shim.ts')
    }
  },
  plugins: [
    vue(),
    vueJsx(),
    nodePolyfills({
      include: ['buffer'],
      globals: { Buffer: true }
    })
  ],
  build: {
    outDir: resolve(__dirname, 'dist'),
    emptyOutDir: true,
    target: 'chrome120',
    chunkSizeWarningLimit: 4096
  },
  server: {
    port: 5173,
    strictPort: true
  }
})