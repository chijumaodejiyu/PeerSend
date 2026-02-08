import { invoke } from '@tauri-apps/api/core'

export async function getVersion() {
  return await invoke('get_version')
}

export async function getStatus() {
  return await invoke('get_status')
}

export async function startDaemon(config) {
  return await invoke('start_daemon', {
    network_name: config.networkName,
    network_secret: config.networkSecret,
    peers: config.peers
  })
}

export async function stopDaemon() {
  return await invoke('stop_daemon')
}

export async function discoverPeers() {
  return await invoke('discover_peers')
}
