<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="任务ID">
          <el-input v-model="query.jobId" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option label="失败" :value="0" />
            <el-option label="成功" :value="1" />
            <el-option label="超时" :value="2" />
            <el-option label="重试中" :value="3" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="loadData">搜索</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>执行日志</span>
          <el-button type="danger" @click="handleClean">清空日志</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="日志编号" width="100" show-overflow-tooltip />
        <el-table-column prop="jobId" label="任务ID" width="100" show-overflow-tooltip />
        <el-table-column prop="handlerName" label="处理器" min-width="160" show-overflow-tooltip />
        <el-table-column prop="executeIndex" label="执行次数" width="90" align="center" />
        <el-table-column label="开始时间" width="180">
          <template #default="{ row }">{{ formatTime(row.beginTime) }}</template>
        </el-table-column>
        <el-table-column label="结束时间" width="180">
          <template #default="{ row }">{{ formatTime(row.endTime) }}</template>
        </el-table-column>
        <el-table-column label="执行时长" width="100" align="center">
          <template #default="{ row }">{{ formatDuration(row.duration) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="result" label="结果" max-width="200" show-overflow-tooltip />
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="handleDetail(row)">查看详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrap">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="size"
          :total="total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @current-change="loadData"
          @size-change="loadData"
        />
      </div>
    </el-card>

    <!-- 日志详情对话框 -->
    <el-dialog v-model="detailVisible" title="日志详情" width="600px" destroy-on-close>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="日志编号">{{ detail.id }}</el-descriptions-item>
        <el-descriptions-item label="任务ID">{{ detail.jobId }}</el-descriptions-item>
        <el-descriptions-item label="处理器">{{ detail.handlerName }}</el-descriptions-item>
        <el-descriptions-item label="处理器参数">{{ detail.handlerParam || '-' }}</el-descriptions-item>
        <el-descriptions-item label="执行次数">{{ detail.executeIndex }}</el-descriptions-item>
        <el-descriptions-item label="开始时间">{{ formatTime(detail.beginTime) }}</el-descriptions-item>
        <el-descriptions-item label="结束时间">{{ formatTime(detail.endTime) }}</el-descriptions-item>
        <el-descriptions-item label="执行时长">{{ formatDuration(detail.duration) }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagType(detail.status)">{{ statusLabel(detail.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="结果">
          <pre v-if="detail.result && detail.result.length > 100" class="result-pre">{{ detail.result }}</pre>
          <span v-else>{{ detail.result || '-' }}</span>
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listJobLogs, getJobLog, cleanJobLogs } from '@/api/job'
import { toPageData } from '@/utils'
import type { JobLogResponse } from '@/types'

const loading = ref(false)
const tableData = ref<JobLogResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ jobId: '', status: undefined as number | undefined })

const detailVisible = ref(false)
const detail = reactive<JobLogResponse>({
  id: '',
  jobId: '',
  handlerName: '',
  handlerParam: null,
  executeIndex: 0,
  beginTime: '',
  endTime: null,
  duration: null,
  status: 0,
  result: null,
})

function statusLabel(status: number): string {
  const map: Record<number, string> = { 0: '失败', 1: '成功', 2: '超时', 3: '重试中' }
  return map[status] ?? '未知'
}

function statusTagType(status: number): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<number, '' | 'success' | 'warning' | 'danger' | 'info'> = { 0: 'danger', 1: 'success', 2: 'warning', 3: 'info' }
  return map[status] ?? ''
}

function formatDuration(ms: number | null): string {
  if (ms == null) return '-'
  if (ms >= 1000) return (ms / 1000).toFixed(1) + 's'
  return ms + 'ms'
}

/** 将 Unix 毫秒时间戳格式化为 yyyy-MM-dd HH:mm:ss */
function formatTime(ts: string | null): string {
  if (ts == null || ts === '') return '-'
  const d = new Date(Number(ts))
  if (isNaN(d.getTime())) return '-'
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

async function loadData() {
  loading.value = true
  try {
    const res = await listJobLogs({
      jobId: query.jobId || undefined,
      status: query.status,
      page: page.value,
      pageSize: size.value,
    })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {
    /* 错误由拦截器统一处理 */
  } finally {
    loading.value = false
  }
}

function resetQuery() {
  query.jobId = ''
  query.status = undefined
  page.value = 1
  loadData()
}

async function handleDetail(row: JobLogResponse) {
  try {
    const res = await getJobLog(row.id)
    Object.assign(detail, res.data)
    detailVisible.value = true
  } catch {
    /* 错误由拦截器统一处理 */
  }
}

async function handleClean() {
  await ElMessageBox.confirm('确认清空所有执行日志吗？此操作不可恢复。', '提示', {
    type: 'warning',
  })
  await cleanJobLogs()
  ElMessage.success('清空成功')
  loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card {
  margin-bottom: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.pagination-wrap {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
.result-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
  font-size: 12px;
  line-height: 1.5;
}
</style>
