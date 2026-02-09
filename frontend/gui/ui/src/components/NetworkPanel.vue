<template>
  <div class="network-panel" :class="{ expanded: uiStore.networkPanelExpanded }">
    <div class="panel-header" @click="uiStore.toggleNetworkPanel">
      <span class="toggle-icon">{{ uiStore.networkPanelExpanded ? '▼' : '▶' }}</span>
      <span class="title">网络配置</span>
    </div>

    <div class="panel-content" v-if="uiStore.networkPanelExpanded">
      <div class="info-row">
        <span class="label">网络名称</span>
        <span class="value">{{ networkStore.networkName }}</span>
      </div>

      <div class="info-row">
        <span class="label">状态</span>
        <span class="status-badge" :class="networkStore.status">
          {{ statusText }}
        </span>
      </div>

      <div class="info-row">
        <span class="label">节点数量</span>
        <span class="value">{{ networkStore.peerCount }}</span>
      </div>

      <div class="info-row">
        <span class="label">本机 IP</span>
        <span class="value">{{ networkStore.myIp || '获取中...' }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useNetworkStore } from '../stores/networkStore'
import { useUIStore } from '../stores/uiStore'

const networkStore = useNetworkStore()
const uiStore = useUIStore()

const statusText = computed(() => {
  const texts = {
    disconnected: '已断开',
    connecting: '连接中...',
    connected: '已连接',
    error: '错误'
  }
  return texts[networkStore.status] || networkStore.status
})
</script>

<style scoped>
.network-panel {
  background: white;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 20px;
  background: #fafafa;
  cursor: pointer;
  user-select: none;
}

.panel-header:hover {
  background: #f0f0f0;
}

.toggle-icon {
  font-size: 10px;
  color: #666;
  width: 16px;
}

.title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.panel-content {
  padding: 16px 20px;
  border-top: 1px solid #f0f0f0;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
}

.info-row:not(:last-child) {
  border-bottom: 1px solid #f5f5f5;
}

.label {
  font-size: 13px;
  color: #666;
}

.value {
  font-size: 13px;
  color: #333;
  font-weight: 500;
}

.status-badge {
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.status-badge.connected {
  background: #e8f5e9;
  color: #2e7d32;
}

.status-badge.connecting {
  background: #fff3e0;
  color: #e65100;
}

.status-badge.disconnected {
  background: #ffebee;
  color: #c62828;
}

.status-badge.error {
  background: #ffebee;
  color: #c62828;
}
</style>
