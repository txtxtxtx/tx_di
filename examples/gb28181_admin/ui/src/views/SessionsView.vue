<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>会话管理</h1>
        <p>当前活跃点播 / 回放会话</p>
      </div>
      <button class="btn btn-primary" @click="store.fetchSessions()">刷新</button>
    </div>

    <div class="card">
      <div v-if="store.sessions.length === 0" class="empty-state">
        <div class="icon">🎬</div>
        <p>暂无活跃会话</p>
      </div>
      <div v-else class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>类型</th>
              <th>设备 ID</th>
              <th>通道 ID</th>
              <th>RTP 端口</th>
              <th>SSRC</th>
              <th>流 ID</th>
              <th style="text-align:right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in store.sessions" :key="s.call_id">
              <td>
                <span :class="['badge', s.is_realtime ? 'badge-session' : 'badge-online']">
                  {{ s.is_realtime ? '实时' : '回放' }}
                </span>
              </td>
              <td>
                <RouterLink :to="`/devices/${s.device_id}`" class="device-link">
                  {{ s.device_id }}
                </RouterLink>
              </td>
              <td class="mono">{{ s.channel_id }}</td>
              <td>{{ s.rtp_port }}</td>
              <td class="mono">{{ s.ssrc }}</td>
              <td class="mono text-sm">{{ s.stream_id }}</td>
              <td>
                <div style="display:flex;justify-content:flex-end">
                  <button class="btn btn-sm btn-danger" @click="handleHangup(s.call_id)" :disabled="hangupLoading[s.call_id]">
                    <span v-if="hangupLoading[s.call_id]" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
                    挂断
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useGb28181Store } from '../stores/gb28181.js'
import api from '../api/index.js'

const store = useGb28181Store()
const hangupLoading = ref({})

async function handleHangup(callId) {
  hangupLoading.value[callId] = true
  try {
    await api.hangup(callId)
    await store.fetchSessions()
  } finally {
    hangupLoading.value[callId] = false
  }
}

onMounted(() => store.fetchSessions())
</script>

<style scoped>
.mono { font-family: monospace; font-size: 12px; }
.text-sm { font-size: 12px; }
.device-link { color: var(--primary); font-weight: 500; }
.device-link:hover { text-decoration: underline; }
</style>
