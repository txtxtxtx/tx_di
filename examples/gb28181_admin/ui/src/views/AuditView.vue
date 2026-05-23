<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>注册审核</h1>
        <p>管理设备注册请求（{{ pendingCount }} 条待审核）</p>
      </div>
      <div class="flex gap-8">
        <button class="btn btn-primary" @click="handleAutoApprove" :disabled="autoLoading">
          <span v-if="autoLoading" class="spinner" style="width:12px;height:12px;border-width:2px;margin-right:4px"></span>
          一键自动通过
        </button>
        <button class="btn" @click="refresh">刷新</button>
      </div>
    </div>

    <!-- 筛选栏 -->
    <div class="card filter-bar">
      <select v-model="filterStatus" class="select">
        <option value="">全部状态</option>
        <option value="pending">待审核</option>
        <option value="approved">已通过</option>
        <option value="rejected">已拒绝</option>
      </select>
      <input v-model="search" class="search-input" placeholder="搜索设备 ID / IP..." />
    </div>

    <!-- 审核列表 -->
    <div class="card mt-16">
      <div v-if="filteredList.length === 0" class="empty-state">
        <div class="icon">📋</div>
        <p>{{ loading ? '加载中...' : '暂无审核记录' }}</p>
      </div>
      <div v-else class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>状态</th>
              <th>设备 ID</th>
              <th>IP 地址</th>
              <th>注册时间</th>
              <th>审核人</th>
              <th>审核时间</th>
              <th>拒绝原因</th>
              <th style="text-align:right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in filteredList" :key="a.id">
              <td>
                <span :class="['badge', statusClass(a.status)]">{{ statusText(a.status) }}</span>
              </td>
              <td><code class="mono">{{ a.device_id }}</code></td>
              <td>{{ a.ip_addr || '—' }}</td>
              <td class="text-muted">{{ a.registered_at }}</td>
              <td class="text-muted">{{ a.reviewer || '—' }}</td>
              <td class="text-muted">{{ a.reviewed_at || '—' }}</td>
              <td class="text-muted" style="max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ a.reason || '—' }}</td>
              <td>
                <div style="display:flex;justify-content:flex-end;gap:4px">
                  <button
                    v-if="a.status === 'pending'"
                    class="btn btn-sm btn-primary"
                    @click="handleApprove(a.id)"
                    :disabled="actionLoading[a.id]"
                  >通过</button>
                  <button
                    v-if="a.status === 'pending'"
                    class="btn btn-sm btn-danger"
                    @click="openRejectModal(a.id)"
                    :disabled="actionLoading[a.id]"
                  >拒绝</button>
                  <button
                    v-if="a.status !== 'pending'"
                    class="btn btn-sm"
                    @click="handleDelete(a.id)"
                  >删除</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 拒绝原因弹窗 -->
    <Teleport to="body">
      <div v-if="showRejectModal" class="modal-mask" @click.self="showRejectModal = false">
        <div class="modal-box">
          <h3>拒绝注册申请</h3>
          <div class="form-group">
            <label>拒绝原因</label>
            <textarea v-model="rejectReason" class="input" rows="3" placeholder="请输入拒绝原因（可选）"></textarea>
          </div>
          <div class="modal-actions">
            <button class="btn" @click="showRejectModal = false">取消</button>
            <button class="btn btn-danger" @click="handleReject" :disabled="rejectLoading">
              <span v-if="rejectLoading" class="spinner" style="width:12px;height:12px;border-width:2px;margin-right:4px"></span>
              确认拒绝
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import api from '../api/index.js'

const audits       = ref([])
const loading      = ref(false)
const search       = ref('')
const filterStatus = ref('')
const actionLoading = ref({})
const showRejectModal = ref(false)
const rejectReason    = ref('')
const rejectLoading   = ref(false)
const autoLoading     = ref(false)
let   rejectId        = null

const pendingCount = computed(() => audits.value.filter(a => a.status === 'pending').length)
const filteredList = computed(() => {
  let list = audits.value
  if (filterStatus.value) list = list.filter(a => a.status === filterStatus.value)
  if (search.value.trim()) {
    const q = search.value.toLowerCase()
    list = list.filter(a =>
      a.device_id.toLowerCase().includes(q) ||
      (a.ip_addr || '').toLowerCase().includes(q)
    )
  }
  return list
})

function statusText(s) { return { pending: '待审核', approved: '已通过', rejected: '已拒绝' }[s] || s }
function statusClass(s) {
  return { pending: 'badge-unknown', approved: 'badge-online', rejected: 'badge-offline' }[s] || ''
}

async function refresh() {
  loading.value = true
  try {
    const res = await api.auditList()
    if (res.data.code === 200) audits.value = res.data.data
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { loading.value = false }
}

async function handleApprove(id) {
  actionLoading.value[id] = true
  try {
    const res = await api.auditApprove(id)
    if (res.data.code === 200 || res.data.code === 0) {
      await refresh()
    } else { alert(res.data.message || '操作失败') }
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { actionLoading.value[id] = false }
}

function openRejectModal(id) { rejectId = id; rejectReason.value = ''; showRejectModal.value = true }
async function handleReject() {
  rejectLoading.value = true
  try {
    const res = await api.auditReject(rejectId, rejectReason.value)
    if (res.data.code === 200 || res.data.code === 0) {
      showRejectModal.value = false
      await refresh()
    } else { alert(res.data.message || '操作失败') }
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { rejectLoading.value = false }
}

async function handleDelete(id) {
  if (!confirm('确定删除该审核记录？')) return
  try {
    const res = await api.auditDelete(id)
    if (res.data.code === 200 || res.data.code === 0) await refresh()
  } catch (e) { alert(e.response?.data?.message || e.message) }
}

async function handleAutoApprove() {
  if (!confirm('确定自动通过所有待审核申请？')) return
  autoLoading.value = true
  try {
    const res = await api.auditAutoApprove()
    if (res.data.code === 200 || res.data.code === 0) await refresh()
    else alert(res.data.message || '操作失败')
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { autoLoading.value = false }
}

onMounted(refresh)
</script>

<style scoped>
.select {
  border:1px solid var(--border); border-radius:6px;
  padding:6px 10px; font-size:13px; outline:none;
}
.search-input {
  flex:1; max-width:320px; padding:7px 12px; border:1px solid var(--border);
  border-radius:6px; font-size:13px; outline:none; transition:border-color .15s;
}
.search-input:focus { border-color: var(--primary); }
.mono { font-family:monospace; font-size:12px; }
.text-muted { color:var(--text-muted); font-size:12px; }

/* 弹窗 */
.modal-mask {
  position:fixed; inset:0; z-index:1000;
  background:rgba(0,0,0,.45);
  display:flex; align-items:center; justify-content:center;
}
.modal-box {
  background:var(--card-bg); border-radius:10px;
  padding:24px 28px; width:460px;
  box-shadow:0 8px 32px rgba(0,0,0,.18);
}
.modal-box h3 { font-size:16px; font-weight:600; margin-bottom:18px; }
.form-group { margin-bottom:14px; }
.form-group label { display:block; font-size:12px; color:var(--text-muted); margin-bottom:4px; }
.input {
  width:100%; padding:7px 10px; font-size:13px;
  border:1px solid var(--border); border-radius:6px;
  outline:none; font-family:inherit; resize:vertical;
}
.modal-actions { display:flex; justify-content:flex-end; gap:8px; margin-top:18px; }
</style>
