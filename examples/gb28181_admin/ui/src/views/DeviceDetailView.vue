<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div class="flex items-center gap-12">
        <RouterLink to="/devices" class="back-btn">← 返回</RouterLink>
        <div>
          <h1>{{ device?.device_id || id }}</h1>
          <p>
            <span :class="['badge', device?.online ? 'badge-online' : 'badge-offline']" style="margin-right:8px">
              {{ device?.online ? '在线' : '离线' }}
            </span>
            {{ device?.manufacturer }} {{ device?.model }}
          </p>
        </div>
      </div>
      <div class="flex gap-8">
        <button class="btn" @click="refresh" :disabled="loading">刷新</button>
        <button class="btn" @click="handleQueryCatalog">目录查询</button>
        <button class="btn" @click="handleQueryInfo">设备信息</button>
        <button class="btn" @click="handleQueryStatus">状态查询</button>
        <button class="btn btn-danger" @click="handleTeleboot" :disabled="!device?.online">重启设备</button>
      </div>
    </div>

    <div v-if="loading" class="loading-center">
      <span class="spinner"></span>
    </div>

    <div v-else-if="device" class="detail-layout">
      <!-- 左侧：基本信息 + PTZ 控制 -->
      <div class="left-col">
        <!-- 基本信息 -->
        <div class="card">
          <h2 class="card-title">基本信息</h2>
          <div class="info-grid">
            <div class="info-item" v-for="item in deviceInfoItems" :key="item.label">
              <span class="info-label">{{ item.label }}</span>
              <span class="info-value">{{ item.value }}</span>
            </div>
          </div>
        </div>

        <!-- PTZ 控制 -->
        <div class="card mt-16" v-if="channels.length > 0">
          <h2 class="card-title">PTZ 云台控制</h2>
          <div class="ptz-channel-sel">
            <label>选择通道：</label>
            <select v-model="ptzChannelId" class="select">
              <option v-for="ch in channels" :key="ch.channel_id" :value="ch.channel_id">
                {{ ch.name || ch.channel_id }}
              </option>
            </select>
          </div>
          <PtzControl :device-id="id" :channel-id="ptzChannelId" />
        </div>
      </div>

      <!-- 右侧：通道列表 -->
      <div class="right-col">
        <div class="card">
          <div class="flex items-center justify-between" style="margin-bottom:14px">
            <h2 class="card-title">通道列表（{{ channels.length }}）</h2>
            <button class="btn btn-sm btn-primary" @click="handleInviteFirst" :disabled="!device.online || channels.length === 0">
              点播第一路
            </button>
          </div>
          <div v-if="channels.length === 0" class="empty-state">
            <div class="icon">📺</div>
            <p>暂无通道，点击"目录查询"获取</p>
          </div>
          <div v-else class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>状态</th>
                  <th>通道 ID</th>
                  <th>名称</th>
                  <th>IP</th>
                  <th style="text-align:right">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="ch in channels" :key="ch.channel_id">
                  <td>
                    <span :class="['badge', ch.status === 'ON' ? 'badge-online' : 'badge-offline']">
                      {{ ch.status }}
                    </span>
                  </td>
                  <td class="mono">{{ ch.channel_id }}</td>
                  <td>{{ ch.name || '—' }}</td>
                  <td>{{ ch.ip_address || '—' }}</td>
                  <td>
                    <div class="flex gap-8" style="justify-content:flex-end">
                      <button
                        class="btn btn-sm btn-primary"
                        :disabled="ch.status !== 'ON' || inviteLoading[ch.channel_id]"
                        @click="handleInvite(ch.channel_id)"
                      >
                        <span v-if="inviteLoading[ch.channel_id]" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
                        点播
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- 点播结果弹窗 -->
    <div v-if="inviteResult" class="modal-mask" @click.self="inviteResult = null">
      <div class="modal card">
        <h3 style="margin-bottom:14px">点播成功</h3>
        <div class="url-item" v-for="(url, key) in inviteResult.urls" :key="key">
          <span class="url-label">{{ key.toUpperCase() }}</span>
          <span class="url-value">{{ url }}</span>
          <button class="btn btn-sm" @click="copy(url)">复制</button>
        </div>
        <div style="margin-top:16px;text-align:right">
          <button class="btn btn-danger btn-sm" @click="handleHangup(inviteResult.call_id)">挂断</button>
          <button class="btn btn-sm" style="margin-left:8px" @click="inviteResult = null">关闭</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import api from '../api/index.js'
