<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="用户名">
          <el-input v-model="query.username" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="登录IP">
          <el-input v-model="query.loginIp" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="登录结果">
          <el-select v-model="query.result" placeholder="全部" clearable>
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
          <span>登录日志</span>
          <div>
            <el-button type="danger" :disabled="!selectedIds.length" @click="handleDeleteSelected">批量删除</el-button>
            <el-button type="warning" @click="handleClean">清空</el-button>
          </div>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe @selection-change="onSelectionChange">
        <el-table-column type="selection" width="50" />
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="userId" label="用户ID" width="80" />
        <el-table-column prop="username" label="用户名" width="120" />
        <el-table-column prop="loginIp" label="登录IP" width="140" />
        <el-table-column prop="loginType" label="登录方式" width="100" />
        <el-table-column prop="result" label="结果" width="80">
          <template #default="{ row }">
            <el-tag :type="row.result === 0 ? 'success' : 'danger'">{{ row.result === 0 ? '成功' : '失败' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="msg" label="消息" min-width="200" show-overflow-tooltip />
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
import { listLoginLogs, deleteLoginLogs, cleanLoginLogs } from '@/api/log'
import type { LoginLogResponse } from '@/types'

const loading = ref(false)
const tableData = ref<LoginLogResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ username: '', loginIp: '', result: undefined as number | undefined })
const selectedIds = ref<number[]>([])

async function loadData() {
  loading.value = true
  try {
    const res = await listLoginLogs({ username: query.username || undefined, loginIp: query.loginIp || undefined, result: query.result, page: page.value, pageSize: size.value })
    tableData.value = res.data.list
    total.value = res.data.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.username = ''; query.loginIp = ''; query.result = undefined; page.value = 1; loadData() }

function onSelectionChange(rows: LoginLogResponse[]) {
  selectedIds.value = rows.map(r => r.id)
}

async function handleDeleteSelected() {
  await ElMessageBox.confirm(`确认删除 ${selectedIds.value.length} 条日志吗？`, '提示', { type: 'warning' })
  await deleteLoginLogs({ ids: selectedIds.value })
  ElMessage.success('删除成功'); loadData()
}

async function handleClean() {
  await ElMessageBox.confirm('确认清空所有登录日志吗？此操作不可恢复！', '警告', { type: 'warning' })
  await cleanLoginLogs()
  ElMessage.success('清空成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
