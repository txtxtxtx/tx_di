<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>报警管理</h1>
        <p>共 {{ total }} 条报警，未处理 {{ pendingCount }} 条</p>
      </div>
      <div class="flex gap-8">
        <button class="btn btn-primary" @click="refresh">刷新</button>
      </div>
    </div>

    <!-- 筛选栏 -->
    <div class="card filter-bar">
      <select v-model="filterStatus" class="select" @change="fetchAlarms">
        <option value="">全部状态</option>
        <option value="0">未处理</option>
        <option value="1">已确认</option>
        <option value="2">已处理</option>
      </select>
      <input v-model="filterDevice" class="search-input" placeholder="设备 ID 筛选..." @keyup.enter="fetchAlarms" />
      <select v-model="filterType" class="select" @change="fetchAlarms">
        <option value="">全部类型</option>
        <option value="131">设备配置参数异常</option>
        <option value="132">设备外壳破坏</option>
        <option value="133">存储介质异常</option>
        <option value="134">设备电源异常</option>
        <option value="135">视频信号异常</option>
        <option value="136">PIR 报警</option>
        <option value="137">遮挡报警</option>
        <option value="138">非法访问</option>
      </select>
      <button class="btn btn-sm" @click="fetchAlarms">搜索</button>
    </div>

    <!-- 报警列表 -->
    <div class="card mt-16">
      <div v-if="loading" class="loading-center">
        <span class="spinner"></span>
      </div>
      <div v-else-if="alarms.length === 0" class="empty-state">
        <div class="icon">🔔</div>
        <p>暂无报警记录</p>
      </div>
      <div v-else class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>状态</th>
              <th>报警时间</th>
              <th>设备 ID</th>
              <th>通道 ID</th>
              <th>类型</th>
              <th>级别</th>
              <th>描述</th>
              <th>处理人</th>
              <th style="text-align:right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in alarms" :key="a.id">
              <td>
                <span :class="['badge', statusClass(a.status)]">{{ statusLabel(a.status) }}</span>
              </td>
              <td class="mono text-muted">{{ a.alarm_time }}</td>
              <td>
                <RouterLink :to="`/devices/${a.device_id}`" class="device-link">{{ a.device_id }}</RouterLink>
              </td>
              <td class="mono text-sm">{{ a.channel_id || '—' }}</td>
              <td>{{ a.alarm_type || '—' }}</td>
              <td>
                <span :class="['badge', levelClass(a.alarm_level)]">{{ a.alarm_level }}</span>
              </td>
              <td>{{ a.description || '—' }}</td>
              <td>{{ a.handler || '—' }}</td>
              <td>
                <div class="flex gap-6" style="justify-content:flex-end">
                  <button
                    v-if="a.status === 0"
                    class="btn btn-sm btn-primary"
                    @click="openHandle(a, 1)"
                  >确认</button>
                  <button
                    v-if="a.status < 2"
                    class="btn btn-sm"
                    @click="openHandle(a, 2)"
                  >处理</button>
                  <button class="btn btn-sm" @click="openHandle(a, a.status)">备注</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- 分页 -->
      <div class="pagination" v-if="total > pageSize">
        <button class="btn btn-sm" :disabled="page <= 1" @click="changePage(page - 1)">上一页</button>
        <span class="page-info">{{ page }} / {{ totalPages }}</span>
        <button class="btn btn-sm" :disabled="page >= totalPages" @click="changePage(page + 1)">下一页</button>
      </div>
    </div>

    <!-- 报警订阅配置区 -->
    <div class="card mt-16">
      <h2 class="card-title">报警订阅</h2>
      <div class="subscribe-form">
        <div class="form-row">
          <label>设备 ID</label>
          <input v-model="subDeviceId" class="input" placeholder="输入设备 ID" />
        </div>
        <div class="form-row">
          <label>报警类型</label>
          <select v-model="subAlarmType" class="select">
            <option :value="0">全部</option>
            <option :value="1">自定义</option>
          </select>
        </div>
        <div class="form-row">
          <label>有效期（秒）</label>
          <input v-model.number="subExpire" class="input" type="number" min="0" placeholder="0=永久" />
        </div>
        <div class="flex gap-8">
          <button class="btn btn-primary" @click="handleSubscribe" :disabled="!subDeviceId">订阅报警</button>
          <button class="btn" @click="handleAlarmReset" :disabled="!subDeviceId">报警复位</button>
        </div>
      </div>
      <div v-if="subMsg" class="msg" :class="subMsgOk ? 'msg-ok' : 'msg-err'">{{ subMsg }}</div>
    </div>

    <!-- 处理弹窗 -->
    <div v-if="handleDialog" class="modal-mask" @click.self="handleDialog = null">
      <div class="modal card">
        <h3 style="margin-bottom:14px">处理报警 #{{ handleDialog.id }}</h3>
        <div class="form-row">
          <label>处理状态</label>
          <select v-model="handleStatus" class="select">
            <option :value="1">已确认</option>
            <option :value="2">已处理</option>
          </select>
        </div>
        <div class="form-row">
          <label>处理人</label>
          <input v-model="handleUser" class="input" placeholder="处理人姓名" />
        </div>
        <div class="form-row">
          <label>备注</label>
          <textarea v-model="handleRemark" class="textarea" rows="3" placeholder="处理说明..."></textarea>
        </div>
        <div style="text-align:right;margin-top:16px" class="flex gap-8" style="justify-content:flex-end">
          <button class="btn btn-primary" @click="submitHandle" :disabled="handleLoading">
            <span v-if="handleLoading" class="spinner" style="width:12px;height:12px;border-width:2px"></span>
            提交
          </button>
          <button class="btn" @click="handleDialog = null">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import api from '../api/index.js'

