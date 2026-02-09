<template>
  <div class="device-list">
    <DeviceCard
      v-for="device in deviceStore.devices"
      :key="device.id"
      :device="device"
    />

    <div v-if="deviceStore.discovering" class="scanning-indicator">
      <div class="spinner"></div>
      <span>正在扫描局域网...</span>
    </div>
  </div>
</template>

<script setup>
import { useDeviceStore } from '../stores/deviceStore'
import DeviceCard from './DeviceCard.vue'

const deviceStore = useDeviceStore()
</script>

<style scoped>
.device-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.scanning-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 32px;
  color: #666;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 3px solid #e0e0e0;
  border-top-color: #4CAF50;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
