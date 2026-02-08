<template>
  <div class="progress-bar">
    <div class="progress-info">
      <span class="progress-filename">{{ filename }}</span>
      <span class="progress-percentage">{{ percentage }}%</span>
    </div>
    <div class="progress-track">
      <div class="progress-fill" :style="{ width: percentage + '%' }"></div>
    </div>
    <div class="progress-stats">
      <span class="progress-speed">{{ formattedSpeed }}</span>
      <span class="progress-time">{{ formattedTime }}</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { formatFileSize, formatSpeed, formatTime } from '../utils/format'

const props = defineProps({
  filename: {
    type: String,
    default: ''
  },
  progress: {
    type: Number,
    default: 0
  },
  speed: {
    type: Number,
    default: 0
  },
  remaining: {
    type: Number,
    default: 0
  }
})

const percentage = computed(() => Math.round(props.progress * 100))

const formattedSpeed = computed(() => formatSpeed(props.speed))

const formattedTime = computed(() => formatTime(props.remaining))
</script>

<style scoped>
.progress-bar {
  padding: 12px 0;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
}

.progress-filename {
  font-size: 13px;
  color: #333;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-right: 12px;
}

.progress-percentage {
  font-size: 13px;
  font-weight: 600;
  color: #4CAF50;
}

.progress-track {
  height: 8px;
  background: #e0e0e0;
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4CAF50, #81C784);
  border-radius: 4px;
  transition: width 0.3s ease;
}

.progress-stats {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 12px;
  color: #888;
}
</style>
