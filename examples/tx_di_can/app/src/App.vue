<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { api, onCanEvent } from './api/can.ts'
import { state, pushLog, handleEvent } from './store.ts'
import TraceView from './views/TraceView.vue'
import UdsView from './views/UdsView.vue'
import FlashView from './views/FlashView.vue'
import ConfigView from './views/ConfigView.vue'
import SimEcuView from './views/SimEcuView.vue'
import RecordReplayView from './views/RecordReplayView.vue'
import DbcView from './views/DbcView.vue'

const activeTab = ref('trace')
const tabs = [
  { key: 'trace', label: '总线监控' },
  { key: 'uds', label: 'UDS 诊断' },
  { key: 'flash', label: '固件刷写' },
  { key: 'simecu', label: 'ECU 仿真' },
  { key: 'record', label: '录制回放' },
  { key: 'dbc', label: 'DBC 解码' },
  { key: 'config', label: '连接配置' },
]

let unlisten: (() => void) | null = null

onMounted(async () => {
  try {
    unlisten = await onCanEvent(handleEvent)
    state.config = await api.getConfig()
    state.connected = await api.isConnected()
    pushLog('已连接事件总线')
  } catch (e) {
    pushLog('初始化失败: ' + String(e))
  }
})
onUnmounted(() => unlisten?.())
</script>

<template>
  <div class="app">
    <header class="topbar">
      <span class="title">CAN 诊断上位机</span>
      <span class="status" :class="{ on: state.connected }">
        {{ state.connected ? '已连接' : '未连接' }}
      </span>
    </header>
    <nav class="tabs">
      <button
        v-for="t in tabs"
        :key="t.key"
        :class="{ active: activeTab === t.key }"
        @click="activeTab = t.key"
      >
        {{ t.label }}
      </button>
    </nav>
    <main class="content">
      <TraceView v-if="activeTab === 'trace'" />
      <UdsView v-else-if="activeTab === 'uds'" />
      <FlashView v-else-if="activeTab === 'flash'" />
      <SimEcuView v-else-if="activeTab === 'simecu'" />
      <RecordReplayView v-else-if="activeTab === 'record'" />
      <DbcView v-else-if="activeTab === 'dbc'" />
      <ConfigView v-else-if="activeTab === 'config'" />
    </main>
  </div>
</template>
