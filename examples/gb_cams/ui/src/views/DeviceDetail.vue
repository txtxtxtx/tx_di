<template>
  <div class="device-detail" v-if="device">
    <div class="page-header">
      <router-link to="/devices" class="back-link">← 返回设备列表</router-link>
      <h1>{{ device.name }}</h1>
    </div>

    <div class="info-grid">
      <div class="info-card">
        <div class="info-label">设备ID</div>
        <div class="info-value mono">{{ device.device_id }}</div>
      </div>
      <div class="info-card">
        <div class="info-label">SIP 端口</div>
        <div class="info-value">{{ device.sip_port }}</div>
      </div>
      <div class="info-card">
        <div class="info-label">注册状态</div>
        <div class="info-value">
          <span class="status-badge" :class="device.status">{{ statusLabel(device.status) }}</span>
        </div>
      </div>
      <div class="info-card">
        <div class="info-label">心跳次数</div>
        <div class="info-value">{{ device.keepalive_count }}</div>
      </div>
      <div class="info-card">
        <div class="info-label">通道数量</div>
        <div class="info-value">{{ device.channel_count }}</div>
      </div>
      <div class="info-card" v-if="device.error">
        <div class="info-label">错误信息</div>
        <div class="info-value error-text">{{ device.error }}</div>
      </div>
    </div>

    <div class="section">
      <h2>通道列表 ({{ channels.length }})</h2>
      <div class="channel-table">
        <table>
          <thead>
            <tr>
              <th>序号</th>
              <th>通道ID</th>
              <th>通道名称</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(ch, i) in channels" :key="ch.channel_id">
              <td>{{ i + 1 }}</td>
              <td class="mono">{{ ch.channel_id }}</td>
              <td>{{ ch.name }}</td>
              <td>
                <span class="status-badge" :class="ch.status === 'ON' ? 'registered' : 'idle'">
                  {{ ch.status }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
  <div v-else class="loading">
    <p v-if="error">{{ error }}</p>
    <p v-else>加载中...</p>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useDeviceStore } from '../stores/devices'

const route = useRoute()
const store = useDeviceStore()
const device = ref(null)
const error = ref(null)

const channels = computed(() => device.value?.channels || [])

onMounted(async () => {
  const id = route.params.id
  const data = await store.fetchDevice(id)
  if (data) {
    device.value = data
  } else {
    error.value = `设备 ${id} 不存在`
  }
})

function statusLabel(s) {
  const map = { registered: '已注册', registering: '注册中', idle: '空闲', failed: '失败', unregistered: '已注销' }
  return map[s] || s
}
</script>

<style scoped>
.back-link { color: #667eea; text-decoration: none; font-size: 14px; }
.back-link:hover { text-decoration: underline; }
.page-header { margin-bottom: 24px; }
.page-header h1 { margin-top: 8px; font-size: 24px; }
.info-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; margin-bottom: 32px; }
.info-card {
  background: white;
  padding: 16px;
  border-radius: 8px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
}
.info-label { font-size: 12px; color: #999; margin-bottom: 4px; }
.info-value { font-size: 16px; font-weight: 500; }
.info-value.mono { font-family: monospace; font-size: 14px; }
.error-text { color: #ff4d4f; font-size: 13px; }
.section h2 { font-size: 18px; margin-bottom: 16px; }
.channel-table {
  background: white;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
}
table { width: 100%; border-collapse: collapse; }
th { background: #fafafa; padding: 12px 16px; text-align: left; font-weight: 600; font-size: 13px; color: #666; border-bottom: 1px solid #eee; }
td { padding: 12px 16px; border-bottom: 1px solid #f0f0f0; font-size: 14px; }
.mono { font-family: monospace; font-size: 13px; }
.status-badge {
  padding: 2px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}
.status-badge.registered { background: #f6ffed; color: #52c41a; border: 1px solid #b7eb8f; }
.status-badge.registering { background: #e6f7ff; color: #1890ff; border: 1px solid #91d5ff; }
.status-badge.idle { background: #f5f5f5; color: #999; border: 1px solid #d9d9d9; }
.status-badge.failed { background: #fff2f0; color: #ff4d4f; border: 1px solid #ffccc7; }
.loading { text-align: center; padding: 64px; color: #999; }
</style>