import PtzControl from '../components/PtzControl.vue'

const route = useRoute()
const id = route.params.id

const device   = ref(null)
const channels = ref([])
const loading  = ref(true)
const inviteLoading  = ref({})
const inviteResult   = ref(null)
const ptzChannelId   = ref('')

const deviceInfoItems = computed(() => {
  if (!device.value) return []
  const d = device.value
  return [
    { label: '设备 ID',   value: d.device_id },
    { label: '连接地址',  value: d.remote_addr },
    { label: 'Contact',   value: d.contact },
    { label: '厂商',      value: d.manufacturer || '—' },
    { label: '型号',      value: d.model || '—' },
    { label: '固件版本',  value: d.firmware || '—' },
    { label: '注册时间',  value: d.registered_at },
  ]
})

async function refresh() {
  loading.value = true
  try {
    const res = await api.device(id)
    if (res.data.code === 200) {
      device.value   = res.data.data
      channels.value = res.data.data.channels || []
      if (channels.value.length > 0 && !ptzChannelId.value) {
        ptzChannelId.value = channels.value[0].channel_id
      }
    }
  } finally {
    loading.value = false
  }
}

async function handleQueryCatalog() { await api.queryCatalog(id); setTimeout(refresh, 1500) }
async function handleQueryInfo()    { await api.queryInfo(id) }
async function handleQueryStatus()  { await api.queryStatus(id) }
async function handleTeleboot()     {
  if (confirm('确定要远程重启该设备吗？')) await api.teleboot(id)
}

async function handleInvite(channelId) {
  inviteLoading.value[channelId] = true
  try {
    const res = await api.invite(id, channelId)
    if (res.data.code === 200) {
      const d = res.data.data
      inviteResult.value = { call_id: d.call_id, urls: { hls: d.hls, rtsp: d.rtsp, rtmp: d.rtmp } }
    }
  } finally {
    inviteLoading.value[channelId] = false
  }
}
function handleInviteFirst() {
  if (channels.value.length > 0) handleInvite(channels.value[0].channel_id)
}

async function handleHangup(callId) {
  await api.hangup(callId)
  inviteResult.value = null
}

function copy(text) { navigator.clipboard.writeText(text) }

onMounted(refresh)
</script>

<style scoped>
.back-btn { color: var(--text-muted); font-size: 13px; }
.back-btn:hover { color: var(--primary); }
.loading-center { display: flex; justify-content: center; padding: 60px; }
.card-title { font-size: 15px; font-weight: 600; margin-bottom: 14px; }

.detail-layout { display: grid; grid-template-columns: 360px 1fr; gap: 20px; margin-top: 4px; }
.info-grid { display: grid; gap: 8px; }
.info-item { display: flex; align-items: baseline; gap: 8px; }
.info-label { color: var(--text-muted); font-size: 12px; width: 80px; flex-shrink: 0; }
.info-value { font-size: 13px; font-weight: 500; word-break: break-all; }

.ptz-channel-sel { margin-bottom: 12px; display: flex; align-items: center; gap: 8px; font-size: 13px; }
.select { border: 1px solid var(--border); border-radius: 5px; padding: 5px 8px; font-size: 13px; outline: none; }

.mono { font-family: monospace; font-size: 12px; }

/* 弹窗 */
.modal-mask {
  position: fixed; inset: 0; background: rgba(0,0,0,.35);
  display: flex; align-items: center; justify-content: center; z-index: 100;
}
.modal { min-width: 400px; max-width: 560px; }
.url-item { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
.url-label { width: 48px; font-size: 12px; font-weight: 600; color: var(--text-muted); flex-shrink: 0; }
.url-value { flex: 1; font-size: 12px; font-family: monospace; word-break: break-all; }

@media (max-width: 900px) {
  .detail-layout { grid-template-columns: 1fr; }
}
</style>
