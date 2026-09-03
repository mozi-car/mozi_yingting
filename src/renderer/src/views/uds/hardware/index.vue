<template>
  <div class="hw-main">
    <div class="hw-toolbar">
      <el-tabs v-model="activeType" class="hw-tabs">
        <el-tab-pane v-for="t in BUS_TYPES" :key="t" :label="TAB_LABELS[t]" :name="t" />
      </el-tabs>
      <el-dropdown trigger="click" @command="handleCanvasCommand">
        <el-button link class="hw-more" aria-label="Canvas actions">...</el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="zoomIn">
              <Icon :icon="zoomInRounded" />
              {{ i18next.t('uds.hardware.canvas.zoomIn') }}
            </el-dropdown-item>
            <el-dropdown-item command="zoomOut">
              <Icon :icon="zoomOutRounded" />
              {{ i18next.t('uds.hardware.canvas.zoomOut') }}
            </el-dropdown-item>
            <el-dropdown-item command="fit">
              <Icon :icon="fullscreenIcon" />
              {{ i18next.t('uds.hardware.canvas.reset') }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
    <div class="hw-body">
      <div class="hw-side">
        <div class="hw-side-title">{{ i18next.t('uds.hardware.side.title') }}</div>
        <el-scrollbar :height="canvasHeight() - 30 + 'px'">
          <el-tree
            ref="vendorTreeRef"
            :data="vendorTree"
            node-key="id"
            :expand-on-click-node="false"
            :default-expanded-keys="expandedKeys"
            @node-expand="rememberExpanded"
            @node-collapse="rememberCollapsed"
          >
            <template #default="{ node, data }">
              <div class="hw-side-node">
                <span class="hw-side-label" :title="node.label">
                  <Icon
                    v-if="data.kind === 'vendor'"
                    :icon="driverIcon"
                    class="hw-side-icon"
                  />
                  <Icon v-else :icon="chipIcon" class="hw-side-icon dim" />
                  <span :class="{ vendorName: data.kind === 'vendor' }">{{ node.label }}</span>
                  <span v-if="data.serial" class="hw-side-serial">#{{ data.serial }}</span>
                </span>
                <el-button
                  v-if="data.kind === 'vendor'"
                  link
                  type="primary"
                  :title="i18next.t('uds.hardware.side.addChannel')"
                  @click.stop="addChannelForVendor(data.vendor)"
                >
                  <Icon :icon="plusIcon" />
                </el-button>
              </div>
            </template>
          </el-tree>
          <el-empty
            v-if="vendorTree.length === 0"
            :description="i18next.t('uds.hardware.side.noVendor')"
            :image-size="60"
          />
        </el-scrollbar>
      </div>
      <div class="hw-canvas-area">
        <div
          v-for="t in BUS_TYPES"
          v-show="activeType === t"
          :key="t"
          :ref="setCanvasRef(t)"
          class="hw-canvas"
        />
      </div>
    </div>

    <channelDrawer
      v-model:visible="drawerVisible"
      :channel="editingChannel"
      :bus-type="activeType"
      :preset-vendor="presetVendor"
      @saved="onChannelSaved"
    />
  </div>
</template>
<script lang="ts" setup>
import { ref, inject, onMounted, onBeforeUnmount, watch, nextTick, h } from 'vue'
import * as joint from '@joint/core'
import { Icon } from '@iconify/vue'
import { ElMessageBox } from 'element-plus'
import i18next from 'i18next'
import { v4 } from 'uuid'
import { cloneDeep } from 'lodash'
import fullscreenIcon from '@iconify/icons-material-symbols/fullscreen'
import zoomInRounded from '@iconify/icons-material-symbols/zoom-in-rounded'
import zoomOutRounded from '@iconify/icons-material-symbols/zoom-out-rounded'
import driverIcon from '@iconify/icons-material-symbols/hard-drive-outline'
import chipIcon from '@iconify/icons-material-symbols/memory-alt-outline'
import plusIcon from '@iconify/icons-ep/circle-plus-filled'
import { useDataStore } from '@r/stores/data'
import { useProjectStore } from '@r/stores/project'
import { Layout } from '../layout'
import type { HardwareChannel } from 'src/preload/data'
import type { CanVendor } from 'nodeCan/can'
import { HardwareBoard, type ChannelVM, type ChildVM } from './hardwareGraph'
import {
  BUS_TYPES,
  BUS_TYPE_PREFIX,
  IA_SUPPORTED_TYPES,
  vendorSupportsType,
  getDeviceSummary,
  type BusType
} from './busTypes'
import channelDrawer from './channelDrawer.vue'
import nodeConfig from '../network/nodeConfig.vue'
import { ecubusPro } from '../../../../../../package.json'

const props = defineProps<{
  height: number
  width: number
  deviceId?: string
}>()

const winKey = 'hardware'
const dataBase = useDataStore()
const projectStore = useProjectStore()
const layout = inject('layout') as Layout

const TAB_LABELS: Record<BusType, string> = {
  can: 'CAN',
  lin: 'LIN',
  eth: 'ETH',
  pwm: 'PWM',
  serial: 'Serial'
}

const activeType = ref<BusType>('can')
const drawerVisible = ref(false)
const editingChannel = ref<HardwareChannel | null>(null)
const presetVendor = ref<CanVendor | undefined>()

interface VendorTreeNode {
  id: string
  label: string
  kind: 'vendor' | 'device'
  vendor: CanVendor
  serial?: string
  children?: VendorTreeNode[]
}
const vendorTree = ref<VendorTreeNode[]>([])
const vendorTreeRef = ref<any>()
const expandedByType = new Map<BusType, Set<string>>()
const expandedKeys = ref<string[]>([])
let hardwareRefreshTimer: ReturnType<typeof setInterval> | undefined
let hardwareLoadSerial = 0

function rememberExpanded(node: { data?: VendorTreeNode }) {
  const id = node.data?.id
  if (!id) return
  const keys = expandedByType.get(activeType.value) ?? new Set<string>()
  keys.add(id)
  expandedByType.set(activeType.value, keys)
  expandedKeys.value = [...keys]
}

function rememberCollapsed(node: { data?: VendorTreeNode }) {
  const id = node.data?.id
  if (!id) return
  const keys = expandedByType.get(activeType.value) ?? new Set<string>()
  keys.delete(id)
  expandedByType.set(activeType.value, keys)
  expandedKeys.value = [...keys]
}

const DEVICE_IPC: Record<BusType, string> = {
  can: 'ipc-get-can-devices',
  eth: 'ipc-get-eth-devices',
  lin: 'ipc-get-lin-devices',
  pwm: 'ipc-get-pwm-devices',
  serial: 'ipc-get-serial-devices'
}

/** Loads the always-visible vendor/driver list (and any detected hardware) for the active bus type. */
async function loadVendorTree(t: BusType) {
  const loadSerial = ++hardwareLoadSerial
  let vendors: CanVendor[] = []
  try {
    vendors = (await window.electron.ipcRenderer.invoke('ipc-get-vendor', ecubusPro)).map(
      (v: any) => v.name as CanVendor
    )
  } catch {
    vendors = []
  }
  // PC is the host-machine provider for generic Ethernet and serial ports.
  // Keep it separate from simulation; hardware-backed vendors are enumerated
  // by their own driver in the child nodes below.
  if ((t === 'eth' || t === 'serial') && !vendors.includes('pc')) vendors.push('pc')
  const supported = vendors.filter((v) => vendorSupportsType(v, t))
  const nodes: VendorTreeNode[] = []
  for (const vendor of supported) {
    const node: VendorTreeNode = {
      id: `${t}:${vendor}`,
      label: vendor.toUpperCase(),
      kind: 'vendor',
      vendor,
      children: []
    }
    try {
      const devices =
        t === 'serial'
          ? await window.electron.ipcRenderer.invoke(DEVICE_IPC[t])
          : await window.electron.ipcRenderer.invoke(DEVICE_IPC[t], vendor.toUpperCase())
      for (const d of devices || []) {
        node.children!.push({
          id: `${t}:${vendor}:${d.handle ?? d.id ?? d.label}`,
          label: d.label ?? d.name ?? String(d.handle),
          kind: 'device',
          vendor,
          serial: d.serialNumber
        })
      }
    } catch {
      // Enumeration can fail if the driver DLL/hardware is unavailable; still show the vendor.
    }
    nodes.push(node)
  }
  if (activeType.value === t && loadSerial === hardwareLoadSerial) {
    const keys = expandedByType.get(t)
    if (!keys) {
      const initial = new Set(nodes.filter((node) => node.kind === 'vendor').map((node) => node.id))
      expandedByType.set(t, initial)
      expandedKeys.value = [...initial]
    } else {
      // Preserve an explicit collapse while dropping keys for vendors no
      // longer present after a driver/device refresh.
      const available = new Set(nodes.map((node) => node.id))
      for (const key of [...keys]) if (!available.has(key)) keys.delete(key)
      expandedKeys.value = [...keys]
    }
    vendorTree.value = nodes
    await nextTick()
    vendorTreeRef.value?.setExpandedKeys?.(expandedKeys.value)
  }
}

function addChannelForVendor(vendor: CanVendor) {
  const t = activeType.value
  const id = v4()
  const count = Object.values(dataBase.channels).filter((c) => c.type === t).length
  dataBase.channels[id] = {
    id,
    type: t,
    name: `${BUS_TYPE_PREFIX[t]}${count}`
  }
  renderType(t)
  presetVendor.value = vendor
  editingChannel.value = dataBase.channels[id]
  drawerVisible.value = true
}

watch(activeType, () => {
  renderType(activeType.value)
  loadVendorTree(activeType.value)
})

watch(drawerVisible, (v) => {
  if (!v) presetVendor.value = undefined
})

function zoom(delta: number) {
  const paper = papers[activeType.value]
  if (!paper) return
  const s = Math.max(0.3, Math.min(2.5, paper.scale().sx + delta))
  paper.scale(s)
}
function fit() {
  const paper = papers[activeType.value]
  if (!paper) return
  paper.scale(1)
  paper.translate(0, 0)
}

function handleCanvasCommand(command: 'zoomIn' | 'zoomOut' | 'fit') {
  if (command === 'zoomIn') zoom(0.2)
  else if (command === 'zoomOut') zoom(-0.2)
  else fit()
}

// --- per bus-type canvas / graph setup ---
const canvasEls: Partial<Record<BusType, HTMLElement>> = {}
function setCanvasRef(t: BusType) {
  return (el: unknown) => {
    if (el) canvasEls[t] = el as HTMLElement
  }
}
const boards: Partial<Record<BusType, HardwareBoard>> = {}
const papers: Partial<Record<BusType, joint.dia.Paper>> = {}

function canvasHeight() {
  return Math.max(200, props.height - 88)
}

function canvasWidth() {
  return Math.max(200, props.width - 226)
}

function attachPan(paper: joint.dia.Paper) {
  let panning = false
  let start = { x: 0, y: 0 }
  paper.on('blank:pointerdown', (_evt: unknown, x: number, y: number) => {
    panning = true
    start = { x, y }
  })
  paper.on('blank:pointermove', (_evt: unknown, x: number, y: number) => {
    if (panning) {
      paper.translate(paper.translate().tx + (x - start.x), paper.translate().ty + (y - start.y))
    }
  })
  paper.on('blank:pointerup', () => {
    panning = false
  })
  paper.el.addEventListener('wheel', (event: WheelEvent) => {
    event.preventDefault()
    const delta = event.deltaY
    if (event.ctrlKey) {
      const offset = paper.clientToLocalPoint(event.clientX, event.clientY)
      const currentScale = Math.max(0.3, Math.min(2.5, paper.scale().sx - delta * 0.001))
      paper.scaleUniformAtPoint(currentScale, offset)
    } else {
      paper.translate(paper.translate().tx, paper.translate().ty - delta * 0.2)
    }
  })
}

function createBoard(t: BusType) {
  const graph = new joint.dia.Graph()
  const board = new HardwareBoard(graph)
  boards[t] = board
  const paper = new joint.dia.Paper({
    el: canvasEls[t],
    model: graph,
    width: canvasWidth(),
    height: canvasHeight(),
    gridSize: 10,
    drawGrid: false,
    background: { color: 'var(--el-fill-color-blank)' },
    interactive: false,
    defaultLink: () => new joint.shapes.standard.Link(),
    linkPinning: false
  })
  papers[t] = paper
  board.setPaper(paper)
  attachPan(paper)
  bindBoardEvents(t, board)
  renderType(t)
}

function bindBoardEvents(t: BusType, board: HardwareBoard) {
  board.on('addChannel', () => addChannel(t))
  board.on('gear', (id) => openDrawer(t, id))
  board.on('moveUp', (id) => moveChannel(t, id, -1))
  board.on('moveDown', (id) => moveChannel(t, id, 1))
  board.on('addIa', (id) => addIa(t, id))
  board.on('addNode', (id) => addNode(t, id))
  board.on('removeChannel', (id) => removeChannel(t, id))
  board.on('removeChild', (id) => removeChild(board, id))
  board.on('editChild', (id) => editChild(board, id))
}

function buildChannelVMs(type: BusType): ChannelVM[] {
  const list = Object.values(dataBase.channels).filter((c) => c.type === type)

  return list.map((c, idx) => {
    const device = c.deviceId ? dataBase.devices[c.deviceId] : undefined
    const children: ChildVM[] = []
    if (c.deviceId) {
      for (const [iaId, ia] of Object.entries(dataBase.ia)) {
        if (ia.devices.includes(c.deviceId)) {
          children.push({ id: iaId, kind: 'ia', name: ia.name })
        }
      }
      for (const [nodeId, nd] of Object.entries(dataBase.nodes)) {
        if (nd.channel[0] === c.deviceId) {
          children.push({ id: nodeId, kind: 'node', name: nd.name })
        }
      }
    }
    const summary = getDeviceSummary(device)
    const hasIa = children.some((child) => child.kind === 'ia')
    const canDevice = device?.canDevice
    const canVendorNode = canDevice
      ? vendorTree.value.find((node) => node.id === `${type}:${canDevice.vendor}`)
      : undefined
    const detectedCanName = canVendorNode?.children?.find((child) =>
      child.id.endsWith(`:${String(canDevice?.handle)}`)
    )?.label
    const hardwareName =
      canDevice?.hardwareName ||
      detectedCanName ||
      device?.linDevice?.label ||
      device?.ethDevice?.device?.label ||
      device?.serialDevice?.device?.label
    return {
      id: c.id,
      // Channel cards show the driver-reported hardware name once configured;
      // the editable channel label remains only for an unconfigured channel.
      name: hardwareName || c.name,
      configured: !!c.deviceId,
      summary: c.deviceId
        ? {
            vendorDevice: [summary.vendor?.toUpperCase(), summary.model].filter(Boolean).join(' · '),
            baudRes: [summary.param1, summary.param2].filter(Boolean).join(' · ')
          }
        : null,
      canMoveUp: idx > 0,
      canMoveDown: idx < list.length - 1,
      canAddIa: !!c.deviceId && IA_SUPPORTED_TYPES.includes(type) && !hasIa,
      children
    }
  })
}

function renderType(t: BusType) {
  const board = boards[t]
  if (!board) return
  const channels = buildChannelVMs(t)
  board.render(TAB_LABELS[t], channels, canvasWidth(), canvasHeight())
}

function addChannel(t: BusType) {
  const id = v4()
  const count = Object.values(dataBase.channels).filter((c) => c.type === t).length
  dataBase.channels[id] = {
    id,
    type: t,
    name: `${BUS_TYPE_PREFIX[t]}${count}`
  }
  renderType(t)
}

function openDrawer(t: BusType, channelId: string) {
  const ch = dataBase.channels[channelId]
  if (!ch) return
  editingChannel.value = ch
  drawerVisible.value = true
}

function onChannelSaved(channelId: string, deviceId: string) {
  const ch = dataBase.channels[channelId]
  if (ch) {
    ch.deviceId = deviceId
  }
  renderType(activeType.value)
}

function moveChannel(t: BusType, id: string, dir: -1 | 1) {
  const keys = Object.keys(dataBase.channels)
  const typeKeys = keys.filter((k) => dataBase.channels[k].type === t)
  const idx = typeKeys.indexOf(id)
  const swapWith = idx + dir
  if (idx < 0 || swapWith < 0 || swapWith >= typeKeys.length) return
  const otherId = typeKeys[swapWith]
  const idxInAll = keys.indexOf(id)
  const idxOtherInAll = keys.indexOf(otherId)
  ;[keys[idxInAll], keys[idxOtherInAll]] = [keys[idxOtherInAll], keys[idxInAll]]
  const rebuilt: Record<string, HardwareChannel> = {}
  for (const k of keys) rebuilt[k] = dataBase.channels[k]
  dataBase.channels = rebuilt
  renderType(t)
}

function removeChannel(t: BusType, id: string) {
  const ch = dataBase.channels[id]
  if (!ch) return
  ElMessageBox.confirm(
    i18next.t('uds.hardware.dialogs.deleteChannelMessage'),
    i18next.t('uds.hardware.dialogs.warning'),
    {
      confirmButtonText: i18next.t('uds.hardware.dialogs.ok'),
      cancelButtonText: i18next.t('uds.hardware.dialogs.cancel'),
      type: 'warning',
      buttonSize: 'small',
      appendTo: `#win${winKey}`
    }
  )
    .then(() => {
      const deviceId = ch.deviceId
      if (deviceId) {
        for (const iaId of Object.keys(dataBase.ia)) {
          const ia = dataBase.ia[iaId]
          if (ia.devices.includes(deviceId)) {
            ia.devices = ia.devices.filter((d) => d !== deviceId)
            if (ia.devices.length === 0) delete dataBase.ia[iaId]
          }
        }
        for (const nodeId of Object.keys(dataBase.nodes)) {
          const nd = dataBase.nodes[nodeId]
          if (nd.channel.includes(deviceId)) {
            nd.channel = nd.channel.filter((d) => d !== deviceId)
            if (nd.channel.length === 0) delete dataBase.nodes[nodeId]
          }
        }
        delete dataBase.devices[deviceId]
      }
      delete dataBase.channels[id]
      renderType(t)
    })
    .catch(() => null)
}

function addIa(t: BusType, channelId: string) {
  if (!IA_SUPPORTED_TYPES.includes(t)) return
  const ch = dataBase.channels[channelId]
  if (!ch?.deviceId) return
  const id = v4()
  dataBase.ia[id] = {
    id,
    name: i18next.t('uds.network.names.iaTemplate', { label: ch.name }),
    type: t as 'can' | 'lin' | 'pwm' | 'eth' | 'serial',
    devices: [ch.deviceId],
    action: []
  }
  renderType(t)
}

function addNode(t: BusType, channelId: string) {
  const ch = dataBase.channels[channelId]
  if (!ch?.deviceId) return
  const id = v4()
  dataBase.nodes[id] = {
    id,
    name: i18next.t('uds.network.names.nodeTemplate', {
      count: Object.keys(dataBase.nodes).length + 1
    }),
    channel: [ch.deviceId]
  }
  renderType(t)
}

function removeChild(board: HardwareBoard, id: string) {
  const kind = board.getChildKind(id)
  if (kind === 'ia') {
    delete dataBase.ia[id]
  } else if (kind === 'node') {
    const node = dataBase.nodes[id]
    if (node) {
      window.electron.ipcRenderer.invoke(
        'ipc-delete-node',
        projectStore.projectInfo.path,
        projectStore.projectInfo.name,
        cloneDeep(node)
      )
    }
    delete dataBase.nodes[id]
  }
  renderType(activeType.value)
}

function editChild(board: HardwareBoard, id: string) {
  const kind = board.getChildKind(id)
  if (kind === 'ia') {
    const ia = dataBase.ia[id]
    if (!ia) return
    const winTypeByBus: Record<string, string> = {
      can: 'cani',
      lin: 'lini',
      pwm: 'pwmi',
      eth: 'ethi',
      serial: 'seriali'
    }
    const winType = winTypeByBus[ia.type] ?? 'pwmi'
    layout.addWin(winType, `${id}_ia`, { name: ia.name, params: { 'edit-index': id } })
  } else if (kind === 'node') {
    const node = dataBase.nodes[id]
    if (!node) return
    ElMessageBox({
      buttonSize: 'small',
      showConfirmButton: false,
      title: i18next.t('uds.network.udsView.dialogs.editNode', { name: node.name }),
      showClose: false,
      customStyle: {
        width: '600px',
        maxWidth: 'none'
      },
      message: () => h(nodeConfig, { editIndex: id })
    }).catch(() => null)
  }
}

/** Back-compat: projects created with the previous device-tree UI have `devices`
 * entries but no `channels` wrapper yet. Auto-wrap them once on load. */
function ensureChannelsForOrphanDevices() {
  const wrapped = new Set(
    Object.values(dataBase.channels)
      .map((c) => c.deviceId)
      .filter(Boolean)
  )
  for (const [deviceId, device] of Object.entries(dataBase.devices)) {
    if (wrapped.has(deviceId)) continue
    if (
      device.type !== 'can' &&
      device.type !== 'lin' &&
      device.type !== 'eth' &&
      device.type !== 'pwm' &&
      device.type !== 'serial'
    ) {
      continue
    }
    const id = v4()
    const count = Object.values(dataBase.channels).filter((c) => c.type === device.type).length
    dataBase.channels[id] = {
      id,
      type: device.type,
      name: `${BUS_TYPE_PREFIX[device.type]}${count}`,
      deviceId
    }
  }
}

function focusDevice(deviceId: string) {
  const ch = Object.values(dataBase.channels).find((c) => c.deviceId === deviceId)
  if (ch) {
    activeType.value = ch.type
    nextTick(() => openDrawer(ch.type, ch.id))
  }
}

watch(
  () => props.deviceId,
  (v) => {
    if (v) focusDevice(v)
  }
)

watch(
  () => [dataBase.channels, dataBase.devices, dataBase.ia, dataBase.nodes],
  () => {
    renderType(activeType.value)
  },
  { deep: true }
)

watch([() => props.width, () => props.height], () => {
  for (const t of BUS_TYPES) {
    renderType(t)
  }
})

onMounted(() => {
  ensureChannelsForOrphanDevices()
  for (const t of BUS_TYPES) {
    createBoard(t)
  }
  loadVendorTree(activeType.value)
  hardwareRefreshTimer = setInterval(() => {
    void loadVendorTree(activeType.value)
  }, 2000)
  if (props.deviceId) {
    focusDevice(props.deviceId)
  }
})

onBeforeUnmount(() => {
  if (hardwareRefreshTimer) clearInterval(hardwareRefreshTimer)
})
</script>
<style scoped>
.hw-main {
  position: relative;
  height: v-bind('props.height + "px"');
  width: v-bind('props.width + "px"');
  display: flex;
  flex-direction: column;
}

.hw-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px 8px;
}

