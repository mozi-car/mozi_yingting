<template>
  <div class="ethi" :style="{ height: height + 'px' }">
    <el-tabs v-model="activeTab" type="card" class="ethi-tabs">
      <el-tab-pane :label="i18next.t('uds.network.ethi.tabs.actions')" name="actions">
        <div class="tab-content">
          <div class="toolbar">
            <el-button size="small" type="primary" :icon="AddIcon" @click="openAdd">
              {{ i18next.t('uds.network.ethi.buttons.add') }}
            </el-button>
          </div>
          <el-table :data="ia.action" size="small" border style="width: 100%">
            <el-table-column
              :label="i18next.t('uds.network.ethi.columns.name')"
              prop="name"
              min-width="110"
            />
            <el-table-column
              :label="i18next.t('uds.network.ethi.columns.protocol')"
              width="80"
            >
              <template #default="{ row }">{{ row.protocol.toUpperCase() }}</template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.ethi.columns.dest')" min-width="150">
              <template #default="{ row }">
                <span v-if="row.protocol === 'raw'">{{ row.dstMac || '-' }}</span>
                <span v-else>{{ (row.dstIp || '-') + ':' + (row.dstPort ?? '-') }}</span>
              </template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.ethi.columns.trigger')" width="110">
              <template #default="{ row }">
                {{
                  row.trigger.type === 'periodic'
                    ? `${i18next.t('uds.network.ethi.trigger.periodic')} ${row.trigger.period ?? 0}ms`
                    : i18next.t('uds.network.ethi.trigger.manual')
                }}
              </template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.ethi.columns.payload')" min-width="140">
              <template #default="{ row }">
                <span class="mono">{{ row.payload || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.ethi.columns.ops')" width="150">
              <template #default="{ row, $index }">
                <el-button
                  link
                  type="primary"
                  size="small"
                  :disabled="!isConnected || row.trigger.type !== 'manual'"
                  @click="sendNow(row)"
                >
                  {{ i18next.t('uds.network.ethi.buttons.send') }}
                </el-button>
                <el-button link type="primary" size="small" @click="openEdit($index)">
                  {{ i18next.t('uds.network.ethi.buttons.edit') }}
                </el-button>
                <el-button link type="danger" size="small" @click="removeAction($index)">
                  {{ i18next.t('uds.network.ethi.buttons.delete') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-tab-pane>

      <el-tab-pane :label="i18next.t('uds.network.ethi.tabs.device')" name="device">
        <div class="tab-content">
          <el-transfer
            v-model="ia.devices"
            style="text-align: left; display: inline-block"
            :data="allDeviceLabel"
            :titles="[
              i18next.t('uds.network.ethi.transfer.valid'),
              i18next.t('uds.network.ethi.transfer.assigned')
            ]"
            :disabled="isConnected"
          />
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="dialogVisible"
      :title="i18next.t('uds.network.ethi.dialog.title')"
      width="560px"
      append-to-body
    >
      <el-form v-if="draft" label-width="120px" size="small">
        <el-form-item :label="i18next.t('uds.network.ethi.fields.name')">
          <el-input v-model="draft.name" />
        </el-form-item>
        <el-form-item :label="i18next.t('uds.network.ethi.fields.protocol')">
          <el-radio-group v-model="draft.protocol">
            <el-radio-button value="udp">UDP</el-radio-button>
            <el-radio-button value="tcp">TCP</el-radio-button>
            <el-radio-button value="raw">RAW</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          v-if="draft.protocol !== 'raw'"
          :label="i18next.t('uds.network.ethi.fields.autoHeader')"
        >
          <el-switch v-model="draft.autoHeader" />
          <span class="hint">{{ i18next.t('uds.network.ethi.fields.autoHeaderHint') }}</span>
        </el-form-item>

        <template v-if="draft.protocol !== 'raw'">
          <template v-if="!draft.autoHeader">
            <el-form-item :label="i18next.t('uds.network.ethi.fields.srcIp')">
              <el-input v-model="draft.srcIp" placeholder="192.168.1.10" />
            </el-form-item>
            <el-form-item :label="i18next.t('uds.network.ethi.fields.srcPort')">
              <el-input-number v-model="draft.srcPort" :min="0" :max="65535" controls-position="right" />
            </el-form-item>
          </template>
          <el-form-item :label="i18next.t('uds.network.ethi.fields.dstIp')">
            <el-input v-model="draft.dstIp" placeholder="192.168.1.20" />
          </el-form-item>
          <el-form-item :label="i18next.t('uds.network.ethi.fields.dstPort')">
            <el-input-number v-model="draft.dstPort" :min="0" :max="65535" controls-position="right" />
          </el-form-item>
          <el-form-item :label="i18next.t('uds.network.ethi.fields.ttl')">
            <el-input-number v-model="draft.ttl" :min="1" :max="255" controls-position="right" />
          </el-form-item>
        </template>
        <template v-else>
          <el-form-item :label="i18next.t('uds.network.ethi.fields.dstMac')">
            <el-input v-model="draft.dstMac" placeholder="AA:BB:CC:DD:EE:FF" />
          </el-form-item>
        </template>

        <el-form-item :label="i18next.t('uds.network.ethi.fields.payload')">
          <el-input
            v-model="draft.payload"
            type="textarea"
            :rows="2"
            placeholder="02 FD 00 00"
          />
        </el-form-item>
        <el-form-item :label="i18next.t('uds.network.ethi.fields.trigger')">
          <el-radio-group v-model="draft.trigger.type">
            <el-radio-button value="manual">
              {{ i18next.t('uds.network.ethi.trigger.manual') }}
            </el-radio-button>
            <el-radio-button value="periodic">
              {{ i18next.t('uds.network.ethi.trigger.periodic') }}
            </el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          v-if="draft.trigger.type === 'periodic'"
          :label="i18next.t('uds.network.ethi.fields.period')"
        >
          <el-input-number v-model="draft.trigger.period" :min="1" controls-position="right" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="dialogVisible = false">
          {{ i18next.t('uds.network.ethi.buttons.cancel') }}
        </el-button>
        <el-button size="small" type="primary" @click="saveDraft">
          {{ i18next.t('uds.network.ethi.buttons.ok') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, toRef, watch, onUnmounted } from 'vue'
import { cloneDeep } from 'lodash'
import { v4 } from 'uuid'
import i18next from 'i18next'
import AddIcon from '@iconify/icons-material-symbols/add'
import { useDataStore } from '@r/stores/data'
import { useGlobalStart } from '@r/stores/runtime'
import type { EthInter, EthInterAction } from 'src/preload/data'

const props = defineProps<{ height: number; editIndex: string }>()
const height = toRef(props, 'height')
const dataBase = useDataStore()
const isConnected = useGlobalStart()

const ia = computed(() => dataBase.ia[props.editIndex] as EthInter)
const activeTab = ref('actions')

const allDeviceLabel = computed(() => {
  const list: { key: string; label: string; disabled: boolean }[] = []
  for (const id of Object.keys(dataBase.devices)) {
    const d = dataBase.devices[id]
    if (d && d.type === 'eth' && d.ethDevice) {
      list.push({ key: id, label: d.ethDevice.name, disabled: false })
    }
  }
  return list
})

const dialogVisible = ref(false)
const draft = ref<EthInterAction | null>(null)
const editingIndex = ref(-1)

function newAction(): EthInterAction {
  return {
    uuid: v4(),
    name: i18next.t('uds.network.ethi.defaultName'),
    protocol: 'udp',
    autoHeader: true,
    dstIp: '',
    dstPort: 0,
    ttl: 64,
    payload: '',
    trigger: { type: 'manual' }
  }
}

function openAdd() {
  draft.value = newAction()
  editingIndex.value = -1
  dialogVisible.value = true
}

function openEdit(index: number) {
  draft.value = cloneDeep(ia.value.action[index])
  editingIndex.value = index
  dialogVisible.value = true
}

function saveDraft() {
  if (!draft.value) return
  if (editingIndex.value >= 0) {
    ia.value.action[editingIndex.value] = draft.value
  } else {
    ia.value.action.push(draft.value)
  }
  dialogVisible.value = false
}

function removeAction(index: number) {
  ia.value.action.splice(index, 1)
}

function sendNow(row: EthInterAction) {
  window.electron.ipcRenderer.send('ipc-send-eth', cloneDeep(ia.value), cloneDeep(row))
}

const periodKeys = new Set<string>()
function stopAllPeriodic() {
  for (const key of periodKeys) {
    window.electron.ipcRenderer.send('ipc-raw-ia-stop-period', key)
  }
  periodKeys.clear()
}
function startPeriodic() {
  for (const action of ia.value.action) {
    if (action.trigger.type === 'periodic') {
      const key = `${ia.value.id}:${action.uuid}`
      periodKeys.add(key)
      window.electron.ipcRenderer.send(
        'ipc-eth-start-period',
        key,
        cloneDeep(ia.value),
        cloneDeep(action)
      )
    }
  }
}
watch(isConnected, (on) => {
  if (on) startPeriodic()
  else stopAllPeriodic()
})
onUnmounted(stopAllPeriodic)
</script>

<style scoped>
.ethi {
  padding: 8px;
  box-sizing: border-box;
}
.ethi-tabs {
  height: 100%;
}
.tab-content {
  padding: 8px;
}
.toolbar {
  margin-bottom: 8px;
}
.mono {
  font-family: monospace;
}
.hint {
  margin-left: 10px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
