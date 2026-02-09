import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUIStore = defineStore('ui', () => {
  const networkPanelExpanded = ref(true)
  const selectedDeviceId = ref(null)
  const showSendDialog = ref(false)
  const showReceiveDialog = ref(false)
  const showPinDialog = ref(false)
  const pinCode = ref('')

  function toggleNetworkPanel() {
    networkPanelExpanded.value = !networkPanelExpanded.value
  }

  function selectDevice(deviceId) {
    selectedDeviceId.value = deviceId
    showSendDialog.value = true
  }

  function closeSendDialog() {
    showSendDialog.value = false
    selectedDeviceId.value = null
  }

  function openReceiveDialog() {
    showReceiveDialog.value = true
  }

  function closeReceiveDialog() {
    showReceiveDialog.value = false
  }

  function openPinDialog(code) {
    pinCode.value = code
    showPinDialog.value = true
  }

  function closePinDialog() {
    showPinDialog.value = false
    pinCode.value = ''
  }

  return {
    networkPanelExpanded,
    selectedDeviceId,
    showSendDialog,
    showReceiveDialog,
    showPinDialog,
    pinCode,
    toggleNetworkPanel,
    selectDevice,
    closeSendDialog,
    openReceiveDialog,
    closeReceiveDialog,
    openPinDialog,
    closePinDialog
  }
})
