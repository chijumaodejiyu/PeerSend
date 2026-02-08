<template>
  <div class="device-card" @click="handleClick">
    <div class="device-icon">
      <span class="icon">{{ deviceIcon }}</span>
    </div>
    <div class="device-info">
      <div class="device-name">{{ device.name }}</div>
      <div class="device-meta">
        <span class="device-ip">{{ device.ip }}</span>
        <span class="separator">|</span>
        <span class="device-version">{{ device.version }}</span>
      </div>
    </div>
    <div class="device-actions">
      <button class="btn-select" @click.stop="handleSelect">
        选择文件
      </button>
      <button class="btn-send" @click.stop="handleSend" v-if="hasSelection">
        发送
      </button>
    </div>
    <div class="device-status" :class="device.status">
      {{ statusText }}
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useUIStore } from '../stores/uiStore'
import { DEVICE_TYPE } from '../utils/constants'

const props = defineProps({
  device: {
    type: Object,
    required: true
  }
})

const uiStore = useUIStore()

const deviceIcon = computed(() => {
  const icons = {
    [DEVICE_TYPE.PHONE]: '📱',
    [DEVICE_TYPE.COMPUTER]: '💻',
    [DEVICE_TYPE.TABLET]: '📱',
    [DEVICE_TYPE.UNKNOWN]: '🖥️'
  }
  return icons[props.device.type] || icons[DEVICE_TYPE.UNKNOWN]
})

const statusText = computed(() => {
  const texts = {
    online: '在线',
    offline: '离线',
    busy: '忙碌'
  }
  return texts[props.device.status] || props.device.status
})

const hasSelection = computed(() => {
  return props.device.selectedFiles && props.device.selectedFiles.length > 0
})

function handleClick() {
  // 可以展开显示更多设备详情
}

function handleSelect() {
  uiStore.selectDevice(props.device.id)
}

function handleSend() {
  // 发送文件到该设备
}
</script>

<style scoped>
.device-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 20px;
  background: #fafafa;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
}

.device-card:hover {
  background: #f0f0f0;
  border-color: #4CAF50;
}

.device-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border-radius: 10px;
  font-size: 24px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.device-info {
  flex: 1;
  min-width: 0;
}

.device-name {
  font-size: 15px;
  font-weight: 600;
  color: #333;
  margin-bottom: 4px;
}

.device-meta {
  font-size: 12px;
  color: #888;
  display: flex;
  align-items: center;
  gap: 8px;
}

.separator {
  color: #ddd;
}

.device-actions {
  display: flex;
  gap: 8px;
}

.btn-select,
.btn-send {
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-select {
  background: white;
  color: #333;
  border: 1px solid #ddd;
}

.btn-select:hover {
  background: #f5f5f5;
}

.btn-send {
  background: #4CAF50;
  color: white;
  border: none;
}

.btn-send:hover {
  background: #43A047;
}

.device-status {
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.device-status.online {
  background: #e8f5e9;
  color: #2e7d32;
}

.device-status.offline {
  background: #ffebee;
  color: #c62828;
}

.device-status.busy {
  background: #fff3e0;
  color: #e65100;
}
</style>
