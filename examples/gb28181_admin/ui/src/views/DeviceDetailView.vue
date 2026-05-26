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

    <div v-else-if="device">
      <!-- Tab 切换 -->
      <div class="tabs">
        <button
          v-for="tab in tabs" :key="tab.id"
          :class="['tab-btn', activeTab === tab.id ? 'active' : '']"
          @click="activeTab = tab.id"
        >{{ tab.label }}</button>
      </div>

      <!-- 基本信息 + 通道列表 -->
      <div v-show="activeTab === 'info'" class="detail-layout">
        <!-- 左侧：基本信息 + PTZ 控制 -->
        <div class="left-col">
          <div class="card">
            <h2 class="card-title">基本信息</h2>
            <div class="info-grid">
              <div class="info-item" v-for="item in deviceInfoItems" :key="item.label">
                <span class="info-label">{{ item.label }}</span>
                <span class="info-value">{{ item.value }}</span>
              </div>
            </div>
          </div>

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

      <!-- 高级控制面板 -->
      <div v-show="activeTab === 'advanced'" class="advanced-grid">
        <!-- 校时 -->
        <div class="card">
          <h3 class="card-title">网络校时</h3>
          <div class="btn-group">
            <button class="btn btn-sm" @click="adv('time_sync')">查询设备时间</button>
            <button class="btn btn-sm btn-primary" @click="adv('sync_time')">下发标准时间</button>
          </div>
          <div v-if="advMsg['time']" class="msg msg-ok">{{ advMsg['time'] }}</div>
        </div>

        <!-- 配置查询 -->
        <div class="card">
          <h3 class="card-title">配置查询</h3>
          <div class="btn-group">
            <button class="btn btn-sm" @click="advConfig('basic')">基本参数</button>
            <button class="btn btn-sm" @click="advConfig('network')">网络参数</button>
            <button class="btn btn-sm" @click="advConfig('video')">视频参数</button>
          </div>
          <p class="hint">配置回复通过 SSE 事件推送</p>
        </div>

        <!-- 看守位 -->
        <div class="card">
          <h3 class="card-title">看守位控制</h3>
          <div class="form-row">
            <label>通道 ID</label>
            <input v-model="guardChannelId" class="input-sm" placeholder="通道 ID" />
          </div>
          <div class="form-row mt-6">
            <label>预置位编号</label>
            <input v-model.number="guardPresetIdx" class="input-sm" type="number" min="0" max="255" />
          </div>
          <div class="btn-group mt-8">
            <button class="btn btn-sm" @click="advGuard('set')">设置看守位</button>
            <button class="btn btn-sm btn-primary" @click="advGuard('call')">调用看守位</button>
            <button class="btn btn-sm" @click="advGuard('clear')">清除</button>
          </div>
          <div class="btn-group mt-6">
            <button class="btn btn-sm" @click="advGuardBasic(true)">布防</button>
            <button class="btn btn-sm btn-danger" @click="advGuardBasic(false)">撤防</button>
            <button class="btn btn-sm" @click="adv('guard_info')">查询看守位</button>
          </div>
        </div>

        <!-- 预置位 -->
        <div class="card">
          <h3 class="card-title">预置位</h3>
          <div class="form-row">
            <label>通道 ID</label>
            <input v-model="presetChannelId" class="input-sm" placeholder="通道 ID" />
          </div>
          <div class="form-row mt-6">
            <label>预置位编号 (0-255)</label>
            <input v-model.number="presetIdx" class="input-sm" type="number" min="0" max="255" />
          </div>
          <div class="btn-group mt-8">
            <button class="btn btn-sm btn-primary" @click="advPreset('goto')">调用预置位</button>
            <button class="btn btn-sm" @click="advPreset('set')">设置预置位</button>
          </div>
        </div>

        <!-- 巡航 -->
        <div class="card">
          <h3 class="card-title">巡航控制</h3>
          <div class="form-row">
            <label>通道 ID</label>
            <input v-model="cruiseChannelId" class="input-sm" placeholder="通道 ID" />
          </div>
          <div class="form-row mt-6">
            <label>巡航轨迹编号 (0-255)</label>
            <input v-model.number="cruiseNo" class="input-sm" type="number" min="0" max="255" />
          </div>
          <div class="btn-group mt-8">
            <button class="btn btn-sm btn-primary" @click="advCruise('start')">启动巡航</button>
            <button class="btn btn-sm btn-danger" @click="advCruise('stop')">停止巡航</button>
            <button class="btn btn-sm" @click="advCruise('list')">巡航列表</button>
          </div>
        </div>

        <!-- 存储卡 -->
        <div class="card">
          <h3 class="card-title">存储卡</h3>
          <div class="form-row">
            <label>通道 ID</label>
            <input v-model="storageChannelId" class="input-sm" placeholder="通道 ID" />
          </div>
          <div class="btn-group mt-8">
            <button class="btn btn-sm" @click="advStorage('status')">查询状态</button>
            <button class="btn btn-sm btn-danger" @click="confirmStorageFormat()">格式化存储卡</button>
          </div>
          <div v-if="advMsg['storage']" class="msg msg-ok">{{ advMsg['storage'] }}</div>
        </div>

        <!-- 强制关键帧 / 目标跟踪 -->
        <div class="card">
          <h3 class="card-title">扩展控制</h3>
          <div class="form-row">
            <label>通道 ID</label>
            <input v-model="extChannelId" class="input-sm" placeholder="通道 ID" />
          </div>
          <div class="btn-group mt-8">
            <button class="btn btn-sm" @click="advExt('key_frame')">强制关键帧</button>
            <button class="btn btn-sm btn-primary" @click="advExt('track_start')">目标跟踪 开</button>
            <button class="btn btn-sm" @click="advExt('track_stop')">目标跟踪 关</button>
          </div>
          <div v-if="advMsg['ext']" class="msg msg-ok">{{ advMsg['ext'] }}</div>
        </div>

        <!-- 全局操作消息 -->
        <div v-if="advGlobalMsg" class="card" style="background:#e8f5e9">
          <p style="color:#2e7d32;font-size:13px">{{ advGlobalMsg }}</p>
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

