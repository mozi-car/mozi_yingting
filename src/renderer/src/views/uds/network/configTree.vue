<template>
  <div class="net-cfg" :style="{ height: height + 'px', width: width + 'px' }">
    <el-scrollbar :height="height - 8 + 'px'">
      <el-tree
        :data="treeData"
        node-key="id"
        :props="{ children: 'children', label: 'label' }"
        default-expand-all
        :expand-on-click-node="false"
        @node-click="onNodeClick"
      >
        <template #default="{ node, data }">
          <span class="row">
            <span class="row-label">
              <Icon v-if="data.icon" :icon="data.icon" class="row-icon" />
              <span :class="{ isGroup: data.kind === 'group', isNet: data.kind === 'network' }">
                {{ node.label }}
              </span>
              <span v-if="data.sub" class="row-sub">{{ data.sub }}</span>
            </span>
            <el-button
              v-if="data.addKind"
              link
              type="primary"
              :disabled="data.disabled"
              :title="i18next.t('uds.network.configTree.add')"
              @click.stop="onAdd(data)"
            >
              <Icon :icon="plusIcon" />
            </el-button>
          </span>
        </template>
      </el-tree>
      <el-empty
        v-if="treeData.length === 0"
        :description="i18next.t('uds.network.configTree.empty')"
        :image-size="70"
      />
    </el-scrollbar>
  </div>
</template>

<script lang="ts" setup>
import { computed, inject, h } from 'vue'
import { Icon } from '@iconify/vue'
import { ElMessageBox } from 'element-plus'
import i18next from 'i18next'
import { v4 } from 'uuid'
import networkNode from '@iconify/icons-material-symbols/network-node'
import folderIcon from '@iconify/icons-material-symbols/folder-outline'
import deviceIcon from '@iconify/icons-material-symbols/important-devices-outline'
import interIcon from '@iconify/icons-material-symbols/interactive-space-outline'
import nodeIcon from '@iconify/icons-material-symbols/variables-outline-rounded'
import replayIcon from '@iconify/icons-material-symbols/replay'
import databaseIcon from '@iconify/icons-material-symbols/database'
import plusIcon from '@iconify/icons-ep/circle-plus-filled'
import { useDataStore } from '@r/stores/data'
import { Layout } from '../layout'
import { getDeviceSummary, BUS_TYPE_PREFIX, IA_SUPPORTED_TYPES, type BusType } from '../hardware/busTypes'
import nodeConfig from './nodeConfig.vue'
import replayConfig from './replayConfig.vue'

defineProps<{ height: number; width: number }>()

const dataBase = useDataStore()
const layout = inject('layout') as Layout

const BUS_GROUPS: { type: BusType; label: string }[] = [
  { type: 'can', label: 'CAN Networks' },
  { type: 'lin', label: 'LIN Networks' },
  { type: 'eth', label: 'Ethernet Networks' },
  { type: 'pwm', label: 'PWM Networks' },
  { type: 'serial', label: 'Serial Networks' }
]

const IA_WIN: Record<string, string> = {
  can: 'cani',
  lin: 'lini',
  pwm: 'pwmi',
  eth: 'ethi',
  serial: 'seriali'
}

interface TreeNode {
  id: string
  kind: string
  label: string
  sub?: string
  icon?: object
  refId?: string
  iaType?: BusType
  dbKind?: 'can' | 'lin'
  channelId?: string
  busType?: BusType
  addKind?: 'node' | 'ia' | 'database'
  disabled?: boolean
  children?: TreeNode[]
}

