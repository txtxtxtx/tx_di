<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>审计日志</h1>
        <p>管理员操作记录（共 {{ total }} 条）</p>
      </div>
      <button class="btn" @click="refresh">刷新</button>
    </div>

    <!-- 筛选栏 -->
    <div class="card filter-bar">
      <input v-model="params.operator" class="filter-input" placeholder="操作人" />
      <input v-model="params.action"   class="filter-input" placeholder="操作类型" />
      <select v-model="params.result" class="select">
        <option value="">全部结果</option>
        <option value="ok">成功</option>
        <option value="fail">失败</option>
      </select>
      <button class="btn btn-sm btn-primary" @click="() => { params.page = 1; refresh() }">筛选</button>
      <button class="btn btn-sm" @click="resetParams">重置</button>
      <div style="margin-left:auto;font-size:13px;color:var(--text-muted)">
        第 {{ params.page }} / {{ totalPages }} 页
      </div>
    </div>

    <!-- 日志表格 -->
    <div class="card mt-16">
      <div v-if="logs.length === 0 && !loading" class="empty-state">
        <div class="icon">📝</div>
        <p>暂无审计日志</p>
      </div>
      <div v-else class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>操作人</th>
              <th>操作类型</th>
              <th>目标</th>
              <th>详情</th>
              <th>客户端 IP</th>
              <th>结果</th>
              <th>时间</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in logs" :key="l.id">
              <td class="mono">{{ l.id }}</td>
              <td>{{ l.operator }}</td>
              <td><span class="action-badge">{{ l.action }}</span></td>
              <td class="mono" style="max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ l.target }}</td>
              <td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" :title="l.detail">{{ l.detail }}</td>
              <td class="text-muted">{{ l.client_ip || '—' }}</td>
              <td>
                <span :class="['badge', l.result === 'ok' ? 'badge-online' : 'badge-offline']">
                  {{ l.result === 'ok' ? '成功' : '失败' }}
                </span>
              </td>
              <td class="text-muted">{{ l.created_at }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 分页 -->
    <div class="pagination mt-16" v-if="totalPages > 1">
      <button class="btn btn-sm" :disabled="params.page <= 1"  @click="goPage(params.page - 1)">‹ 上一页</button>
      <span style="font-size:13px;color:var(--text);margin:0 8px">
        {{ params.page }} / {{ totalPages }}
      </span>
      <button class="btn btn-sm" :disabled="params.page >= totalPages" @click="goPage(params.page + 1)">下一页 ›</button>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import api from '../api/index.js'

const logs       = ref([])
const loading    = ref(false)
const total      = ref(0)
const totalPages = ref(0)

const params = reactive({
  operator: '',
  action:   '',
  result:   '',
  page:     1,
  page_size: 20,
})

async function refresh() {
  loading.value = true
  try {
    // 把空字符串转成 undefined（不传）
    const q = {}
    if (params.operator.trim()) q.operator = params.operator.trim()
    if (params.action.trim())   q.action   = params.action.trim()
    if (params.result)          q.result   = params.result
    q.page      = params.page
    q.page_size = params.page_size

    const res = await api.auditLogs(q)
    if (res.data.code === 200) {
      const d = res.data.data
      logs.value   = d.items || []
      total.value  = d.total || 0
      totalPages.value = d.total_pages || 0
    }
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { loading.value = false }
}

function resetParams() {
  params.operator = ''
  params.action   = ''
  params.result   = ''
  params.page     = 1
  refresh()
}
function goPage(p) {
  if (p < 1 || p > totalPages.value) return
  params.page = p
  refresh()
}

onMounted(refresh)
</script>

<style scoped>
.filter-bar { display:flex; align-items:center; gap:8px; padding:12px 16px; flex-wrap:wrap; }
.filter-input {
  padding:6px 10px; border:1px solid var(--border); border-radius:6px;
  font-size:13px; outline:none; width:140px;
}
.filter-input:focus { border-color: var(--primary); }
.select {
  border:1px solid var(--border); border-radius:6px;
  padding:6px 10px; font-size:13px; outline:none;
}
.mono { font-family:monospace; font-size:12px; }
.text-muted { color:var(--text-muted); font-size:12px; }
.action-badge {
  display:inline-block; padding:1px 7px; border-radius:4px;
  font-size:11px; font-weight:600; background:#f1f5f9; color:var(--primary);
}

/* 分页 */
.pagination { display:flex; align-items:center; justify-content:center; gap:8px; }
</style>