// Tab
const activeTab = ref('info')
const tabs = [
  { id: 'info',     label: '📷 基本信息 / 通道' },
  { id: 'advanced', label: '⚙️ 高级控制' },
]

// 高级控制用的通道/参数 state
const guardChannelId  = ref('')
const guardPresetIdx  = ref(0)
const presetChannelId = ref('')
const presetIdx       = ref(1)
const cruiseChannelId = ref('')
const cruiseNo        = ref(1)
const storageChannelId = ref('')
const extChannelId    = ref('')

const advMsg       = ref({})
const advGlobalMsg = ref('')

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

// ─── 高级控制辅助 ───
async function showAdvMsg(group, fn) {
  try {
    await fn()
    advMsg.value[group] = '指令已发送'
    advGlobalMsg.value  = '指令已发送，结果通过 SSE 事件返回'
  } catch (e) {
    advMsg.value[group] = e.response?.data?.msg || '失败'
    advGlobalMsg.value  = ''
  }
  setTimeout(() => { advMsg.value[group] = ''; advGlobalMsg.value = '' }, 3000)
}

async function adv(type) {
  const callMap = {
    time_sync:  () => api.timeSync(id),
    sync_time:  () => api.syncTime(id),
    guard_info: () => api.guardInfo(id),
  }
  await showAdvMsg(type, callMap[type])
}

async function advConfig(type) {
  await showAdvMsg('config', () => api.queryConfig(id, type))
}

async function advGuard(mode) {
  await showAdvMsg('guard', () =>
    api.guardControl(id, guardChannelId.value || channels.value[0]?.channel_id || '', mode, guardPresetIdx.value)
  )
}

async function advGuardBasic(guard) {
  await showAdvMsg('guard', () =>
    api.guardBasic(id, guardChannelId.value || channels.value[0]?.channel_id || '', guard)
  )
}

async function advPreset(action) {
  const ch = presetChannelId.value || channels.value[0]?.channel_id || ''
  if (action === 'goto') {
    await showAdvMsg('preset', () => api.gotoPreset(id, ch, presetIdx.value))
  } else {
    await showAdvMsg('preset', () => api.setPreset(id, ch, presetIdx.value))
  }
}