const alarms = ref([])
const loading = ref(false)
const total   = ref(0)
const page    = ref(1)
const pageSize = 20

const filterStatus = ref('')
const filterDevice = ref('')
const filterType   = ref('')

const pendingCount = ref(0)

// 订阅区
const subDeviceId  = ref('')
const subAlarmType = ref(0)
const subExpire    = ref(3600)
const subMsg       = ref('')
const subMsgOk     = ref(true)

// 处理弹窗
const handleDialog = ref(null)
const handleStatus = ref(1)
const handleUser   = ref('')
const handleRemark = ref('')
const handleLoading = ref(false)

const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize)))

function statusClass(s) {
  if (s === 0) return 'badge-danger'
  if (s === 1) return 'badge-warning'
  return 'badge-online'
}
function statusLabel(s) {
  return ['未处理', '已确认', '已处理'][s] ?? '未知'
}
function levelClass(l) {
  if (l <= 1) return 'badge-danger'
  if (l <= 2) return 'badge-warning'
  return 'badge-session'
}

async function fetchAlarms() {
  loading.value = true
  try {
    const params = { page: page.value, page_size: pageSize }
    if (filterStatus.value !== '') params.status = Number(filterStatus.value)
    if (filterDevice.value.trim()) params.device_id = filterDevice.value.trim()
    if (filterType.value)          params.alarm_type = filterType.value
    const res = await api.alarms(params)
    if (res.data.code === 200) {
      alarms.value = res.data.data.items || []
      total.value  = res.data.data.total || 0
    }
  } finally {
    loading.value = false
  }
}

async function fetchPending() {
  try {
    const res = await api.alarms({ status: 0, page: 1, page_size: 1 })
    if (res.data.code === 200) pendingCount.value = res.data.data.total || 0
  } catch {}
}

async function refresh() {
  await Promise.all([fetchAlarms(), fetchPending()])
}

function changePage(p) {
  page.value = p
  fetchAlarms()
}

async function handleSubscribe() {
  try {
    await api.alarmSubscribe(subDeviceId.value, subAlarmType.value, subExpire.value)
    subMsg.value = '订阅指令已发送'
    subMsgOk.value = true
  } catch (e) {
    subMsg.value = e.response?.data?.msg || '发送失败'
    subMsgOk.value = false
  }
  setTimeout(() => subMsg.value = '', 3000)
}

async function handleAlarmReset() {
  try {
    await api.alarmResetDev(subDeviceId.value)
    subMsg.value = '报警复位已发送'
    subMsgOk.value = true
  } catch (e) {
    subMsg.value = e.response?.data?.msg || '发送失败'
    subMsgOk.value = false
  }
  setTimeout(() => subMsg.value = '', 3000)
}

function openHandle(alarm, defaultStatus) {
  handleDialog.value = alarm
  handleStatus.value = defaultStatus
  handleUser.value   = ''
  handleRemark.value = alarm.handle_remark || ''
}

async function submitHandle() {
  handleLoading.value = true
  try {
    await api.handleAlarm(handleDialog.value.id, {
      status: handleStatus.value,
      handler: handleUser.value,
      handle_remark: handleRemark.value,
    })
    handleDialog.value = null
    await refresh()
  } finally {
    handleLoading.value = false
  }
}

onMounted(refresh)
</script>

<style scoped>
.filter-bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; padding: 12px 16px; }
.search-input {
  width: 200px; padding: 7px 12px; border: 1px solid var(--border);
  border-radius: 6px; font-size: 13px; outline: none;
}
.search-input:focus { border-color: var(--primary); }
.select {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; background: var(--card-bg);
}
.card-title { font-size: 15px; font-weight: 600; margin-bottom: 14px; }

/* 订阅表单 */
.subscribe-form { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; }
.form-row { display: flex; flex-direction: column; gap: 4px; }
.form-row label { font-size: 12px; color: var(--text-muted); }
.input {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; width: 180px;
}
.input:focus { border-color: var(--primary); }
.textarea {
  padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
  font-size: 13px; outline: none; resize: vertical; width: 100%;
}
.msg { margin-top: 10px; font-size: 13px; padding: 6px 10px; border-radius: 5px; }
.msg-ok  { background: #e8f5e9; color: #2e7d32; }
.msg-err { background: #ffebee; color: #c62828; }

/* 分页 */
.pagination { display: flex; align-items: center; gap: 10px; padding: 12px 0 0; justify-content: center; }
.page-info { font-size: 13px; color: var(--text-muted); }

/* 弹窗 */
.modal-mask {
  position: fixed; inset: 0; background: rgba(0,0,0,.35);
  display: flex; align-items: center; justify-content: center; z-index: 100;
}
.modal { min-width: 400px; max-width: 520px; }

.device-link { color: var(--primary); font-weight: 500; }
.device-link:hover { text-decoration: underline; }
.mono { font-family: monospace; }
.text-muted { color: var(--text-muted); font-size: 12px; }
.text-sm { font-size: 12px; }

.badge-danger { background: #ffebee; color: #c62828; }
.badge-warning { background: #fff8e1; color: #ef6c00; }
.loading-center { display: flex; justify-content: center; padding: 40px; }
</style>
