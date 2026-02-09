<template>
  <div class="main-view">
    <header class="header">
      <div class="header-left">
        <svg viewBox="0 0 100 100" width="32" height="32">
          <circle cx="50" cy="50" r="45" fill="#4CAF50"/>
          <path d="M30 50 L45 65 L70 35" stroke="white" stroke-width="8" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <h1>PeerSend</h1>
      </div>
      <div class="header-right">
        <span class="network-badge" v-if="networkStore.isConnected">
          {{ networkStore.networkName }}
        </span>
        <button class="btn-disconnect" @click="handleDisconnect">
          断开连接
        </button>
      </div>
    </header>

    <div class="content">
      <NetworkPanel />

      <div class="divider"></div>

      <div class="discovery-section">
        <div class="section-header">
          <h2>LocalSend 设备</h2>
          <div class="header-actions">
            <button class="btn-refresh" @click="handleDiscovery" :disabled="deviceStore.discovering">
              {{ deviceStore.discovering ? '扫描中...' : '刷新' }}
            </button>
          </div>
        </div>

        <DeviceList />

        <div v-if="deviceStore.devices.length === 0 && !deviceStore.discovering" class="empty-state">
          <p>点击刷新按钮扫描局域网中的 LocalSend 设备</p>
        </div>
      </div>

      <!-- 传入请求通知 -->
      <div class="incoming-notice" v-if="deviceStore.hasIncomingRequests && !uiStore.showReceiveDialog" @click="showIncomingRequests">
        <span class="notice-icon">📥</span>
        <span class="notice-text">收到 {{ deviceStore.incomingRequests.length }} 个文件传输请求</span>
        <span class="notice-arrow">→</span>
      </div>
    </div>

    <ReceiveDialog v-if="uiStore.showReceiveDialog" />
    <SendDialog v-if="uiStore.showSendDialog" />
    <PinDialog v-if="uiStore.showPinDialog" />
  </div>
</template>

<script setup>
import { onMounted, onUnmounted } from 'vue'
import { useNetworkStore } from '../stores/networkStore'
import { useDeviceStore } from '../stores/deviceStore'
import { useUIStore } from '../stores/uiStore'
import NetworkPanel from '../components/NetworkPanel.vue'
import DeviceList from '../components/DeviceList.vue'
import ReceiveDialog from '../components/ReceiveDialog.vue'
import SendDialog from '../components/SendDialog.vue'
import PinDialog from '../components/PinDialog.vue'

const networkStore = useNetworkStore()
const deviceStore = useDeviceStore()
const uiStore = useUIStore()

let refreshInterval = null

onMounted(() => {
  handleDiscovery()
  // 定期刷新设备列表
  refreshInterval = setInterval(() => {
    if (networkStore.isConnected) {
      deviceStore.startDiscovery()
    }
  }, 30000)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})

async function handleDiscovery() {
  await deviceStore.startDiscovery()
}

async function handleDisconnect() {
  await networkStore.disconnect()
}

function showIncomingRequests() {
  uiStore.openReceiveDialog()
}
</script>

<style scoped>
.main-view {
  min-height: 100vh;
  background: #f5f5f5;
}

.header {
  background: white;
  padding: 16px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-left h1 {
  font-size: 20px;
  font-weight: 600;
  color: #333;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.network-badge {
  background: #e8f5e9;
  color: #2e7d32;
  padding: 6px 12px;
  border-radius: 16px;
  font-size: 13px;
  font-weight: 500;
}

.btn-disconnect {
  padding: 8px 16px;
  background: #ffebee;
  color: #c62828;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-disconnect:hover {
  background: #ffcdd2;
}

.content {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
}

.divider {
  height: 1px;
  background: #e0e0e0;
  margin: 24px 0;
}

.discovery-section {
  background: white;
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.section-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: #333;
}

.header-actions {
  display: flex;
  gap: 12px;
}

.btn-refresh {
  padding: 8px 16px;
  background: #4CAF50;
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-refresh:hover:not(:disabled) {
  background: #43A047;
}

.btn-refresh:disabled {
  background: #a5d6a7;
  cursor: not-allowed;
}

.empty-state {
  text-align: center;
  padding: 48px 24px;
  color: #999;
}

.incoming-notice {
  position: fixed;
  bottom: 24px;
  right: 24px;
  background: #4CAF50;
  color: white;
  padding: 16px 24px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(76, 175, 80, 0.4);
  transition: transform 0.2s, box-shadow 0.2s;
  z-index: 100;
}

.incoming-notice:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(76, 175, 80, 0.5);
}

.notice-icon {
  font-size: 24px;
}

.notice-text {
  font-size: 14px;
  font-weight: 500;
}

.notice-arrow {
  font-size: 18px;
  opacity: 0.8;
}
</style>
