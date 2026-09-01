<template>
  <el-drawer
    :model-value="visible"
    :title="channel ? `${channel.name} ${i18next.t('uds.hardware.channelDrawer.title')}` : ''"
    size="480px"
    :close-on-click-modal="false"
    :before-close="handleBeforeClose"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <template v-if="channel">
      <el-form label-width="90px" size="small" style="padding: 0 4px">
        <el-form-item :label="i18next.t('uds.hardware.channelDrawer.vendor')">
          <el-select
            v-model="vendor"
            :placeholder="i18next.t('uds.hardware.channelDrawer.selectVendor')"
            style="width: 100%"
            :disabled="globalStart"
          >
            <el-option v-for="v in availableVendors" :key="v" :label="v.toUpperCase()" :value="v" />
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="databaseOptions.length > 0 || canChooseDatabase"
          :label="i18next.t('uds.hardware.channelDrawer.database')"
        >
          <el-select
            v-model="channel.databaseId"
            clearable
            style="width: 100%"
            :placeholder="i18next.t('uds.hardware.channelDrawer.selectDatabase')"
            :disabled="globalStart"
            @change="onDatabaseChange"
          >
            <el-option
              v-for="db in databaseOptions"
              :key="db.id"
              :label="db.name"
              :value="db.id"
            />
          </el-select>
          <el-button
            v-if="canChooseDatabase"
            size="small"
            link
            type="primary"
            :disabled="globalStart"
            @click="importDatabase"
          >
            {{ i18next.t('uds.hardware.channelDrawer.importDatabase') }}
          </el-button>
        </el-form-item>
      </el-form>
      <el-divider />
      <div v-if="vendor" class="form-host">
        <component
          :is="formComponent"
          :key="`${channel.id}-${vendor}`"
          ref="formRef"
          v-model="dataModify"
          :index="channel.deviceId ?? ''"
          :vendor="vendor"
          @change="onChange"
        />
      </div>
      <el-empty v-else :description="i18next.t('uds.hardware.channelDrawer.pickVendorFirst')" />
    </template>
    <template #footer>
      <el-button size="small" @click="handleCancel">
        {{ i18next.t('uds.hardware.dialogs.cancel') }}
      </el-button>
      <el-button
        size="small"
        type="primary"
        :disabled="!vendor || globalStart"
        @click="handleSave"
      >
        {{ i18next.t('uds.hardware.channelDrawer.save') }}
      </el-button>
    </template>
  </el-drawer>
</template>
<script lang="ts" setup>
import { ref, computed, watch, nextTick, inject } from 'vue'
import { ElMessageBox } from 'element-plus'
import i18next from 'i18next'
import type { CanVendor } from 'nodeCan/can'
import type { HardwareChannel } from 'src/preload/data'
import { useDataStore } from '@r/stores/data'
import { useGlobalStart } from '@r/stores/runtime'
import { Layout } from '../layout'
import { v4 } from 'uuid'
import { vendorSupportsType, getDeviceSummary, type BusType } from './busTypes'
import { ecubusPro } from '../../../../../../package.json'
import canNodeVue from './canNode.vue'
import ethNodeVue from './ethNode.vue'
import linNodeVue from './linNode.vue'
import pwmNodeVue from './pwmNode.vue'
import serialNodeVue from './serialNode.vue'

const props = defineProps<{
  visible: boolean
  channel: HardwareChannel | null
  busType: BusType
  presetVendor?: CanVendor
}>()
const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'saved', channelId: string, deviceId: string): void
}>()

const dataBase = useDataStore()
const globalStart = useGlobalStart()
const layout = inject('layout') as Layout

const vendor = ref<CanVendor | undefined>()
const dataModify = ref(false)
const formRef = ref()
const allVendors = ref<CanVendor[]>([])

const availableVendors = computed(() =>
  allVendors.value.filter((v) => vendorSupportsType(v, props.busType))
)

const canChooseDatabase = computed(() => props.busType === 'can' || props.busType === 'lin')
const databaseOptions = computed(() => {
  const bucket = props.busType === 'can' ? dataBase.database.can : props.busType === 'lin' ? dataBase.database.lin : {}
  return Object.entries(bucket).map(([id, db]) => ({ id, name: (db as { name?: string }).name || id }))
})

const formComponent = computed(() => {
  switch (props.busType) {
    case 'can':
      return canNodeVue
    case 'eth':
      return ethNodeVue
    case 'lin':
      return linNodeVue
    case 'pwm':
      return pwmNodeVue
    case 'serial':
      return serialNodeVue
    default:
      return undefined
  }
})

async function loadVendors() {
  allVendors.value = (await window.electron.ipcRenderer.invoke('ipc-get-vendor', ecubusPro)).map(
    (v: any) => v.name
  )
}

watch(
  () => props.visible,
  async (v) => {
    if (v && props.channel) {
      dataModify.value = false
      await loadVendors()
      const device = props.channel.deviceId ? dataBase.devices[props.channel.deviceId] : undefined
      const summary = getDeviceSummary(device)
      if (!props.channel.databaseId && device) {
        props.channel.databaseId =
          props.busType === 'can'
            ? device.canDevice?.database
            : props.busType === 'lin'
              ? device.linDevice?.database
              : undefined
      }
      if (props.presetVendor && availableVendors.value.includes(props.presetVendor)) {
        vendor.value = props.presetVendor
      } else if (summary.vendor && availableVendors.value.includes(summary.vendor as CanVendor)) {
        vendor.value = summary.vendor as CanVendor
      } else {
        vendor.value = availableVendors.value[0]
      }
    }
  }
)

function onChange(id: string) {
  if (props.channel) {
    emit('saved', props.channel.id, id)
  }
}

function onDatabaseChange(databaseId: string | undefined) {
  if (!props.channel?.deviceId) return
  const device = dataBase.devices[props.channel.deviceId]
  if (props.busType === 'can' && device?.canDevice) device.canDevice.database = databaseId || ''
  if (props.busType === 'lin' && device?.linDevice) device.linDevice.database = databaseId || ''
}

async function handleSave() {
  const ok = await formRef.value?.save?.()
  if (ok) {
    nextTick(() => emit('update:visible', false))
  }
}

async function importDatabase() {
  if (!props.channel || !canChooseDatabase.value) return
  const type = props.busType === 'can' ? 'can' : 'lin'
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
  const id = v4()
  props.channel.databaseId = id
  layout.addWin(type === 'can' ? 'dbc' : 'ldf', id, {
    params: {
      'edit-index': id,
      ...(type === 'can' ? { dbcFile: file } : { ldfFile: file })
    }
  })
}

function handleCancel() {
  attemptClose(() => emit('update:visible', false))
}

function attemptClose(done: () => void) {
  if (dataModify.value) {
    ElMessageBox.confirm(
      i18next.t('uds.hardware.dialogs.discardChangesMessage'),
      i18next.t('uds.hardware.dialogs.warning'),
      {
        confirmButtonText: i18next.t('uds.hardware.dialogs.discard'),
        cancelButtonText: i18next.t('uds.hardware.dialogs.cancel'),
        type: 'warning',
        buttonSize: 'small'
      }
    )
      .then(() => {
        dataModify.value = false
        done()
      })
      .catch(() => {
        null
      })
  } else {
    done()
  }
}

function handleBeforeClose(done: () => void) {
  attemptClose(done)
}
</script>
<style scoped>
.form-host {
  max-height: calc(100vh - 260px);
  overflow: auto;
}
</style>
