import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useNetworkStore = defineStore('network', () => {
  const status = ref('disconnected')
  const networkName = ref('')
  const peerCount = ref(0)
  const myIp = ref('')
  const error = ref(null)
  const loading = ref(false)

  const isConnected = computed(() => status.value === 'connected')

  async function checkStatus() {
    try {
      const daemonStatus = await invoke('get_status')
      if (daemonStatus.running) {
        status.value = 'connected'
        networkName.value = daemonStatus.network_name
        peerCount.value = daemonStatus.peer_count
      } else {
        status.value = 'disconnected'
      }
    } catch (e) {
      console.error('检查状态失败:', e)
      status.value = 'disconnected'
    }
  }

  async function connect(config) {
    loading.value = true
    error.value = null

    try {
      await invoke('start_daemon', {
        network_name: config.networkName,
        network_secret: config.networkSecret,
        peers: config.peers
      })

      status.value = 'connected'
      networkName.value = config.networkName
    } catch (e) {
      error.value = e.toString()
      status.value = 'error'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function disconnect() {
    loading.value = true
    try {
      await invoke('stop_daemon')
      status.value = 'disconnected'
      networkName.value = ''
      peerCount.value = 0
      myIp.value = ''
    } catch (e) {
      console.error('断开连接失败:', e)
    } finally {
      loading.value = false
    }
  }

  return {
    status,
    networkName,
    peerCount,
    myIp,
    error,
    loading,
    isConnected,
    checkStatus,
    connect,
    disconnect
  }
})
