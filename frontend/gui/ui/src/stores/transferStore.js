import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useTransferStore = defineStore('transfer', () => {
  const sending = ref([])
  const receiving = ref([])
  const completed = ref([])
  const downloadDir = ref('')

  async function sendFiles(paths, deviceId) {
    try {
      await invoke('send_files', { paths, peer_id: deviceId })
      await refreshTransfers()
    } catch (e) {
      console.error('发送文件失败:', e)
      throw e
    }
  }

  async function acceptReceive(sessionId, path) {
    try {
      await invoke('accept_transfer', { id: sessionId, path })
      await refreshTransfers()
    } catch (e) {
      console.error('接受传输失败:', e)
      throw e
    }
  }

  async function rejectReceive(sessionId) {
    try {
      await invoke('reject_file_request', { session_id: sessionId })
      await refreshTransfers()
    } catch (e) {
      console.error('拒绝传输失败:', e)
      throw e
    }
  }

  async function cancelTransfer(id) {
    try {
      await invoke('cancel_transfer', { id })
      await refreshTransfers()
    } catch (e) {
      console.error('取消传输失败:', e)
      throw e
    }
  }

  async function refreshTransfers() {
    try {
      const transfers = await invoke('get_transfers')
      sending.value = transfers.filter(t => t.type === 'send')
      receiving.value = transfers.filter(t => t.type === 'receive')
      completed.value = transfers.filter(t => t.state === 'completed' || t.state === 'cancelled' || t.state === 'rejected')
    } catch (e) {
      console.error('刷新传输列表失败:', e)
    }
  }

  async function loadIncomingRequests() {
    try {
      const requests = await invoke('get_file_requests')
      return requests
    } catch (e) {
      console.error('加载传入请求失败:', e)
      return []
    }
  }

  async function loadDownloadDir() {
    try {
      downloadDir.value = await invoke('get_download_dir')
    } catch (e) {
      console.error('加载下载目录失败:', e)
      const home = process.env.HOME || '/tmp'
      downloadDir.value = `${home}/Downloads/PeerSend`
    }
  }

  async function setDownloadDir(path) {
    try {
      await invoke('set_download_dir', { path })
      downloadDir.value = path
    } catch (e) {
      console.error('设置下载目录失败:', e)
    }
  }

  async function getListenerPort() {
    try {
      return await invoke('get_listener_port')
    } catch (e) {
      console.error('获取监听端口失败:', e)
      return 53317
    }
  }

  return {
    sending,
    receiving,
    completed,
    downloadDir,
    sendFiles,
    acceptReceive,
    rejectReceive,
    cancelTransfer,
    refreshTransfers,
    loadIncomingRequests,
    loadDownloadDir,
    setDownloadDir,
    getListenerPort
  }
})
