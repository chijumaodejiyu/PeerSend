import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useDeviceStore = defineStore('device', () => {
  const devices = ref([])
  const discovering = ref(false)
  const incomingRequest = ref(null)
  const incomingRequests = ref([])
  const selectedDevice = ref(null)

  async function startDiscovery() {
    discovering.value = true
    try {
      const found = await invoke('discover_peers')
      devices.value = found.map(d => ({
        ...d,
        status: d.status || 'offline'
      }))
    } catch (e) {
      console.error('发现设备失败:', e)
    } finally {
      discovering.value = false
    }
  }

  function addDevice(device) {
    const exists = devices.value.find(d => d.id === device.id)
    if (!exists) {
      devices.value.push({
        ...device,
        status: device.status || 'online'
      })
    }
  }

  function removeDevice(id) {
    devices.value = devices.value.filter(d => d.id !== id)
  }

  function setIncomingRequest(request) {
    incomingRequest.value = request
    if (request && !incomingRequests.value.find(r => r.sessionId === request.sessionId)) {
      incomingRequests.value.push(request)
    }
  }

  function clearIncomingRequest() {
    incomingRequest.value = null
  }

  function removeIncomingRequest(sessionId) {
    incomingRequests.value = incomingRequests.value.filter(r => r.sessionId !== sessionId)
    if (incomingRequest.value?.sessionId === sessionId) {
      incomingRequest.value = null
    }
  }

  function selectDevice(device) {
    selectedDevice.value = device
  }

  function clearSelection() {
    selectedDevice.value = null
  }

  const hasIncomingRequests = computed(() => incomingRequests.value.length > 0)

  return {
    devices,
    discovering,
    incomingRequest,
    incomingRequests,
    selectedDevice,
    hasIncomingRequests,
    startDiscovery,
    addDevice,
    removeDevice,
    setIncomingRequest,
    clearIncomingRequest,
    removeIncomingRequest,
    selectDevice,
    clearSelection
  }
})