/** Builds the per-channel config tree straight from the device view's `dataBase.channels`. */
const treeData = computed<TreeNode[]>(() => {
  const groups: TreeNode[] = []
  for (const g of BUS_GROUPS) {
    const channels = Object.values(dataBase.channels).filter((c) => c.type === g.type)
    if (channels.length === 0) continue

    const groupNode: TreeNode = {
      id: `group:${g.type}`,
      kind: 'group',
      label: g.label,
      icon: networkNode,
      children: []
    }

    for (const ch of channels) {
      const deviceId = ch.deviceId
      const device = deviceId ? dataBase.devices[deviceId] : undefined
      const summary = getDeviceSummary(device)
      const hasIa = deviceId
        ? Object.values(dataBase.ia).some(
            (ia) => ia.type === g.type && ia.devices.includes(deviceId)
          )
        : false

      const nodesCat: TreeNode = {
        id: `nodes:${ch.id}`,
        kind: 'cat',
        label: i18next.t('uds.network.configTree.nodes'),
        icon: nodeIcon,
        channelId: ch.id,
        busType: g.type,
        addKind: 'node',
        children: []
      }
      const iaCat: TreeNode = {
        id: `ia:${ch.id}`,
        kind: 'cat',
        label: i18next.t('uds.network.configTree.interactive'),
        icon: interIcon,
        channelId: ch.id,
        busType: g.type,
        addKind: IA_SUPPORTED_TYPES.includes(g.type) ? 'ia' : undefined,
        disabled: hasIa,
        children: []
      }
      const replayCat: TreeNode = {
        id: `replay:${ch.id}`,
        kind: 'cat',
        label: i18next.t('uds.network.configTree.replays'),
        icon: replayIcon,
        children: []
      }
      const dbCat: TreeNode = {
        id: `db:${ch.id}`,
        kind: 'cat',
        label: i18next.t('uds.network.configTree.databases'),
        icon: databaseIcon,
        channelId: ch.id,
        busType: g.type,
        addKind: g.type === 'can' || g.type === 'lin' ? 'database' : undefined,
        children: []
      }
      const channelsCat: TreeNode = {
        id: `channels:${ch.id}`,
        kind: 'cat',
        label: i18next.t('uds.network.configTree.channels'),
        icon: deviceIcon,
        children: [
          {
            id: `channel:${ch.id}`,
            kind: 'leaf-channel',
            label: ch.name,
            sub: [summary.vendor?.toUpperCase(), summary.model].filter(Boolean).join(' · '),
            icon: deviceIcon,
            refId: ch.id
          }
        ]
      }

      if (deviceId) {
        for (const [iaId, ia] of Object.entries(dataBase.ia)) {
          if (ia.devices.includes(deviceId)) {
            iaCat.children!.push({
              id: `ia-leaf:${iaId}`,
              kind: 'leaf-ia',
              label: ia.name,
              icon: interIcon,
              refId: iaId,
              iaType: ia.type as BusType
            })
          }
        }
        for (const [nodeId, nd] of Object.entries(dataBase.nodes)) {
          if (nd.channel.includes(deviceId)) {
            nodesCat.children!.push({
              id: `node-leaf:${nodeId}`,
              kind: 'leaf-node',
              label: nd.name,
              icon: nodeIcon,
              refId: nodeId
            })
          }
        }
        for (const [replayId, rp] of Object.entries(dataBase.replays)) {
          if (rp.channel.includes(deviceId)) {
            replayCat.children!.push({
              id: `replay-leaf:${replayId}`,
              kind: 'leaf-replay',
              label: rp.name,
              icon: replayIcon,
              refId: replayId
            })
          }
        }
      }

      const dbBucket = g.type === 'can' ? dataBase.database.can : g.type === 'lin' ? dataBase.database.lin : undefined
      if (dbBucket) {
        for (const [dbId, db] of Object.entries(dbBucket)) {
          dbCat.children!.push({
            id: `db-leaf:${dbId}`,
            kind: 'leaf-db',
            label: `${(db as { name?: string }).name || dbId}${ch.databaseId === dbId ? ' *' : ''}`,
            icon: databaseIcon,
            refId: dbId,
            channelId: ch.id,
            dbKind: g.type === 'can' ? 'can' : 'lin'
          })
        }
      }

      groupNode.children!.push({
        id: `net:${ch.id}`,
        kind: 'network',
        label: ch.name,
        icon: folderIcon,
        children: [nodesCat, iaCat, replayCat, dbCat, channelsCat]
      })
    }

    groups.push(groupNode)
  }
  return groups
})

