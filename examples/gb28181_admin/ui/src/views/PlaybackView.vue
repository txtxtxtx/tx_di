<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>录像 / 回放</h1>
        <p>录像查询、历史回放与录像控制</p>
      </div>
    </div>

    <!-- 查询区 -->
    <div class="card">
      <h2 class="card-title">录像查询</h2>
      <div class="query-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="queryDeviceId" class="input" placeholder="输入设备 ID" />
        </div>
        <div class="form-row">
          <label>通道 ID *</label>
          <input v-model="queryChannelId" class="input" placeholder="输入通道 ID" />
        </div>
        <div class="form-row">
          <label>开始时间</label>
          <input v-model="queryStart" class="input" type="datetime-local" />
        </div>
        <div class="form-row">
          <label>结束时间</label>
          <input v-model="queryEnd" class="input" type="datetime-local" />
        </div>
        <div class="form-row">
          <label>录像类型</label>
          <select v-model.number="queryType" class="select">
            <option :value="0">全部</option>
            <option :value="1">定时</option>
            <option :value="2">报警</option>
            <option :value="3">手动</option>
          </select>
        </div>
        <div class="form-row align-end">
          <button class="btn btn-primary" @click="handleQueryRecords" :disabled="!queryDeviceId || !queryChannelId || queryLoading">
            <span v-if="queryLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
            发送查询
          </button>
        </div>
      </div>
      <div v-if="queryMsg" class="msg" :class="queryMsgOk ? 'msg-ok' : 'msg-err'">{{ queryMsg }}</div>
      <p class="hint">录像列表由设备异步返回，请在"事件日志"中查看 RecordInfoReceived 事件</p>
    </div>

    <!-- 历史回放 -->
    <div class="card mt-16">
      <h2 class="card-title">历史回放</h2>
      <div class="query-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="pbDeviceId" class="input" placeholder="设备 ID" />
        </div>
        <div class="form-row">
          <label>通道 ID *</label>
          <input v-model="pbChannelId" class="input" placeholder="通道 ID" />
        </div>
        <div class="form-row">
          <label>开始时间</label>
          <input v-model="pbStart" class="input" type="datetime-local" />
        </div>
        <div class="form-row">
          <label>结束时间</label>
          <input v-model="pbEnd" class="input" type="datetime-local" />
        </div>
        <div class="form-row align-end">
          <button class="btn btn-primary" @click="handleStartPlayback" :disabled="!pbDeviceId || !pbChannelId || pbLoading">
            <span v-if="pbLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
            发起回放
          </button>
        </div>
      </div>

      <!-- 当前回放会话 -->
      <div v-if="pbResult" class="pb-result">
        <h3>回放会话 <span class="mono">{{ pbResult.call_id }}</span></h3>
        <div class="url-grid">
          <div v-for="(url, key) in pbResult.urls" :key="key" class="url-item" v-if="url">
            <span class="url-label">{{ key.toUpperCase() }}</span>
            <span class="url-value">{{ url }}</span>
            <button class="btn btn-sm" @click="copy(url)">复制</button>
          </div>
        </div>

        <!-- 回放控制 -->
        <div class="pb-controls mt-16">
          <h4 class="ctrl-title">回放控制</h4>
          <div class="ctrl-row">
            <button class="btn btn-sm" @click="pbCtrl({ cmd: 'pause' })">⏸ 暂停</button>
            <button class="btn btn-sm" @click="pbCtrl({ cmd: 'resume' })">▶ 继续</button>
            <button class="btn btn-sm" @click="pbCtrl({ cmd: 'fast_forward', speed: 2 })">⏩ 2x</button>
            <button class="btn btn-sm" @click="pbCtrl({ cmd: 'fast_forward', speed: 4 })">⏩ 4x</button>
            <button class="btn btn-sm" @click="pbCtrl({ cmd: 'slow_forward', speed: 2 })">🐢 慢放</button>
            <button class="btn btn-sm btn-danger" @click="stopPlayback">⏹ 停止</button>
          </div>
          <div class="ctrl-row mt-8">
            <label style="font-size:12px;margin-right:6px">拖动到：</label>
            <input v-model="seekTime" class="input" type="datetime-local" style="width:210px" />
            <button class="btn btn-sm btn-primary" @click="pbCtrl({ cmd: 'seek', time: seekTime })">跳转</button>
          </div>
          <div v-if="pbCtrlMsg" class="msg" :class="pbCtrlOk ? 'msg-ok' : 'msg-err'">{{ pbCtrlMsg }}</div>
        </div>
      </div>
    </div>

    <!-- 录像控制 -->
    <div class="card mt-16">
      <h2 class="card-title">录像控制（录制启停）</h2>
      <div class="query-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="recCtrlDeviceId" class="input" placeholder="设备 ID" />
        </div>
        <div class="form-row">
          <label>通道 ID *</label>
          <input v-model="recCtrlChannelId" class="input" placeholder="通道 ID" />
        </div>
        <div class="form-row align-end">
          <button class="btn btn-primary" @click="sendRecordCtrl(true)" :disabled="!recCtrlDeviceId || !recCtrlChannelId">
            开始录像
          </button>
          <button class="btn btn-danger" @click="sendRecordCtrl(false)" :disabled="!recCtrlDeviceId || !recCtrlChannelId" style="margin-left:8px">
            停止录像
          </button>
        </div>
      </div>
      <div v-if="recCtrlMsg" class="msg" :class="recCtrlOk ? 'msg-ok' : 'msg-err'">{{ recCtrlMsg }}</div>
    </div>

    <!-- 录像下载 -->
    <div class="card mt-16">
      <h2 class="card-title">录像下载</h2>
      <div class="query-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="dlDeviceId" class="input" placeholder="设备 ID" />
        </div>
        <div class="form-row">
          <label>通道 ID *</label>
          <input v-model="dlChannelId" class="input" placeholder="通道 ID" />
        </div>
        <div class="form-row">
          <label>下载速度（可选）</label>
          <input v-model.number="dlSpeed" class="input" type="number" min="1" placeholder="不限速" />
        </div>
        <div class="form-row align-end">
          <button class="btn btn-primary" @click="handleStartDownload" :disabled="!dlDeviceId || !dlChannelId || dlLoading">
            <span v-if="dlLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
            发起下载
          </button>
        </div>
      </div>
      <div v-if="dlResult" class="pb-result">
        <h3>下载会话 <span class="mono">{{ dlResult.call_id }}</span></h3>
        <div class="url-grid">
          <div v-for="(url, key) in dlResult.urls" :key="key" class="url-item" v-if="url">
            <span class="url-label">{{ key.toUpperCase() }}</span>
            <span class="url-value">{{ url }}</span>
            <button class="btn btn-sm" @click="copy(url)">复制</button>
          </div>
        </div>
        <button class="btn btn-danger btn-sm mt-8" @click="hangupDl">挂断下载</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import api from '../api/index.js'

