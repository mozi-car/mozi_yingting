<template>
  <div class="seriali" :style="{ height: height + 'px' }">
    <el-tabs v-model="activeTab" type="card" class="seriali-tabs">
      <el-tab-pane :label="i18next.t('uds.network.seriali.tabs.actions')" name="actions">
        <div class="tab-content">
          <div class="toolbar">
            <el-button size="small" type="primary" :icon="AddIcon" @click="openAdd">
              {{ i18next.t('uds.network.seriali.buttons.add') }}
            </el-button>
          </div>
          <el-table :data="ia.action" size="small" border style="width: 100%">
            <el-table-column
              :label="i18next.t('uds.network.seriali.columns.name')"
              prop="name"
              min-width="120"
            />
            <el-table-column :label="i18next.t('uds.network.seriali.columns.encoding')" width="90">
              <template #default="{ row }">{{ row.encoding.toUpperCase() }}</template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.seriali.columns.trigger')" width="120">
              <template #default="{ row }">
                {{
                  row.trigger.type === 'periodic'
                    ? `${i18next.t('uds.network.seriali.trigger.periodic')} ${row.trigger.period ?? 0}ms`
                    : i18next.t('uds.network.seriali.trigger.manual')
                }}
              </template>
            </el-table-column>
            <el-table-column
              :label="i18next.t('uds.network.seriali.columns.payload')"
              min-width="160"
            >
              <template #default="{ row }">
                <span class="mono">{{ row.payload || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column :label="i18next.t('uds.network.seriali.columns.ops')" width="150">
              <template #default="{ row, $index }">
                <el-button
                  link
                  type="primary"
                  size="small"
                  :disabled="!isConnected || row.trigger.type !== 'manual'"
                  @click="sendNow(row)"
                >
                  {{ i18next.t('uds.network.seriali.buttons.send') }}
                </el-button>
                <el-button link type="primary" size="small" @click="openEdit($index)">
                  {{ i18next.t('uds.network.seriali.buttons.edit') }}
                </el-button>
                <el-button link type="danger" size="small" @click="removeAction($index)">
                  {{ i18next.t('uds.network.seriali.buttons.delete') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-tab-pane>

      <el-tab-pane :label="i18next.t('uds.network.seriali.tabs.device')" name="device">
        <div class="tab-content">
          <el-transfer
            v-model="ia.devices"
            style="text-align: left; display: inline-block"
            :data="allDeviceLabel"
            :titles="[
              i18next.t('uds.network.seriali.transfer.valid'),
              i18next.t('uds.network.seriali.transfer.assigned')
            ]"
            :disabled="isConnected"
          />
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="dialogVisible"
      :title="i18next.t('uds.network.seriali.dialog.title')"
      width="520px"
      append-to-body
    >
      <el-form v-if="draft" label-width="120px" size="small">
        <el-form-item :label="i18next.t('uds.network.seriali.fields.name')">
          <el-input v-model="draft.name" />
        </el-form-item>
        <el-form-item :label="i18next.t('uds.network.seriali.fields.encoding')">
          <el-radio-group v-model="draft.encoding">
            <el-radio-button value="hex">HEX</el-radio-button>
            <el-radio-button value="ascii">ASCII</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          v-if="draft.encoding === 'ascii'"
          :label="i18next.t('uds.network.seriali.fields.lineEnding')"
        >
          <el-select v-model="draft.lineEnding" style="width: 160px">
            <el-option label="None" value="none" />
            <el-option label="LF (\n)" value="lf" />
            <el-option label="CR (\r)" value="cr" />
            <el-option label="CRLF (\r\n)" value="crlf" />
          </el-select>
        </el-form-item>
        <el-form-item :label="i18next.t('uds.network.seriali.fields.payload')">
          <el-input
            v-model="draft.payload"
            type="textarea"
            :rows="2"
            :placeholder="draft.encoding === 'hex' ? '02 FD 00 00' : 'AT+RESET'"
          />
        </el-form-item>
        <el-form-item :label="i18next.t('uds.network.seriali.fields.trigger')">
          <el-radio-group v-model="draft.trigger.type">
            <el-radio-button value="manual">
              {{ i18next.t('uds.network.seriali.trigger.manual') }}
            </el-radio-button>
            <el-radio-button value="periodic">
              {{ i18next.t('uds.network.seriali.trigger.periodic') }}
            </el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          v-if="draft.trigger.type === 'periodic'"
          :label="i18next.t('uds.network.seriali.fields.period')"
        >
          <el-input-number v-model="draft.trigger.period" :min="1" controls-position="right" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="dialogVisible = false">
          {{ i18next.t('uds.network.seriali.buttons.cancel') }}
        </el-button>
        <el-button size="small" type="primary" @click="saveDraft">
          {{ i18next.t('uds.network.seriali.buttons.ok') }}
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
import type { SerialInter, SerialInterAction } from 'src/preload/data'

const props = defineProps<{ height: number; editIndex: string }>()
const height = toRef(props, 'height')
const dataBase = useDataStore()
const isConnected = useGlobalStart()

const ia = computed(() => dataBase.ia[props.editIndex] as SerialInter)
const activeTab = ref('actions')

const allDeviceLabel = computed(() => {
  const list: { key: string; label: string; disabled: boolean }[] = []
  for (const id of Object.keys(dataBase.devices)) {
    const d = dataBase.devices[id]
    if (d && d.type === 'serial' && d.serialDevice) {
      list.push({ key: id, label: d.serialDevice.name, disabled: false })
    }
  }
  return list
})

const dialogVisible = ref(false)
const draft = ref<SerialInterAction | null>(null)
const editingIndex = ref(-1)

function newAction(): SerialInterAction {
  return {
    uuid: v4(),
    name: i18next.t('uds.network.seriali.defaultName'),
    payload: '',
    encoding: 'hex',
    lineEnding: 'none',
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

function sendNow(row: SerialInterAction) {
  window.electron.ipcRenderer.send('ipc-send-serial', cloneDeep(ia.value), cloneDeep(row))
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
        'ipc-serial-start-period',
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
.seriali {
  padding: 8px;
  box-sizing: border-box;
}
.seriali-tabs {
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
</style>
