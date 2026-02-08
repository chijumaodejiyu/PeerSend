<template>
  <div class="app">
    <ConfigView v-if="networkStore.status === 'disconnected'" />
    <MainView v-else />
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import { useNetworkStore } from './stores/networkStore'
import ConfigView from './views/ConfigView.vue'
import MainView from './views/MainView.vue'

const networkStore = useNetworkStore()

onMounted(async () => {
  await networkStore.checkStatus()
})
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background-color: #f5f5f5;
  color: #333;
  line-height: 1.6;
}

.app {
  min-height: 100vh;
}
</style>
