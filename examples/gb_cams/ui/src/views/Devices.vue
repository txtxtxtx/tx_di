<template>
  <div class="devices-page">
    <div class="page-header">
      <h1>设备管理</h1>
      <div class="header-actions">
        <button @click="showGenerateModal = true" class="btn btn-primary">批量生成</button>
        <button @click="refresh" class="btn btn-outline">刷新</button>
      </div>
    </div>

    <!-- 统计条 -->
    <div class="stats-bar">
      <span>共 <strong>{{ store.stats.total }}</strong> 台设备</span>
      <span>在线 <strong>{{ store.stats.online }}</strong></span>
      <span>通道 <strong>{{ store.stats.channels }}</strong></span>
    </div>

    <!-- 设备列表 -->
    <div class="device-table">
      <table>
        <thead>
          <tr>
            <th>设备ID</th>
            <th>名称</th>
            <th>SIP端口</th>
            <th>通道数</th>
            <th>状态</th>
            <th>心跳次数</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="store.devices.length === 0">
            <td colspan="7" class="empty-row">暂无设备，请批量生成</td>
          </tr>
          <tr v-for="dev in store.devices" :key="dev.device_id">
            <td class="device-id">{{ dev.device_id }}</td>
            <td>{{ dev.name }}</td>
            <td>{{ dev.sip_port }}</td>
            <td>{{ dev.channel_count }}</td>
            <td>
              <span class="status-badge" :class="dev.status">
                {{ statusLabel(dev.status) }}
              </span>
            </td>
            <td>{{ dev.keepalive_count }}</td>
            <td>
              <router-link :to="`/devices/${dev.device_id}`" class="action-link">详情</router-link>
              <button @click="deleteDevice(dev.device_id)" class="action-btn danger">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 批量生成弹窗 -->
    <div v-if="showGenerateModal" class="modal-overlay" @click.self="showGenerateModal = false">
      <div class="modal">
        <h2>批量生成设备</h2>
        <div class="form-group">
          <label>设备数量</label>
          <input v-model.number="genForm.count" type="number" min="1" max="1000" />
        </div>
        <div class="form-group">
          <label>每设备通道数</label>
          <input v-model.number="genForm.channels_per_device" type="number" min="1" max="64" />
        </div>
        <div class="form-group">
          <label>设备ID前缀（14位）</label>
          <input v-model="genForm.prefix" type="text" maxlength="14" />
        </div>
        <div class="form-group">
          <label>起始序号</label>
          <input v-model.number="genForm.base_seq" type="number" min="1" />
        </div>
        <div class="form-group">
          <label>
            <input v-model="genForm.auto_register" type="checkbox" />
            生成后自动注册到上级平台
          </label>
        </div>
        <div class="modal-actions">
          <button @click="doGenerate" class="btn btn-primary" :disabled="generating">
            {{ generating ? '生成中...' : '确认生成' }}
          </button>
          <button @click="showGenerateModal = false" class="btn btn-outline">取消</button>
        </div>
        <div v-if="genResult" class="gen-result">
          ✅ 已生成 {{ genResult.count }} 个设备
          <span v-if="genResult.auto_register">（自动注册中...）</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useDeviceStore } from '../stores/devices'

const store = useDeviceStore()
const showGenerateModal = ref(false)
const generating = ref(false)
const genResult = ref(null)
const genForm = ref({
  count: 10,
  channels_per_device: 4,
  prefix: '34020000001320',
  base_seq: 1,
  auto_register: false,
})

onMounted(() => {
  store.fetchDevices()
  store.fetchStats()
})

function refresh() {
  store.fetchDevices()
  store.fetchStats()
}

function statusLabel(s) {
  const map = { registered: '已注册', registering: '注册中', idle: '空闲', failed: '失败', unregistered: '已注销' }
  return map[s] || s
}

async function doGenerate() {
  generating.value = true
  genResult.value = null
  try {
    const resp = await store.generateDevices(genForm.value)
    if (resp.code === 0) {
      genResult.value = resp.data
      await store.fetchDevices()
      await store.fetchStats()
    }
  } finally {
    generating.value = false
  }
}

async function deleteDevice(id) {
  if (!confirm(`确定删除设备 ${id}？`)) return
  await store.removeDevice(id)
  await store.fetchDevices()
  await store.fetchStats()
}
</script>

<style scoped>
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.page-header h1 { font-size: 24px; }
.header-actions { display: flex; gap: 8px; }
.stats-bar {
  background: white;
  padding: 12px 20px;
  border-radius: 8px;
  margin-bottom: 16px;
  display: flex;
  gap: 24px;
  font-size: 14px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
}
.stats-bar strong { color: #667eea; }
.device-table {
  background: white;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
}
table { width: 100%; border-collapse: collapse; }
th { background: #fafafa; padding: 12px 16px; text-align: left; font-weight: 600; font-size: 13px; color: #666; border-bottom: 1px solid #eee; }
td { padding: 12px 16px; border-bottom: 1px solid #f0f0f0; font-size: 14px; }
.device-id { font-family: monospace; font-size: 13px; }
.empty-row { text-align: center; color: #999; padding: 32px; }
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
.status-badge.unregistered { background: #fff7e6; color: #faad14; border: 1px solid #ffe58f; }
.action-link { color: #667eea; text-decoration: none; margin-right: 12px; font-size: 13px; }
.action-link:hover { text-decoration: underline; }
.action-btn {
  padding: 2px 8px;
  border: 1px solid #d9d9d9;
  border-radius: 4px;
  background: white;
  font-size: 12px;
  cursor: pointer;
}
.action-btn.danger { color: #ff4d4f; border-color: #ff4d4f; }
.action-btn.danger:hover { background: #fff2f0; }
.btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}
.btn:disabled { opacity: 0.6; cursor: not-allowed; }
.btn-primary { background: #667eea; color: white; }
.btn-primary:hover:not(:disabled) { background: #5a6fd6; }
.btn-outline { background: white; border: 1px solid #d9d9d9; color: #333; }
.btn-outline:hover { border-color: #667eea; color: #667eea; }
.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: white;
  border-radius: 12px;
  padding: 32px;
  width: 480px;
  max-width: 90vw;
}
.modal h2 { margin-bottom: 24px; font-size: 20px; }
.form-group { margin-bottom: 16px; }
.form-group label { display: block; margin-bottom: 4px; font-size: 14px; color: #666; }
.form-group input[type="number"],
.form-group input[type="text"] {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #d9d9d9;
  border-radius: 6px;
  font-size: 14px;
}
.form-group input[type="checkbox"] { margin-right: 8px; }
.modal-actions { display: flex; gap: 12px; margin-top: 24px; }
.gen-result {
  margin-top: 16px;
  padding: 12px;
  background: #f6ffed;
  border: 1px solid #b7eb8f;
  border-radius: 6px;
  color: #52c41a;
  font-size: 14px;
}
</style>