function onNodeClick(data: TreeNode) {
  if (data.kind === 'leaf-node' && data.refId) {
    ElMessageBox({
      buttonSize: 'small',
      showConfirmButton: false,
      showClose: true,
      title: dataBase.nodes[data.refId]?.name ?? data.label,
      customStyle: { width: '600px', maxWidth: 'none' },
      message: () => h(nodeConfig, { editIndex: data.refId as string })
    }).catch(() => null)
  } else if (data.kind === 'leaf-ia' && data.refId && data.iaType) {
    const win = IA_WIN[data.iaType] ?? 'pwmi'
    layout.addWin(win, `${data.refId}_ia`, {
      name: data.label,
      params: { 'edit-index': data.refId }
    })
  } else if (data.kind === 'leaf-replay' && data.refId) {
    ElMessageBox({
      buttonSize: 'small',
      showConfirmButton: false,
      showClose: true,
      title: dataBase.replays[data.refId]?.name ?? data.label,
      customStyle: { width: '700px', maxWidth: 'none' },
      message: () => h(replayConfig, { editIndex: data.refId as string })
    }).catch(() => null)
  } else if (data.kind === 'leaf-db' && data.refId) {
    if (data.channelId) {
      const channel = dataBase.channels[data.channelId]
      if (channel) channel.databaseId = data.refId
    }
    layout.addWin(data.dbKind === 'can' ? 'dbc' : 'ldf', data.refId, { name: data.label })
  } else if (data.kind === 'leaf-channel' && data.refId) {
    // Jump to the Devices view to (re)configure this channel's hardware mapping.
    const deviceId = dataBase.channels[data.refId]?.deviceId
    layout.addWin('hardware', 'hardware', {
      params: deviceId ? { deviceId } : undefined
    })
  }
}

async function onAdd(data: TreeNode) {
  const ch = data.channelId ? dataBase.channels[data.channelId] : undefined
  if (!ch?.deviceId) {
    ElMessageBox.alert(i18next.t('uds.network.configTree.needDevice'), {
      type: 'warning',
      buttonSize: 'small'
    }).catch(() => null)
    return
  }
  if (data.addKind === 'ia' && data.busType) {
    const hasIa = Object.values(dataBase.ia).some(
      (ia) => ia.type === data.busType && ia.devices.includes(ch.deviceId as string)
    )
    if (hasIa) {
      ElMessageBox.alert(i18next.t('uds.network.configTree.iaLimit'), {
        type: 'warning',
        buttonSize: 'small'
      }).catch(() => null)
      return
    }
  }
  const id = v4()
  if (data.addKind === 'node') {
    dataBase.nodes[id] = {
      id,
      name: i18next.t('uds.network.names.nodeTemplate', {
        count: Object.keys(dataBase.nodes).length + 1
      }),
      channel: [ch.deviceId]
    }
  } else if (data.addKind === 'ia' && data.busType) {
    dataBase.ia[id] = {
      id,
      name: i18next.t('uds.network.names.iaTemplate', { label: ch.name }),
      type: data.busType as 'can' | 'lin' | 'pwm' | 'eth' | 'serial',
      devices: [ch.deviceId],
      action: []
    }
  } else if (data.addKind === 'database' && (data.busType === 'can' || data.busType === 'lin')) {
    const type = data.busType
    const result = await window.electron.ipcRenderer.invoke('ipc-show-open-dialog', {
      title: i18next.t('database.openDatabaseTitle'),
      properties: ['openFile'],
      filters: [{
        name: i18next.t('database.fileFilterName'),
        extensions: type === 'can' ? ['dbc', 'arxml'] : ['ldf']
      }]
    })
    const file = result?.filePaths?.[0]
    if (!file) return
    const dbId = v4()
    ch.databaseId = dbId
    layout.addWin(type === 'can' ? 'dbc' : 'ldf', dbId, {
      params: {
        'edit-index': dbId,
        ...(type === 'can' ? { dbcFile: file } : { ldfFile: file })
      }
    })
  }
}
</script>

<style scoped>
.net-cfg {
  box-sizing: border-box;
  padding: 6px 4px;
  overflow: hidden;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding-right: 8px;
}
.row-label {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}
.row-icon {
  flex: 0 0 auto;
  color: var(--el-text-color-secondary);
}
.isGroup {
  font-weight: 700;
}
.isNet {
  font-weight: 600;
  color: var(--el-color-primary);
}
.row-sub {
  margin-left: 8px;
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
</style>
