<template>
  <div class="dashboard">
    <h1>总览仪表盘</h1>

    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-value">{{ store.stats.total }}</div>
        <div class="stat-label">设备总数</div>
      </div>
      <div class="stat-card online">
        <div class="stat-value">{{ store.stats.online }}</div>
        <div class="stat-label">在线设备</div>
      </div>
      <div class="stat-card channels">
        <div class="stat-value">{{ store.stats.channels }}</div>
        <div class="stat-label">通道总数</div>
      </div>
    </div>

    <div class="section">
      <h2>实时事件</h2>
      <div class="event-log">
        <div v-if="store.events.length === 0" class="no-events">暂无事件，等待设备注册...</div>
        <div v-for="(ev, i) in store.events" :key="i" class="event-item" :class="eventTypeClass(ev.type)">
          <span class="event-type">{{ ev.type }}</span>
          <span class="event-device">{{ ev.device_id || '' }}</span>
          <span class="event-detail">{{ formatEventDetail(ev) }}</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h2>快速操作</h2>
      <div class="actions">
        <button @click="generateSample" class="btn btn-primary" :disabled="generating">
          {{ generating ? '生成中...' : '一键生成 10 个设备' }}
        </button>
        <button @click="generateAndRegister" class="btn btn-success" :disabled="generating">
          {{ generating ? '生成中...' : '生成 10 个设备并自动注册' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useDeviceStore } from '../stores/devices'

const store = useDeviceStore()
const generating = ref(false)

onMounted(() => {
  store.fetchStats()
})

function eventTypeClass(type) {
  if (type === 'Registered') return 'event-success'
  if (type === 'RegisterFailed') return 'event-error'
  if (type === 'Keepalive') return 'event-info'
  return ''
}

function formatEventDetail(ev) {
  if (ev.reason) return ev.reason
  if (ev.call_id) return `call_id=${ev.call_id}`
  if (ev.sn) return `sn=${ev.sn}`
  if (ev.channel_id) return `channel=${ev.channel_id}`
  return ''
}

async function generateSample() {
  generating.value = true
  try {
    await store.generateDevices({ count: 10, channels_per_device: 4, auto_register: false })
    await store.fetchStats()
    await store.fetchDevices()
  } finally {
    generating.value = false
  }
}

async function generateAndRegister() {
  generating.value = true
  try {
    await store.generateDevices({ count: 10, channels_per_device: 4, auto_register: true })
    await store.fetchStats()
  } finally {
    generating.value = false
  }
}
</script>

<style scoped>
.dashboard h1 { margin-bottom: 24px; font-size: 24px; }
.stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-bottom: 32px; }
.stat-card {
  background: white;
  border-radius: 12px;
  padding: 24px;
  text-align: center;
  box-shadow: 0 2px 8px rgba(0,0,0,0.06);
  border-left: 4px solid #667eea;
}
.stat-card.online { border-left-color: #52c41a; }
.stat-card.channels { border-left-color: #faad14; }
.stat-value { font-size: 36px; font-weight: 700; color: #1a1a2e; }
.stat-label { font-size: 14px; color: #666; margin-top: 4px; }
.section { margin-bottom: 32px; }
.section h2 { font-size: 18px; margin-bottom: 16px; }
.event-log {
  background: #1a1a2e;
  border-radius: 8px;
  padding: 16px;
  max-height: 300px;
  overflow-y: auto;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 13px;
}
.no-events { color: #666; padding: 16px 0; text-align: center; }
.event-item {
  padding: 4px 0;
  color: #aaa;
  display: flex;
  gap: 12px;
  border-bottom: 1px solid rgba(255,255,255,0.05);
}
.event-type { color: #667eea; min-width: 140px; }
.event-device { color: #52c41a; min-width: 220px; }
.event-detail { color: #888; flex: 1; }
.event-success .event-type { color: #52c41a; }
.event-error .event-type { color: #ff4d4f; }
.event-info .event-type { color: #1890ff; }
.actions { display: flex; gap: 12px; }
.btn {
  padding: 10px 24px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}
.btn:disabled { opacity: 0.6; cursor: not-allowed; }
.btn-primary { background: #667eea; color: white; }
.btn-primary:hover:not(:disabled) { background: #5a6fd6; }
.btn-success { background: #52c41a; color: white; }
.btn-success:hover:not(:disabled) { background: #49b018; }
</style>
