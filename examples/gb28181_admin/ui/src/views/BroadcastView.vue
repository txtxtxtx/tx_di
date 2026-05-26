<template>
  <div>
    <div class="page-header">
      <h1>语音广播 / 对讲</h1>
      <p>向设备发起语音广播邀请或双向对讲</p>
    </div>

    <!-- 语音广播 -->
    <div class="card">
      <h2 class="card-title">语音广播</h2>
      <p class="desc">
        平台发起广播邀请 → 设备向平台推送音频流。流程：发送邀请 → 收到设备确认后接受 → 完成后停止。
      </p>
      <div class="ctrl-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="bcDeviceId" class="input" placeholder="设备 ID" />
        </div>
        <div class="form-row">
          <label>接收端口</label>
          <input v-model.number="bcAudioPort" class="input" type="number" min="1024" max="65535" placeholder="平台接收端口" />
        </div>
      </div>

      <div class="action-row mt-12">
        <button
          class="btn btn-primary"
          @click="handleBroadcastInvite"
          :disabled="!bcDeviceId || bcLoading || bcActive"
        >
          <span v-if="bcLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
          发送广播邀请
        </button>
        <button
          class="btn"
          @click="handleBroadcastAccept"
          :disabled="!bcDeviceId || !bcAudioPort || !bcActive"
        >
          确认接收（端口 {{ bcAudioPort }}）
        </button>
        <button
          class="btn btn-danger"
          @click="handleBroadcastStop"
          :disabled="!bcDeviceId || !bcActive"
        >
          结束广播
        </button>
      </div>

      <div v-if="bcStatus" class="status-badge" :class="bcActive ? 'status-active' : 'status-idle'">
        {{ bcStatus }}
      </div>
      <div v-if="bcMsg" class="msg" :class="bcMsgOk ? 'msg-ok' : 'msg-err'">{{ bcMsg }}</div>
    </div>

    <!-- 语音对讲 -->
    <div class="card mt-16">
      <h2 class="card-title">语音对讲（双向）</h2>
      <p class="desc">
        平台发起双向对讲 INVITE，同时传输发送和接收端的音频。
      </p>
      <div class="ctrl-form">
        <div class="form-row">
          <label>设备 ID *</label>
          <input v-model="tbDeviceId" class="input" placeholder="设备 ID" />
        </div>
        <div class="form-row">
          <label>通道 ID *</label>
          <input v-model="tbChannelId" class="input" placeholder="通道 ID" />
        </div>
        <div class="form-row">
          <label>本端音频端口 *</label>
          <input v-model.number="tbAudioPort" class="input" type="number" min="1024" max="65535" placeholder="平台发送端口" />
        </div>
        <div class="form-row">
          <label>音频编码</label>
          <select v-model="tbCodec" class="select">
            <option value="">默认(PCMU)</option>
            <option value="pcma">PCMA</option>
            <option value="aac">AAC</option>
            <option value="g7221">G.722.1</option>
          </select>
        </div>
      </div>

      <div class="action-row mt-12">
        <button
          class="btn btn-primary"
          @click="handleStartTalkback"
          :disabled="!tbDeviceId || !tbChannelId || !tbAudioPort || tbLoading || !!tbSession"
        >
          <span v-if="tbLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
          发起对讲
        </button>
        <button
          class="btn btn-danger"
          @click="handleStopTalkback"
          :disabled="!tbSession"
        >
          挂断对讲
        </button>
      </div>

      <!-- 对讲会话信息 -->
      <div v-if="tbSession" class="session-info mt-12">
        <h4 class="session-title">对讲会话</h4>
        <div class="info-row"><span class="il">会话 ID</span><span class="iv mono">{{ tbSession.call_id }}</span></div>
        <div class="info-row"><span class="il">设备音频地址</span><span class="iv mono">{{ tbSession.device_ip }}:{{ tbSession.device_audio_port }}</span></div>
        <p class="hint">将本端音频 RTP 流发送到上述地址即可完成对讲。</p>
      </div>
      <div v-if="tbMsg" class="msg" :class="tbMsgOk ? 'msg-ok' : 'msg-err'">{{ tbMsg }}</div>
    </div>

    <!-- 对讲历史 -->
    <div class="card mt-16" v-if="history.length > 0">
      <h2 class="card-title">操作历史</h2>
      <div class="history-list">
        <div v-for="(h, i) in history" :key="i" class="history-item">
          <span class="history-time">{{ h.time }}</span>
          <span class="history-type" :class="h.ok ? 'ok' : 'err'">{{ h.type }}</span>
          <span class="history-msg">{{ h.msg }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import api from '../api/index.js'

// ── 广播 ──
const bcDeviceId  = ref('')
const bcAudioPort = ref(10000)
const bcLoading   = ref(false)
const bcActive    = ref(false)
const bcStatus    = ref('')
const bcMsg       = ref('')
const bcMsgOk     = ref(true)

async function handleBroadcastInvite() {
  bcLoading.value = true
  try {
    await api.broadcastInvite(bcDeviceId.value)
    bcActive.value = true
    bcStatus.value = '已发送广播邀请，等待设备响应后点击"确认接收"'
    addHistory('广播邀请', true, `设备 ${bcDeviceId.value}`)
  } catch (e) {
    bcMsg.value   = e.response?.data?.msg || '发送失败'
    bcMsgOk.value = false
    addHistory('广播邀请', false, bcMsg.value)
  } finally {
    bcLoading.value = false
  }
  setTimeout(() => bcMsg.value = '', 3000)
}

async function handleBroadcastAccept() {
  try {
    await api.broadcastAccept(bcDeviceId.value, bcAudioPort.value)
    bcStatus.value = `✅ 广播接收中，监听端口 ${bcAudioPort.value}`
    addHistory('确认接收', true, `端口 ${bcAudioPort.value}`)
  } catch (e) {
    bcMsg.value   = e.response?.data?.msg || '确认失败'
    bcMsgOk.value = false
    addHistory('确认接收', false, bcMsg.value)
  }
  setTimeout(() => bcMsg.value = '', 3000)
}

async function handleBroadcastStop() {
  try {
    await api.broadcastStop(bcDeviceId.value)
    bcActive.value = false
    bcStatus.value = '广播已结束'
    addHistory('结束广播', true, `设备 ${bcDeviceId.value}`)
  } catch (e) {
    bcMsg.value   = e.response?.data?.msg || '结束失败'
    bcMsgOk.value = false
    addHistory('结束广播', false, bcMsg.value)
  }
  setTimeout(() => bcMsg.value = '', 3000)
}

// ── 对讲 ──
const tbDeviceId  = ref('')
const tbChannelId = ref('')
const tbAudioPort = ref(20000)
const tbCodec     = ref('')
const tbLoading   = ref(false)
const tbSession   = ref(null)
const tbMsg       = ref('')
const tbMsgOk     = ref(true)

async function handleStartTalkback() {
  tbLoading.value = true
  try {
    const res = await api.startTalkback(tbDeviceId.value, {
      channel_id: tbChannelId.value,
      audio_port: tbAudioPort.value,
      codec:      tbCodec.value || null,
    })
    if (res.data.code === 200) {
      tbSession.value = res.data.data
      addHistory('发起对讲', true, `会话 ${res.data.data.call_id}`)
    }
  } catch (e) {
    tbMsg.value   = e.response?.data?.msg || '发起失败'
    tbMsgOk.value = false
    addHistory('发起对讲', false, tbMsg.value)
  } finally {
    tbLoading.value = false
  }
  setTimeout(() => tbMsg.value = '', 3000)
}

async function handleStopTalkback() {
  if (!tbSession.value) return
  try {
    await api.hangup(tbSession.value.call_id)
    addHistory('挂断对讲', true, `会话 ${tbSession.value.call_id}`)
    tbSession.value = null
  } catch (e) {
    tbMsg.value   = e.response?.data?.msg || '挂断失败'
    tbMsgOk.value = false
  }
  setTimeout(() => tbMsg.value = '', 3000)
}

// ── 历史 ──
const history = ref([])
function addHistory(type, ok, msg) {
  history.value.unshift({
    time: new Date().toLocaleTimeString(),
    type,
    ok,
    msg,
  })
  if (history.value.length > 30) history.value.pop()
}
</script>

<style scoped>
.card-title { font-size: 15px; font-weight: 600; margin-bottom: 8px; }
.desc { font-size: 13px; color: var(--text-muted); margin-bottom: 14px; }
.ctrl-form { display: flex; flex-wrap: wrap; gap: 14px; }
.form-row { display: flex; flex-direction: column; gap: 4px; }
.form-row label { font-size: 12px; color: var(--text-muted); }
.input {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; width: 200px;
}
.input:focus { border-color: var(--primary); }
.select {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; background: var(--card-bg);
}
.action-row { display: flex; gap: 10px; flex-wrap: wrap; }
.mt-8  { margin-top: 8px; }
.mt-12 { margin-top: 12px; }
.mt-16 { margin-top: 16px; }
.msg { margin-top: 10px; font-size: 13px; padding: 6px 10px; border-radius: 5px; }
.msg-ok  { background: #e8f5e9; color: #2e7d32; }
.msg-err { background: #ffebee; color: #c62828; }
.hint { font-size: 12px; color: var(--text-muted); margin-top: 6px; }

.status-badge {
  margin-top: 10px; display: inline-block;
  padding: 6px 12px; border-radius: 5px; font-size: 13px;
}
.status-active { background: #e3f2fd; color: #1565c0; }
.status-idle   { background: #f3f3f3; color: var(--text-muted); }

.session-info { padding: 12px; background: var(--bg); border-radius: var(--radius); border: 1px solid var(--border); }
.session-title { font-size: 13px; font-weight: 600; margin-bottom: 8px; }
.info-row { display: flex; gap: 8px; align-items: center; margin-bottom: 4px; }
.il { font-size: 12px; color: var(--text-muted); width: 90px; flex-shrink: 0; }
.iv { font-size: 13px; }
.mono { font-family: monospace; font-size: 12px; }

.history-list { display: flex; flex-direction: column; gap: 4px; }
.history-item { display: flex; align-items: center; gap: 10px; padding: 5px 0; border-bottom: 1px solid var(--border); font-size: 12px; }
.history-time { color: var(--text-muted); width: 70px; flex-shrink: 0; }
.history-type { font-weight: 600; width: 60px; flex-shrink: 0; }
.history-type.ok  { color: #2e7d32; }
.history-type.err { color: #c62828; }
.history-msg { color: var(--text-muted); flex: 1; }
</style>
