<template>
  <div class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h3>输入 PIN 码</h3>
        <button class="btn-close" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <p class="pin-hint">请在对方设备上输入以下 PIN 码:</p>

        <div class="pin-display">
          <span v-for="(digit, index) in pinDigits" :key="index" class="pin-digit">
            {{ digit }}
          </span>
        </div>

        <div class="pin-input-section">
          <p class="input-hint">或者在此输入对方显示的 PIN 码:</p>
          <input
            type="text"
            v-model="userPin"
            placeholder="输入 PIN 码"
            maxlength="6"
            class="pin-input"
          />
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn-cancel" @click="handleClose">取消</button>
        <button class="btn-confirm" @click="handleConfirm" :disabled="userPin.length === 0">
          确认
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useUIStore } from '../stores/uiStore'

const uiStore = useUIStore()

const userPin = ref('')

const pinDigits = computed(() => {
  const pin = uiStore.pinCode || '000000'
  return pin.padEnd(6, '0').split('')
})

function handleConfirm() {
  if (userPin.value.length > 0) {
    // 确认 PIN 码
    console.log('确认 PIN:', userPin.value)
  }
  handleClose()
}

function handleClose() {
  uiStore.closePinDialog()
  userPin.value = ''
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.dialog {
  background: white;
  border-radius: 16px;
  width: 100%;
  max-width: 400px;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid #f0f0f0;
}

.dialog-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: #333;
}

.btn-close {
  width: 32px;
  height: 32px;
  border: none;
  background: #f5f5f5;
  color: #666;
  border-radius: 50%;
  font-size: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.dialog-body {
  padding: 32px 24px;
}

.pin-hint {
  text-align: center;
  color: #666;
  margin-bottom: 24px;
}

.pin-display {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-bottom: 32px;
}

.pin-digit {
  width: 48px;
  height: 64px;
  background: #4CAF50;
  color: white;
  font-size: 32px;
  font-weight: bold;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(76, 175, 80, 0.3);
}

.input-hint {
  text-align: center;
  color: #999;
  font-size: 13px;
  margin-bottom: 12px;
}

.pin-input {
  width: 100%;
  padding: 16px;
  border: 2px solid #e0e0e0;
  border-radius: 10px;
  font-size: 24px;
  text-align: center;
  letter-spacing: 8px;
  font-weight: bold;
}

.pin-input:focus {
  outline: none;
  border-color: #4CAF50;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 20px 24px;
  border-top: 1px solid #f0f0f0;
}

.btn-cancel,
.btn-confirm {
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel {
  background: white;
  color: #666;
  border: 1px solid #ddd;
}

.btn-cancel:hover {
  background: #f5f5f5;
}

.btn-confirm {
  background: #4CAF50;
  color: white;
  border: none;
}

.btn-confirm:hover:not(:disabled) {
  background: #43A047;
}

.btn-confirm:disabled {
  background: #a5d6a7;
  cursor: not-allowed;
}
</style>
