import { invoke } from '@tauri-apps/api/core'

export async function sendFiles(paths, peerId) {
  return await invoke('send_files', { paths, peer_id: peerId })
}

export async function getTransfers() {
  return await invoke('get_transfers')
}

export async function cancelTransfer(id) {
  return await invoke('cancel_transfer', { id })
}

export async function acceptTransfer(id, path) {
  return await invoke('accept_transfer', { id, path })
}
