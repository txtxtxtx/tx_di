<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="任务名称">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in dictToOptions(dictStore.dictMap['sys_job_status'] || [])" :key="o.value" :label="o.label" :value="o.value" />
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
          <span>任务列表</span>
          <el-button type="primary" @click="openDialog()">新增任务</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="name" label="任务名称" min-width="160" show-overflow-tooltip />
        <el-table-column prop="handlerName" label="处理器" width="140" show-overflow-tooltip />
        <el-table-column prop="cronExpression" label="CRON 表达式" width="160" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="dictColorType(dictStore.dictMap['sys_job_status'] || [], row.status) as any">
              {{ dictLabel(dictStore.dictMap['sys_job_status'] || [], row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="retryCount" label="重试次数" width="100" />
        <el-table-column prop="retryInterval" label="重试间隔" width="100">
          <template #default="{ row }">{{ row.retryInterval }}s</template>
        </el-table-column>
        <el-table-column prop="monitorTimeout" label="超时时间" width="100">
          <template #default="{ row }">{{ row.monitorTimeout > 0 ? row.monitorTimeout + 's' : '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="primary" size="small" @click="handleRun(row)">手动执行</el-button>
            <el-button link :type="row.status === 1 ? 'warning' : 'success'" size="small" @click="handleToggleStatus(row)">
              {{ row.status === 1 ? '暂停' : '启动' }}
            </el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
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

    <!-- 新增/编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑任务' : '新增任务'" width="600px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="任务名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="处理器名称" prop="handlerName">
          <el-input v-model="form.handlerName" placeholder="如 noop、echo、http_request" />
        </el-form-item>
        <el-form-item label="处理器参数">
          <el-input v-model="form.handlerParam" type="textarea" :rows="3" placeholder="JSON 参数" />
        </el-form-item>
        <el-form-item label="CRON 表达式" prop="cronExpression">
          <el-input v-model="form.cronExpression" placeholder="如 0 */5 * * * *" />
        </el-form-item>
        <el-form-item label="重试次数">
          <el-input-number v-model="form.retryCount" :min="0" />
        </el-form-item>
        <el-form-item label="重试间隔">
          <el-input-number v-model="form.retryInterval" :min="0" />
          <span style="margin-left: 8px">秒</span>
        </el-form-item>
        <el-form-item label="超时时间">
          <el-input-number v-model="form.monitorTimeout" :min="0" />
          <span style="margin-left: 8px">秒</span>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listJobs, createJob, updateJob, deleteJob, changeJobStatus, runJob } from '@/api/job'
import { toPageData, dictToOptions, dictLabel, dictColorType } from '@/utils'
import { useDictStore } from '@/stores/dict'
import type { JobResponse } from '@/types'

const dictStore = useDictStore()

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<JobResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ name: '', status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref('')
const formRef = ref<FormInstance>()
const form = reactive({
  name: '',
  handlerName: '',
  handlerParam: '',
  cronExpression: '',
  retryCount: 0,
  retryInterval: 0,
  monitorTimeout: 0,
})
const formRules: FormRules = {
  name: [{ required: true, message: '请输入任务名称', trigger: 'blur' }],
  handlerName: [{ required: true, message: '请输入处理器名称', trigger: 'blur' }],
  cronExpression: [{ required: true, message: '请输入 CRON 表达式', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listJobs({
      name: query.name || undefined,
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
  query.name = ''
  query.status = undefined
  page.value = 1
  loadData()
}

function openDialog(row?: JobResponse) {
  isEdit.value = !!row
  if (row) {
    editingId.value = row.id
    Object.assign(form, {
      name: row.name,
      handlerName: row.handlerName,
      handlerParam: row.handlerParam || '',
      cronExpression: row.cronExpression,
      retryCount: row.retryCount,
      retryInterval: row.retryInterval,
      monitorTimeout: row.monitorTimeout,
    })
  } else {
    editingId.value = ''
    Object.assign(form, {
      name: '',
      handlerName: '',
      handlerParam: '',
      cronExpression: '',
      retryCount: 0,
      retryInterval: 0,
      monitorTimeout: 0,
    })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    const data = {
      name: form.name,
      handlerName: form.handlerName,
      handlerParam: form.handlerParam || undefined,
      cronExpression: form.cronExpression,
      retryCount: form.retryCount,
      retryInterval: form.retryInterval,
      monitorTimeout: form.monitorTimeout,
    }
    if (isEdit.value) {
      await updateJob(editingId.value, data)
      ElMessage.success('更新成功')
    } else {
      await createJob(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadData()
  } catch {
    /* 错误由拦截器统一处理 */
  } finally {
    submitLoading.value = false
  }
}

async function handleRun(row: JobResponse) {
  await runJob(row.id)
  ElMessage.success('执行成功')
}

async function handleToggleStatus(row: JobResponse) {
  const newStatus = row.status === 1 ? 0 : 1
  const label = newStatus === 1 ? '启动' : '暂停'
  await changeJobStatus(row.id, newStatus)
  ElMessage.success(`${label}成功`)
  loadData()
}

async function handleDelete(row: JobResponse) {
  await ElMessageBox.confirm(`确认删除任务 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteJob(row.id)
  ElMessage.success('删除成功')
  loadData()
}

onMounted(async () => {
  await dictStore.getDictData('sys_job_status')
  loadData()
})
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
</style>
