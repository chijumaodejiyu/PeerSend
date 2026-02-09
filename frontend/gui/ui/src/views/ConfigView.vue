<template>
  <div class="config-view">
    <div class="config-container">
      <div class="logo">
        <svg viewBox="0 0 100 100" width="80" height="80">
          <circle cx="50" cy="50" r="45" fill="#4CAF50"/>
          <path d="M30 50 L45 65 L70 35" stroke="white" stroke-width="8" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <h1>PeerSend</h1>
        <p class="subtitle">P2P 文件传输</p>
      </div>

      <div class="config-form">
        <h2>加入或创建网络</h2>

        <div class="form-group">
          <label for="networkName">网络名称</label>
          <input
            id="networkName"
            v-model="form.networkName"
            type="text"
            placeholder="输入网络名称"
            :disabled="loading"
          />
        </div>

        <div class="form-group">
          <label for="networkSecret">网络密钥 (可选)</label>
          <input
            id="networkSecret"
            v-model="form.networkSecret"
            type="password"
            placeholder="留空表示公开网络"
            :disabled="loading"
          />
        </div>

        <div class="form-group">
          <label for="peers">节点地址 (可选)</label>
          <input
            id="peers"
            v-model="form.peersInput"
            type="text"
            placeholder="例如: 192.168.1.100:11011"
            :disabled="loading"
          />
          <span class="hint">多个地址用逗号分隔</span>
        </div>

        <div class="form-group">
          <label>
            <input type="checkbox" v-model="form.useDhcp" :disabled="loading" />
            自动获取 IP 地址 (DHCP)
          </label>
        </div>

        <div class="form-group" v-if="!form.useDhcp">
          <label for="ipv4">静态 IP 地址</label>
          <input
            id="ipv4"
            v-model="form.ipv4"
            type="text"
            placeholder="例如: 10.0.0.5"
            :disabled="loading"
          />
        </div>

        <div class="error-message" v-if="error">
          {{ error }}
        </div>

        <div class="buttons">
          <button class="btn-primary" @click="handleConnect" :disabled="loading">
            <span v-if="loading">连接中...</span>
            <span v-else>加入网络</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { useNetworkStore } from '../stores/networkStore'

const networkStore = useNetworkStore()

const loading = ref(false)
const error = ref(null)

const form = reactive({
  networkName: '',
  networkSecret: '',
  peersInput: '',
  useDhcp: true,
  ipv4: ''
})

async function handleConnect() {
  if (!form.networkName.trim()) {
    error.value = '请输入网络名称'
    return
  }

  loading.value = true
  error.value = null

  try {
    const peers = form.peersInput
      .split(',')
      .map(p => p.trim())
      .filter(p => p.length > 0)

    await networkStore.connect({
      networkName: form.networkName.trim(),
      networkSecret: form.networkSecret || null,
      peers,
      dhcp: form.useDhcp,
      ipv4: form.useDhcp ? null : form.ipv4
    })
  } catch (e) {
    error.value = e.message || '连接失败'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.config-view {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 20px;
}

.config-container {
  background: white;
  border-radius: 16px;
  padding: 40px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  width: 100%;
  max-width: 450px;
}

.logo {
  text-align: center;
  margin-bottom: 32px;
}

.logo h1 {
  font-size: 28px;
  color: #333;
  margin-top: 16px;
}

.subtitle {
  color: #666;
  font-size: 14px;
}

.config-form h2 {
  font-size: 18px;
  color: #333;
  margin-bottom: 24px;
  text-align: center;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  font-size: 14px;
  color: #555;
  margin-bottom: 8px;
  font-weight: 500;
}

.form-group input[type="text"],
.form-group input[type="password"] {
  width: 100%;
  padding: 12px 16px;
  border: 2px solid #e0e0e0;
  border-radius: 8px;
  font-size: 14px;
  transition: border-color 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: #4CAF50;
}

.form-group input:disabled {
  background: #f5f5f5;
  cursor: not-allowed;
}

.hint {
  font-size: 12px;
  color: #999;
  margin-top: 4px;
  display: block;
}

.error-message {
  background: #ffebee;
  color: #c62828;
  padding: 12px;
  border-radius: 8px;
  margin-bottom: 20px;
  font-size: 14px;
}

.buttons {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}

.btn-primary {
  flex: 1;
  padding: 14px 24px;
  background: #4CAF50;
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary:hover:not(:disabled) {
  background: #43A047;
}

.btn-primary:disabled {
  background: #a5d6a7;
  cursor: not-allowed;
}
</style>