.hw-tabs {
  flex: 1;
  min-width: 0;
}

.hw-tabs :deep(.el-tabs__header) {
  margin-bottom: 0;
}

.hw-tabs :deep(.el-tabs__nav-wrap::after),
.hw-tabs :deep(.el-tabs__active-bar) {
  display: none;
}

.hw-tabs :deep(.el-tabs__nav) {
  display: flex;
  gap: 6px;
}

.hw-tabs :deep(.el-tabs__item) {
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--el-border-color);
  border-radius: 7px;
  color: var(--el-text-color-regular);
}

.hw-tabs :deep(.el-tabs__item.is-active) {
  border-color: var(--el-color-primary-light-5);
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.hw-more {
  flex-shrink: 0;
  min-width: 28px;
  font-size: 18px;
  letter-spacing: 0;
}

.hw-body {
  display: flex;
  flex: 1;
  min-height: 0;
  gap: 10px;
  padding: 0 10px 10px;
}

.hw-side {
  width: 196px;
  flex: 0 0 196px;
  overflow: hidden;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 7px;
  background: var(--el-fill-color-blank);
}

.hw-side-title {
  height: 32px;
  padding: 0 10px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 12px;
  font-weight: 600;
  line-height: 32px;
  color: var(--el-text-color-primary);
}

.hw-side :deep(.el-tree-node__content) {
  height: 30px;
  padding-right: 6px;
}

.hw-side-node,
.hw-side-label {
  display: flex;
  align-items: center;
  min-width: 0;
}

.hw-side-node {
  flex: 1;
  justify-content: space-between;
}

.hw-side-label {
  gap: 5px;
  overflow: hidden;
  font-size: 12px;
}

.hw-side-label > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hw-side-icon {
  flex: 0 0 auto;
  color: var(--el-color-primary);
}

.hw-side-icon.dim,
.hw-side-serial {
  color: var(--el-text-color-secondary);
}

.hw-side-serial {
  font-size: 10px;
}

.hw-canvas-area {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow: auto;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 10px;
  background: var(--el-fill-color-blank);
}

.hw-canvas {
  position: absolute;
  top: 0;
  left: 0;
}
</style>
