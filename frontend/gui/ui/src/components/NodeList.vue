<template>
  <div class="node-list">
    <div class="node-item" v-for="node in nodes" :key="node.id">
      <div class="node-icon">
        <span>{{ nodeIcon }}</span>
      </div>
      <div class="node-info">
        <span class="node-id">{{ node.id }}</span>
        <span class="node-addr">{{ node.address }}</span>
      </div>
      <div class="node-status" :class="node.status">
        {{ statusText }}
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  nodes: {
    type: Array,
    default: () => []
  }
})

const nodeIcon = computed(() => '🖥️')

const statusText = computed(() => {
  return '在线'
})
</script>

<style scoped>
.node-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.node-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: #f5f5f5;
  border-radius: 8px;
}

.node-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border-radius: 8px;
  font-size: 18px;
}

.node-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.node-id {
  font-size: 13px;
  color: #333;
  font-weight: 500;
}

.node-addr {
  font-size: 11px;
  color: #888;
}

.node-status {
  padding: 4px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
}

.node-status.online {
  background: #e8f5e9;
  color: #2e7d32;
}
</style>