// ── 录像查询 ──
const queryDeviceId  = ref('')
const queryChannelId = ref('')
const queryStart     = ref('')
const queryEnd       = ref('')
const queryType      = ref(0)
const queryLoading   = ref(false)
const queryMsg       = ref('')
const queryMsgOk     = ref(true)

async function handleQueryRecords() {
  queryLoading.value = true
  try {
    await api.queryRecords(queryDeviceId.value, {
      channel_id:  queryChannelId.value,
      start_time:  queryStart.value || new Date(Date.now() - 86400000).toISOString().slice(0, 16),
      end_time:    queryEnd.value   || new Date().toISOString().slice(0, 16),
      record_type: queryType.value,
    })
    queryMsg.value = '录像查询已发送，等待设备响应'
    queryMsgOk.value = true
  } catch (e) {
    queryMsg.value = e.response?.data?.msg || '发送失败'
    queryMsgOk.value = false
  } finally {
    queryLoading.value = false
  }
  setTimeout(() => queryMsg.value = '', 4000)
}

// ── 历史回放 ──
const pbDeviceId  = ref('')
const pbChannelId = ref('')
const pbStart     = ref('')
const pbEnd       = ref('')
const pbLoading   = ref(false)
const pbResult    = ref(null)
const seekTime    = ref('')
const pbCtrlMsg   = ref('')
const pbCtrlOk    = ref(true)