async function advCruise(action) {
  const ch = cruiseChannelId.value || channels.value[0]?.channel_id || ''
  if (action === 'start') {
    await showAdvMsg('cruise', () => api.startCruise(id, ch, cruiseNo.value))
  } else if (action === 'stop') {
    await showAdvMsg('cruise', () => api.stopCruise(id, ch, cruiseNo.value))
  } else {
    await showAdvMsg('cruise', () => api.cruiseList(id, ch))
  }
}

async function advStorage(action) {
  const ch = storageChannelId.value || channels.value[0]?.channel_id || ''
  if (action === 'status') {
    await showAdvMsg('storage', () => api.storageStatus(id, ch))
  }
}

async function confirmStorageFormat() {
  if (!confirm('⚠️ 确定要格式化存储卡？此操作不可逆！')) return
  const ch = storageChannelId.value || channels.value[0]?.channel_id || ''
  await showAdvMsg('storage', () => api.storageFormat(id, ch))
}

async function advExt(action) {
  const ch = extChannelId.value || channels.value[0]?.channel_id || ''
  if (action === 'key_frame') {
    await showAdvMsg('ext', () => api.makeKeyFrame(id, ch))
  } else if (action === 'track_start') {
    await showAdvMsg('ext', () => api.targetTrack(id, ch, true))
  } else {
    await showAdvMsg('ext', () => api.targetTrack(id, ch, false))
  }
}

onMounted(refresh)
</script>

<style scoped>
.back-btn { color: var(--text-muted); font-size: 13px; }
.back-btn:hover { color: var(--primary); }
.loading-center { display: flex; justify-content: center; padding: 60px; }
.card-title { font-size: 15px; font-weight: 600; margin-bottom: 14px; }

/* Tabs */
.tabs { display: flex; gap: 0; margin-bottom: 16px; border-bottom: 2px solid var(--border); }
.tab-btn {
  padding: 9px 20px; font-size: 13px; font-weight: 500;
  border: none; background: none; cursor: pointer; color: var(--text-muted);
  border-bottom: 2px solid transparent; margin-bottom: -2px; transition: all .15s;
}
.tab-btn:hover  { color: var(--primary); }
.tab-btn.active { color: var(--primary); border-bottom-color: var(--primary); }

/* 基本信息布局 */
.detail-layout { display: grid; grid-template-columns: 360px 1fr; gap: 20px; margin-top: 4px; }
.info-grid { display: grid; gap: 8px; }
.info-item { display: flex; align-items: baseline; gap: 8px; }
.info-label { color: var(--text-muted); font-size: 12px; width: 80px; flex-shrink: 0; }
.info-value { font-size: 13px; font-weight: 500; word-break: break-all; }

.ptz-channel-sel { margin-bottom: 12px; display: flex; align-items: center; gap: 8px; font-size: 13px; }
.select { border: 1px solid var(--border); border-radius: 5px; padding: 5px 8px; font-size: 13px; outline: none; background: var(--card-bg); }
.mono { font-family: monospace; font-size: 12px; }

/* 高级控制 */
.advanced-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
  margin-top: 4px;
}
.form-row { display: flex; flex-direction: column; gap: 4px; }
.form-row label { font-size: 12px; color: var(--text-muted); }
.input-sm {
  padding: 5px 8px; border: 1px solid var(--border); border-radius: 5px;
  font-size: 13px; outline: none; width: 100%;
}
.input-sm:focus { border-color: var(--primary); }
.btn-group { display: flex; flex-wrap: wrap; gap: 6px; }
.mt-6  { margin-top: 6px; }
.mt-8  { margin-top: 8px; }
.mt-16 { margin-top: 16px; }
.hint { font-size: 12px; color: var(--text-muted); margin-top: 6px; }
.msg { margin-top: 8px; font-size: 12px; padding: 5px 8px; border-radius: 4px; }
.msg-ok { background: #e8f5e9; color: #2e7d32; }

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
  .advanced-grid { grid-template-columns: 1fr; }
}
</style>
