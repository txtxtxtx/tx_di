<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="用户ID">
          <el-input v-model="query.userId" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="日志类型">
          <el-input v-model="query.logType" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="是否成功">
          <el-select v-model="query.success" placeholder="全部" clearable>
            <el-option label="成功" :value="0" />
            <el-option label="失败" :value="1" />
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
          <span>操作日志</span>
          <div>
            <el-button type="danger" :disabled="!selectedIds.length" @click="handleDeleteSelected">批量删除</el-button>
            <el-button type="warning" @click="handleClean">清空</el-button>
          </div>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe @selection-change="onSelectionChange">
        <el-table-column type="selection" width="50" />
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="traceId" label="追踪ID" width="180" show-overflow-tooltip />
        <el-table-column prop="userId" label="用户ID" width="80" />
        <el-table-column prop="logType" label="日志类型" width="100" />
        <el-table-column prop="subType" label="子类型" width="100" />
        <el-table-column prop="action" label="操作" min-width="150" show-overflow-tooltip />
        <el-table-column prop="success" label="结果" width="80">
          <template #default="{ row }">
            <el-tag :type="row.success === 0 ? 'success' : 'danger'">{{ row.success === 0 ? '成功' : '失败' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="requestMethod" label="请求方法" width="80" />
        <el-table-column prop="requestUrl" label="请求URL" width="200" show-overflow-tooltip />
        <el-table-column prop="userIp" label="IP" width="130" />
      </el-table>

      <div class="pagination-wrap">
        <el-pagination v-model:current-page="page" v-model:page-size="size" :total="total" :page-sizes="[10, 20, 50]" layout="total, sizes, prev, pager, next" @current-change="loadData" @size-change="loadData" />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listOperateLogs, deleteOperateLogs, cleanOperateLogs } from '@/api/log'
import { toPageData } from '@/utils'
import type { OperateLogResponse } from '@/types'

const loading = ref(false)
const tableData = ref<OperateLogResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ userId: undefined as string | undefined, logType: '', success: undefined as number | undefined })
const selectedIds = ref<string[]>([])

async function loadData() {
  loading.value = true
  try {
    const res = await listOperateLogs({ userId: query.userId || undefined, logType: query.logType || undefined, success: query.success, page: page.value, pageSize: size.value })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.userId = undefined; query.logType = ''; query.success = undefined; page.value = 1; loadData() }

function onSelectionChange(rows: OperateLogResponse[]) {
  selectedIds.value = rows.map(r => r.id)
}

async function handleDeleteSelected() {
  await ElMessageBox.confirm(`确认删除 ${selectedIds.value.length} 条日志吗？`, '提示', { type: 'warning' })
  await deleteOperateLogs({ ids: selectedIds.value })
  ElMessage.success('删除成功'); loadData()
}

async function handleClean() {
  await ElMessageBox.confirm('确认清空所有操作日志吗？此操作不可恢复！', '警告', { type: 'warning' })
  await cleanOperateLogs()
  ElMessage.success('清空成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
