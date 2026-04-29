<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>事件日志</h1>
        <p>通过 SSE 实时接收的 GB28181 事件（最近 {{ store.events.length }} 条）</p>
      </div>
      <div class="flex gap-8">
        <select v-model="typeFilter" class="select">
          <option value="">全部类型</option>
          <option v-for="t in eventTypes" :key="t" :value="t">{{ t }}</option>
        </select>
        <button class="btn btn-danger btn-sm" @click="store.events.length = 0; store.events.splice(0)">清空</button>
      </div>
    </div>

    <div class="card">
      <div v-if="filteredEvents.length === 0" class="empty-state">
        <div class="icon">🔔</div>
        <p>暂无事件，等待 GB28181 设备上报...</p>
      </div>
      <div v-else class="event-list">
        <TransitionGroup name="event">
          <div v-for="(ev, i) in filteredEvents" :key="`${i}-${ev._time}`" :class="['event-row', eventClass(ev.type)]">
            <span class="event-time">{{ ev._time }}</span>
            <span class="event-badge" :style="{ background: eventBg(ev.type) }">{{ ev.type }}</span>
            <span class="event-body">{{ eventBody(ev) }}</span>
          </div>
        </TransitionGroup>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useGb28181Store } from '../stores/gb28181.js'

const store = useGb28181Store()
const typeFilter = ref('')

const eventTypes = computed(() => [...new Set(store.events.map(e => e.type))])

const filteredEvents = computed(() => {
  if (!typeFilter.value) return store.events
  return store.events.filter(e => e.type === typeFilter.value)
})

function eventClass(type) {
  if (type.includes('Alarm'))   return 'row-danger'
  if (type.includes('Offline')) return 'row-warning'
  if (type.includes('Registered') || type.includes('Online')) return 'row-success'
  return ''
}

function eventBg(type) {
  if (type.includes('Alarm'))   return '#fee2e2'
  if (type.includes('Offline')) return '#fef3c7'
  if (type.includes('Registered') || type.includes('Online')) return '#dcfce7'
  if (type.includes('Session')) return '#dbeafe'
  return '#f1f5f9'
}

function eventBody(ev) {
  const parts = []
  if (ev.device_id)          parts.push(`设备: ${ev.device_id}`)
  if (ev.channel_id)         parts.push(`通道: ${ev.channel_id}`)
  if (ev.call_id)            parts.push(`CallID: ${ev.call_id}`)
  if (ev.alarm_description)  parts.push(`报警: ${ev.alarm_description}`)
  if (ev.channel_count != null) parts.push(`${ev.channel_count} 个通道`)
  if (ev.longitude)          parts.push(`GPS: ${ev.longitude?.toFixed(4)}, ${ev.latitude?.toFixed(4)}`)
  if (ev.rtp_port)           parts.push(`RTP: ${ev.rtp_port}`)
  return parts.join('  |  ') || JSON.stringify(ev)
}
</script>

<style scoped>
.select {
  border: 1px solid var(--border); border-radius: 6px;
  padding: 6px 10px; font-size: 13px; outline: none;
}
.event-list { display: flex; flex-direction: column; }
.event-row {
  display: flex; align-items: baseline; gap: 10px;
  padding: 9px 12px; border-radius: 6px; margin-bottom: 3px;
  font-size: 13px; transition: background .1s;
}
.event-row:hover { background: var(--bg); }
.row-danger  { background: #fff5f5; }
.row-warning { background: #fffbeb; }
.row-success { background: #f0fdf4; }

.event-time  { color: var(--text-muted); font-size: 11px; white-space: nowrap; min-width: 70px; }
.event-badge {
  padding: 1px 7px; border-radius: 4px; font-size: 11px;
  font-weight: 600; white-space: nowrap; color: var(--text);
}
.event-body  { flex: 1; color: var(--text); }

.event-enter-active { transition: all .2s ease; }
.event-enter-from { opacity: 0; transform: translateY(-6px); }
</style>
