<template>
  <div class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h3>收到文件</h3>
        <button class="btn-close" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <!-- 请求列表 -->
        <div class="requests-list" v-if="requests.length > 1">
          <div
            class="request-item"
            :class="{ active: selectedRequestIndex === index }"
            v-for="(req, index) in requests"
            :key="req.sessionId"
            @click="selectRequest(index)"
          >
            <span class="request-icon">📥</span>
            <span class="request-name">{{ req.senderName || '未知设备' }}</span>
            <span class="request-files">{{ req.files.length }} 个文件</span>
          </div>
        </div>

        <!-- 当前请求详情 -->
        <template v-if="currentRequest">
          <div class="sender-info">
            <span class="device-icon">{{ deviceIcon }}</span>
            <div class="sender-details">
              <span class="sender-name">{{ currentRequest.senderName || '未知设备' }}</span>
              <span class="sender-ip" v-if="currentRequest.senderId">
                ID: {{ currentRequest.senderId.substring(0, 8) }}...
              </span>
            </div>
          </div>

          <div class="file-list">
            <div class="file-item" v-for="file in currentRequest.files" :key="file.id">
              <span class="file-icon">{{ getFileIcon(file.fileType) }}</span>
              <div class="file-info">
                <span class="file-name">{{ file.name }}</span>
                <span class="file-size">{{ formatFileSize(file.size) }}</span>
              </div>
            </div>
          </div>

          <div class="total-size" v-if="totalSize > 0">
            总计: {{ formatFileSize(totalSize) }}
          </div>

          <div class="save-options">
            <label>保存到:</label>
            <div class="path-input">
              <input type="text" v-model="savePath" readonly />
              <button class="btn-browse" @click="browsePath">浏览</button>
            </div>
          </div>
        </template>

        <div v-else class="no-requests">
          暂无文件请求
        </div>
      </div>

      <div class="dialog-footer" v-if="currentRequest">
        <button class="btn-decline" @click="handleDecline">拒绝</button>
        <button class="btn-accept" @click="handleAccept">接收</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useUIStore } from '../stores/uiStore'
import { useTransferStore } from '../stores/transferStore'
import { useDeviceStore } from '../stores/deviceStore'
import { formatFileSize } from '../utils/format'
import { DEVICE_TYPE } from '../utils/constants'

const uiStore = useUIStore()
const transferStore = useTransferStore()
const deviceStore = useDeviceStore()

const savePath = ref('')
const requests = ref([])
const selectedRequestIndex = ref(0)

const currentRequest = computed(() => {
  return requests.value[selectedRequestIndex.value] || null
})

const totalSize = computed(() => {
  if (!currentRequest.value) return 0
  return currentRequest.value.files.reduce((sum, f) => sum + (f.size || 0), 0)
})

const deviceIcon = computed(() => {
  if (!currentRequest.value) return '💻'
  const icons = {
    [DEVICE_TYPE.PHONE]: '📱',
    [DEVICE_TYPE.COMPUTER]: '💻',
    [DEVICE_TYPE.TABLET]: '📱'
  }
  return icons[currentRequest.value.senderType] || '💻'
})

function getFileIcon(fileType) {
  if (!fileType) return '📄'
  const lower = fileType.toLowerCase()
  if (lower.includes('image')) return '🖼️'
  if (lower.includes('video')) return '🎬'
  if (lower.includes('audio')) return '🎵'
  if (lower.includes('pdf')) return '📕'
  if (lower.includes('zip') || lower.includes('rar') || lower.includes('archive')) return '📦'
  return '📄'
}

function selectRequest(index) {
  selectedRequestIndex.value = index
}

async function handleAccept() {
  if (!currentRequest.value) return

  try {
    await transferStore.acceptReceive(currentRequest.value.sessionId, savePath.value)
    deviceStore.removeIncomingRequest(currentRequest.value.sessionId)
    closeIfNoMore()
  } catch (e) {
    console.error('接受失败:', e)
  }
}

async function handleDecline() {
  if (!currentRequest.value) return

  try {
    await transferStore.rejectReceive(currentRequest.value.sessionId)
    deviceStore.removeIncomingRequest(currentRequest.value.sessionId)
    closeIfNoMore()
  } catch (e) {
    console.error('拒绝失败:', e)
  }
}

function closeIfNoMore() {
  requests.value = deviceStore.incomingRequests
  if (requests.value.length === 0) {
    handleClose()
  } else {
    selectedRequestIndex.value = 0
  }
}

function handleClose() {
  uiStore.closeReceiveDialog()
}

async function browsePath() {
  // TODO: 使用 Tauri 的 open dialog
  console.log('浏览路径')
}

onMounted(async () => {
  requests.value = deviceStore.incomingRequests
  selectedRequestIndex.value = 0

  if (transferStore.downloadDir) {
    savePath.value = transferStore.downloadDir
  } else {
    await transferStore.loadDownloadDir()
    savePath.value = transferStore.downloadDir || '/tmp'
  }
})
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
  max-width: 520px;
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

.dialog-body {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
}

.requests-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.request-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #f5f5f5;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
}

.request-item:hover,
.request-item.active {
  background: #e8f5e9;
  border-color: #4CAF50;
}

.request-icon {
  font-size: 20px;
}

.request-name {
  flex: 1;
  font-size: 14px;
  color: #333;
}

.request-files {
  font-size: 12px;
  color: #888;
}

.sender-info {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: #e8f5e9;
  border-radius: 10px;
  margin-bottom: 20px;
}

.device-icon {
  font-size: 40px;
}

.sender-details {
  display: flex;
  flex-direction: column;
}

.sender-name {
  font-size: 16px;
  font-weight: 600;
  color: #2e7d32;
}

.sender-ip {
  font-size: 12px;
  color: #66bb6a;
}

.file-list {
  margin-bottom: 16px;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: #f5f5f5;
  border-radius: 8px;
  margin-bottom: 8px;
}

.file-icon {
  font-size: 24px;
}

.file-info {
  display: flex;
  flex-direction: column;
}

.file-name {
  font-size: 14px;
  color: #333;
  word-break: break-all;
}

.file-size {
  font-size: 12px;
  color: #888;
}

.total-size {
  text-align: right;
  font-size: 14px;
  color: #4CAF50;
  font-weight: 600;
  margin-bottom: 16px;
}

.save-options {
  margin-top: 16px;
}

.save-options label {
  display: block;
  font-size: 14px;
  color: #666;
  margin-bottom: 8px;
}

.path-input {
  display: flex;
  gap: 8px;
}

.path-input input {
  flex: 1;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  background: #f5f5f5;
}

.btn-browse {
  padding: 10px 16px;
  background: #f5f5f5;
  color: #333;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
}

.btn-browse:hover {
  background: #e0e0e0;
}

.no-requests {
  text-align: center;
  padding: 48px;
  color: #999;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 20px 24px;
  border-top: 1px solid #f0f0f0;
}

.btn-decline,
.btn-accept {
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-decline {
  background: #ffebee;
  color: #c62828;
  border: none;
}

.btn-decline:hover {
  background: #ffcdd2;
}

.btn-accept {
  background: #4CAF50;
  color: white;
  border: none;
}

.btn-accept:hover {
  background: #43A047;
}
</style>
