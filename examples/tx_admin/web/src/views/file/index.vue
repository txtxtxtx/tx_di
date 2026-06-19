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
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="name" label="文件名" min-width="200" show-overflow-tooltip />
        <el-table-column prop="fileType" label="类型" width="160" show-overflow-tooltip />
        <el-table-column prop="size" label="大小" width="100">
          <template #default="{ row }">{{ formatBytes(row.size) }}</template>
        </el-table-column>
        <el-table-column prop="path" label="存储路径" min-width="200" show-overflow-tooltip />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="handleDownload(row)">下载</el-button>
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

    <!-- 上传文件对话框 -->
    <el-dialog v-model="uploadDialogVisible" title="上传文件" width="550px" destroy-on-close>
      <el-form label-width="80px">
        <el-form-item label="选择文件">
          <el-upload
            ref="uploadRef"
            :auto-upload="false"
            :multiple="true"
            :limit="10"
            :on-change="handleFileChange"
            :on-remove="handleFileRemove"
            :file-list="fileList"
            drag
          >
            <el-icon class="el-icon--upload"><upload-filled /></el-icon>
            <div class="el-upload__text">拖拽文件至此处，或<em>点击选择</em></div>
            <template #tip>
              <div class="el-upload__tip">支持多文件，单文件最大 10MB（可配置）</div>
            </template>
          </el-upload>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="uploadDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" :disabled="fileList.length === 0" @click="handleUpload">
          上传 {{ fileList.length > 0 ? `(${fileList.length} 个文件)` : '' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { UploadFilled } from '@element-plus/icons-vue'
import type { UploadInstance, UploadFile } from 'element-plus'
import request from '@/api/request'
import { uploadFiles, listFiles, deleteFile } from '@/api/file'
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
const uploadRef = ref<UploadInstance>()
const fileList = ref<UploadFile[]>([])

function handleFileChange(_file: UploadFile, fileListNew: UploadFile[]) {
  fileList.value = fileListNew
}

function handleFileRemove(_file: UploadFile, fileListNew: UploadFile[]) {
  fileList.value = fileListNew
}

async function loadData() {
  loading.value = true
  try {
    const res = await listFiles({
      name: query.name || undefined,
      fileType: query.fileType || undefined,
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
  query.fileType = ''
  page.value = 1
  loadData()
}

async function handleUpload() {
  if (fileList.value.length === 0) {
    ElMessage.warning('请选择文件')
    return
  }
  submitLoading.value = true
  try {
    const formData = new FormData()
    fileList.value.forEach((f) => {
      formData.append('file', f.raw!)
    })
    const res = await uploadFiles(formData)
    ElMessage.success(`成功上传 ${res.data.length} 个文件`)
    uploadDialogVisible.value = false
    fileList.value = []
    loadData()
  } catch {
    /* 错误由拦截器统一处理 */
  } finally {
    submitLoading.value = false
  }
}

async function handleDownload(row: FileResponse) {
  try {
    const res = await request.get(`/api/file/${row.id}/download`, { responseType: 'blob' })
    const url = URL.createObjectURL(res.data)
    const a = document.createElement('a')
    a.href = url
    a.download = row.name
    a.click()
    URL.revokeObjectURL(url)
  } catch {
    /* 错误由拦截器统一处理 */
  }
}

async function handleDelete(row: FileResponse) {
  await ElMessageBox.confirm(`确认删除文件 "${row.name}" 吗？删除后不可恢复。`, '提示', {
    type: 'warning',
  })
  await deleteFile(row.id)
  ElMessage.success('删除成功')
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
</style>
