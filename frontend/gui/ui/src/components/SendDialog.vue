<template>
  <div class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h3>发送文件</h3>
        <button class="btn-close" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <div class="device-info">
          <span class="device-icon">{{ deviceIcon }}</span>
          <span class="device-name">{{ targetDevice?.name }}</span>
        </div>

        <FileSelector v-model="selectedFiles" />

        <div class="options">
          <label>
            <input type="checkbox" v-model="options.anonymous" />
            匿名发送 (无需 PIN)
          </label>
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn-cancel" @click="handleClose">取消</button>
        <button class="btn-send" @click="handleSend" :disabled="selectedFiles.length === 0 || sending">
          {{ sending ? '发送中...' : '发送' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useUIStore } from '../stores/uiStore'
import { useDeviceStore } from '../stores/deviceStore'
import { useTransferStore } from '../stores/transferStore'
import { DEVICE_TYPE } from '../utils/constants'
import FileSelector from './FileSelector.vue'

const uiStore = useUIStore()
const deviceStore = useDeviceStore()
const transferStore = useTransferStore()

const selectedFiles = ref([])
const sending = ref(false)
const options = ref({
  anonymous: false
})

const targetDevice = computed(() => {
  return deviceStore.devices.find(d => d.id === uiStore.selectedDeviceId)
})

const deviceIcon = computed(() => {
  if (!targetDevice.value) return '💻'
  const icons = {
    [DEVICE_TYPE.PHONE]: '📱',
    [DEVICE_TYPE.COMPUTER]: '💻',
    [DEVICE_TYPE.TABLET]: '📱'
  }
  return icons[targetDevice.value.type] || '💻'
})

async function handleSend() {
  if (selectedFiles.value.length === 0) return

  sending.value = true
  try {
    const paths = selectedFiles.value.map(f => f.path || f.name)
    await transferStore.sendFiles(paths, targetDevice.value.id)
    handleClose()
  } catch (e) {
    console.error('发送失败:', e)
  } finally {
    sending.value = false
  }
}

function handleClose() {
  uiStore.closeSendDialog()
  selectedFiles.value = []
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.dialog {
  background: white;
  border-radius: 16px;
  width: 100%;
  max-width: 500px;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid #f0f0f0;
}

.dialog-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: #333;
}

.btn-close {
  width: 32px;
  height: 32px;
  border: none;
  background: #f5f5f5;
  color: #666;
  border-radius: 50%;
  font-size: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-close:hover {
  background: #e0e0e0;
}

.dialog-body {
  padding: 24px;
  overflow-y: auto;
}

.device-info {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
  padding: 16px;
  background: #f5f5f5;
  border-radius: 10px;
}

.device-icon {
  font-size: 32px;
}

.device-name {
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.options {
  margin-top: 16px;
}

.options label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #666;
  cursor: pointer;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 20px 24px;
  border-top: 1px solid #f0f0f0;
}

.btn-cancel,
.btn-send {
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel {
  background: white;
  color: #666;
  border: 1px solid #ddd;
}

.btn-cancel:hover {
  background: #f5f5f5;
}

.btn-send {
  background: #4CAF50;
  color: white;
  border: none;
}

.btn-send:hover:not(:disabled) {
  background: #43A047;
}

.btn-send:disabled {
  background: #a5d6a7;
  cursor: not-allowed;
}
</style>