async function handleStartPlayback() {
  pbLoading.value = true
  try {
    const res = await api.startPlayback(pbDeviceId.value, {
      channel_id: pbChannelId.value,
      start_time: pbStart.value || new Date(Date.now() - 3600000).toISOString().slice(0, 16),
      end_time:   pbEnd.value   || new Date().toISOString().slice(0, 16),
    })
    if (res.data.code === 200) {
      pbResult.value = res.data.data
    }
  } catch (e) {
    alert(e.response?.data?.msg || '发起回放失败')
  } finally {
    pbLoading.value = false
  }
}

async function pbCtrl(body) {
  try {
    await api.playbackControl(pbDeviceId.value, body)
    pbCtrlMsg.value = '控制指令已发送'
    pbCtrlOk.value  = true
  } catch (e) {
    pbCtrlMsg.value = e.response?.data?.msg || '指令失败'
    pbCtrlOk.value  = false
  }
  setTimeout(() => pbCtrlMsg.value = '', 2000)
}

async function stopPlayback() {
  if (!pbResult.value) return
  await api.hangup(pbResult.value.call_id)
  pbResult.value = null
}

// ── 录像控制 ──
const recCtrlDeviceId  = ref('')
const recCtrlChannelId = ref('')
const recCtrlMsg       = ref('')
const recCtrlOk        = ref(true)

async function sendRecordCtrl(start) {
  try {
    await api.recordControl(recCtrlDeviceId.value, {
      channel_id: recCtrlChannelId.value,
      start,
    })
    recCtrlMsg.value = start ? '开始录像指令已发送' : '停止录像指令已发送'
    recCtrlOk.value  = true
  } catch (e) {
    recCtrlMsg.value = e.response?.data?.msg || '指令失败'
    recCtrlOk.value  = false
  }
  setTimeout(() => recCtrlMsg.value = '', 3000)
}

// ── 录像下载 ──
const dlDeviceId  = ref('')
const dlChannelId = ref('')
const dlSpeed     = ref(null)
const dlLoading   = ref(false)
const dlResult    = ref(null)

async function handleStartDownload() {
  dlLoading.value = true
  try {
    const res = await api.startDownload(dlDeviceId.value, {
      channel_id:     dlChannelId.value,
      download_speed: dlSpeed.value || null,
    })
    if (res.data.code === 200) dlResult.value = res.data.data
  } catch (e) {
    alert(e.response?.data?.msg || '发起下载失败')
  } finally {
    dlLoading.value = false
  }
}

async function hangupDl() {
  if (!dlResult.value) return
  await api.hangup(dlResult.value.call_id)
  dlResult.value = null
}

function copy(text) { navigator.clipboard.writeText(text) }
</script>

<style scoped>
.card-title { font-size: 15px; font-weight: 600; margin-bottom: 14px; }
.query-form { display: flex; flex-wrap: wrap; gap: 14px; align-items: flex-end; }
.form-row { display: flex; flex-direction: column; gap: 4px; }
.form-row label { font-size: 12px; color: var(--text-muted); }
.form-row.align-end { justify-content: flex-end; }
.input {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; width: 200px;
}
.input:focus { border-color: var(--primary); }
.select {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; background: var(--card-bg);
}
.hint { font-size: 12px; color: var(--text-muted); margin-top: 8px; }
.msg { margin-top: 10px; font-size: 13px; padding: 6px 10px; border-radius: 5px; }
.msg-ok  { background: #e8f5e9; color: #2e7d32; }
.msg-err { background: #ffebee; color: #c62828; }
.mt-8  { margin-top: 8px; }
.mt-16 { margin-top: 16px; }

/* 回放结果 */
.pb-result {
  margin-top: 16px; padding: 14px; border: 1px solid var(--border);
  border-radius: var(--radius); background: var(--bg);
}
.pb-result h3 { font-size: 14px; font-weight: 600; margin-bottom: 10px; }
.url-grid { display: flex; flex-direction: column; gap: 6px; }
.url-item { display: flex; align-items: center; gap: 8px; }
.url-label { width: 56px; font-size: 12px; font-weight: 600; color: var(--text-muted); }
.url-value { flex: 1; font-size: 12px; font-family: monospace; word-break: break-all; }

/* 回放控制 */
.ctrl-title { font-size: 13px; font-weight: 600; margin-bottom: 8px; }
.ctrl-row { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.mono { font-family: monospace; font-size: 12px; }
</style>
