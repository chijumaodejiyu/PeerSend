<template>
  <div class="file-selector">
    <input
      type="file"
      ref="fileInput"
      multiple
      @change="handleFileSelect"
      style="display: none"
    />

    <div class="dropzone" @click="triggerFileInput" @drop.prevent="handleDrop" @dragover.prevent="isDragging = true" @dragleave="isDragging = false" :class="{ dragging: isDragging }">
      <div class="dropzone-content">
        <div class="upload-icon">📤</div>
        <p class="dropzone-text">点击或拖拽文件到这里</p>
        <p class="dropzone-hint">支持多个文件</p>
      </div>
    </div>

    <div class="file-list" v-if="files.length > 0">
      <div class="file-item" v-for="(file, index) in files" :key="index">
        <span class="file-icon">📄</span>
        <div class="file-info">
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size">{{ formatFileSize(file.size) }}</span>
        </div>
        <button class="btn-remove" @click="removeFile(index)">×</button>
      </div>
    </div>

    <div class="actions" v-if="files.length > 0">
      <button class="btn-add-more" @click="triggerFileInput">+ 添加更多</button>
      <button class="btn-clear" @click="clearFiles">清空</button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { formatFileSize } from '../utils/format'

const props = defineProps({
  modelValue: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['update:modelValue'])

const fileInput = ref(null)
const files = ref([...props.modelValue])
const isDragging = ref(false)

function triggerFileInput() {
  fileInput.value.click()
}

function handleFileSelect(event) {
  const selected = Array.from(event.target.files)
  addFiles(selected)
}

function handleDrop(event) {
  isDragging.value = false
  const dropped = Array.from(event.dataTransfer.files)
  addFiles(dropped)
}

function addFiles(newFiles) {
  const updated = [...files.value, ...newFiles]
  files.value = updated
  emit('update:modelValue', updated)
}

function removeFile(index) {
  const updated = files.value.filter((_, i) => i !== index)
  files.value = updated
  emit('update:modelValue', updated)
}

function clearFiles() {
  files.value = []
  emit('update:modelValue', [])
}
</script>

<style scoped>
.file-selector {
  margin-top: 16px;
}

.dropzone {
  border: 2px dashed #ddd;
  border-radius: 10px;
  padding: 40px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
}

.dropzone:hover,
.dropzone.dragging {
  border-color: #4CAF50;
  background: #f5fdf5;
}

.dropzone-content {
  pointer-events: none;
}

.upload-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.dropzone-text {
  font-size: 15px;
  color: #333;
  margin-bottom: 4px;
}

.dropzone-hint {
  font-size: 13px;
  color: #999;
}

.file-list {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #f5f5f5;
  border-radius: 8px;
}

.file-icon {
  font-size: 20px;
}

.file-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.file-name {
  font-size: 14px;
  color: #333;
  word-break: break-all;
}

.file-size {
  font-size: 12px;
  color: #888;
}

.btn-remove {
  width: 28px;
  height: 28px;
  border: none;
  background: #ffebee;
  color: #c62828;
  border-radius: 50%;
  font-size: 18px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.btn-remove:hover {
  background: #ffcdd2;
}

.actions {
  margin-top: 16px;
  display: flex;
  gap: 12px;
}

.btn-add-more,
.btn-clear {
  padding: 10px 20px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-add-more {
  flex: 1;
  background: #4CAF50;
  color: white;
  border: none;
}

.btn-add-more:hover {
  background: #43A047;
}

.btn-clear {
  background: white;
  color: #666;
  border: 1px solid #ddd;
}

.btn-clear:hover {
  background: #f5f5f5;
}
</style>
