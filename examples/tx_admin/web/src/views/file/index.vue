<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="文件名">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="文件类型">
          <el-input v-model="query.fileType" placeholder="请输入" clearable />
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
          <span>文件列表</span>
          <el-button type="primary" @click="uploadDialogVisible = true">上传文件</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="name" label="文件名" min-width="200" show-overflow-tooltip />
        <el-table-column prop="fileType" label="类型" width="100" />
        <el-table-column prop="size" label="大小" width="100">
          <template #default="{ row }">{{ formatBytes(row.size) }}</template>
        </el-table-column>
        <el-table-column prop="url" label="URL" min-width="250" show-overflow-tooltip />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="handleDownload(row)">下载</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrap">
        <el-pagination v-model:current-page="page" v-model:page-size="size" :total="total" :page-sizes="[10, 20, 50]" layout="total, sizes, prev, pager, next" @current-change="loadData" @size-change="loadData" />
      </div>
    </el-card>

    <!-- 上传文件对话框 -->
    <el-dialog v-model="uploadDialogVisible" title="上传文件" width="500px" destroy-on-close>
      <el-form ref="uploadFormRef" :model="uploadForm" :rules="uploadRules" label-width="80px">
        <el-form-item label="文件名" prop="name">
          <el-input v-model="uploadForm.name" />
        </el-form-item>
        <el-form-item label="路径" prop="path">
          <el-input v-model="uploadForm.path" />
        </el-form-item>
        <el-form-item label="URL" prop="url">
          <el-input v-model="uploadForm.url" />
        </el-form-item>
        <el-form-item label="文件类型">
          <el-input v-model="uploadForm.fileType" />
        </el-form-item>
        <el-form-item label="大小" prop="size">
          <el-input-number v-model="uploadForm.size" :min="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="uploadDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleUpload">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listFiles, uploadFile, deleteFile, downloadFile } from '@/api/file'
import { formatBytes, toPageData } from '@/utils'
import type { FileResponse } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<FileResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ name: '', fileType: '' })

const uploadDialogVisible = ref(false)
const uploadFormRef = ref<FormInstance>()
const uploadForm = reactive({ name: '', path: '', url: '', fileType: '', size: 0 })
const uploadRules: FormRules = {
  name: [{ required: true, message: '请输入文件名', trigger: 'blur' }],
  path: [{ required: true, message: '请输入路径', trigger: 'blur' }],
  url: [{ required: true, message: '请输入URL', trigger: 'blur' }],
  size: [{ required: true, message: '请输入大小', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listFiles({ name: query.name || undefined, fileType: query.fileType || undefined, page: page.value, pageSize: size.value })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.name = ''; query.fileType = ''; page.value = 1; loadData() }

async function handleUpload() {
  const valid = await uploadFormRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    await uploadFile(uploadForm)
    ElMessage.success('上传成功')
    uploadDialogVisible.value = false
    Object.assign(uploadForm, { name: '', path: '', url: '', fileType: '', size: 0 })
    loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDownload(row: FileResponse) {
  try {
    const res = await downloadFile(row.id)
    window.open(res.data.url, '_blank')
  } catch {}
}

async function handleDelete(row: FileResponse) {
  await ElMessageBox.confirm(`确认删除文件 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteFile(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
