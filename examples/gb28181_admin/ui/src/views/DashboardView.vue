<template>
  <div>
    <div class="page-header">
      <h1>概览</h1>
      <p>GB28181 平台运行状态一览</p>
    </div>

    <!-- 统计卡片 -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-label">全部设备</div>
        <div class="stat-value">{{ store.stats.total }}</div>
      </div>
      <div class="stat-card stat-card--green">
        <div class="stat-label">在线设备</div>
        <div class="stat-value">{{ store.stats.online }}</div>
      </div>
      <div class="stat-card stat-card--blue">
        <div class="stat-label">活跃会话</div>
        <div class="stat-value">{{ store.stats.sessions }}</div>
      </div>
      <div class="stat-card stat-card--amber">
        <div class="stat-label">最新事件</div>
        <div class="stat-value">{{ store.events.length }}</div>
      </div>
    </div>

    <!-- 最近事件 + 在线设备 -->
    <div class="dashboard-grid mt-24">
      <!-- 在线设备 -->
      <div class="card">
        <div class="flex items-center justify-between" style="margin-bottom:14px">
          <h2 class="section-title">在线设备</h2>
          <RouterLink to="/devices" class="btn btn-sm">全部</RouterLink>
        </div>
        <div v-if="onlineDevices.length === 0" class="empty-state">
          <div class="icon">📷</div>
          <p>暂无在线设备</p>
        </div>
        <div v-else class="device-mini-list">
          <div v-for="d in onlineDevices.slice(0,8)" :key="d.device_id" class="device-mini-item">
            <span class="dot dot-green" style="margin-right:8px;flex-shrink:0"></span>
            <div class="truncate">
              <RouterLink :to="`/devices/${d.device_id}`" class="device-mini-id">
                {{ d.device_id }}
              </RouterLink>
              <div class="device-mini-meta">{{ d.manufacturer || '未知厂商' }} · {{ d.channel_count }} 通道</div>
            </div>
            <span style="margin-left:auto;color:var(--text-muted);font-size:12px;flex-shrink:0">{{ d.remote_addr }}</span>
          </div>
        </div>
      </div>

      <!-- 最近事件 -->
      <div class="card">
        <div class="flex items-center justify-between" style="margin-bottom:14px">
          <h2 class="section-title">实时事件</h2>
          <RouterLink to="/events" class="btn btn-sm">全部</RouterLink>
        </div>
        <div v-if="store.events.length === 0" class="empty-state">
          <div class="icon">🔔</div>
          <p>暂无事件</p>
        </div>
        <div v-else class="event-mini-list">
          <div v-for="(ev, i) in store.events.slice(0, 10)" :key="i" class="event-mini-item">
            <span :class="['event-dot', eventColor(ev.type)]"></span>
            <div class="truncate">
              <span class="event-type">{{ ev.type }}</span>
              <span class="event-id">{{ ev.device_id }}</span>
            </div>
            <span class="event-time">{{ ev._time }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { useGb28181Store } from '../stores/gb28181.js'

const store = useGb28181Store()
const onlineDevices = computed(() => store.devices.filter(d => d.online))

function eventColor(type) {
  if (type.includes('Alarm'))      return 'dot-danger'
  if (type.includes('Offline'))    return 'dot-warning'
  if (type.includes('Registered') || type.includes('Online')) return 'dot-success'
  return 'dot-info'
}

onMounted(() => {
  store.fetchStats()
  store.fetchDevices()
})
</script>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}
.stat-card {
  background: var(--card-bg); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 20px 24px;
  border-top: 3px solid var(--border);
}
.stat-card--green { border-top-color: var(--success); }
.stat-card--blue  { border-top-color: var(--primary); }
.stat-card--amber { border-top-color: var(--warning); }
.stat-label { font-size: 12px; color: var(--text-muted); margin-bottom: 8px; }
.stat-value { font-size: 32px; font-weight: 700; color: var(--text); }

.dashboard-grid {
  display: grid; grid-template-columns: 1fr 1fr; gap: 20px;
}
.section-title { font-size: 15px; font-weight: 600; }

.device-mini-list { display: flex; flex-direction: column; gap: 2px; }
.device-mini-item {
  display: flex; align-items: center;
  padding: 8px 6px; border-radius: 6px;
  transition: background .1s;
}
.device-mini-item:hover { background: var(--bg); }
.device-mini-id { font-size: 13px; font-weight: 500; color: var(--primary); }
.device-mini-meta { font-size: 12px; color: var(--text-muted); }

.event-mini-list { display: flex; flex-direction: column; gap: 2px; }
.event-mini-item {
  display: flex; align-items: center; gap: 8px;
  padding: 7px 6px; border-radius: 6px;
}
.event-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
.dot-danger  { background: var(--danger); }
.dot-warning { background: var(--warning); }
.dot-success { background: var(--success); }
.dot-info    { background: var(--primary); }
.event-type  { font-size: 13px; font-weight: 500; margin-right: 6px; }
.event-id    { font-size: 12px; color: var(--text-muted); }
.event-time  { margin-left: auto; font-size: 11px; color: var(--text-muted); white-space: nowrap; }

@media (max-width: 900px) {
  .stats-grid    { grid-template-columns: repeat(2, 1fr); }
  .dashboard-grid { grid-template-columns: 1fr; }
}
</style>
