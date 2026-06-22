<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>存储配置</span>
          <el-button type="primary" @click="openDialog()">新增配置</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" show-overflow-tooltip />
        <el-table-column prop="name" label="名称" min-width="150" show-overflow-tooltip />
        <el-table-column prop="storage" label="存储类型" width="140">
          <template #default="{ row }">
            <el-tag :type="dictColorType(dictStore.dictMap['sys_file_storage'] || [], row.storage) as any">
              {{ dictLabel(dictStore.dictMap['sys_file_storage'] || [], row.storage) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column prop="master" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="dictColorType(dictStore.dictMap['sys_file_master'] || [], row.master) as any">
              {{ dictLabel(dictStore.dictMap['sys_file_master'] || [], row.master) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button v-if="row.master !== 1" link type="success" size="small" @click="handleSetMaster(row)">设为主配置</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑配置' : '新增配置'" width="600px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" placeholder="请输入配置名称" />
        </el-form-item>
        <el-form-item label="存储类型" prop="storage">
          <el-select v-model="form.storage" placeholder="请选择存储类型" @change="onStorageChange">
            <el-option v-for="o in dictToOptions(dictStore.dictMap['sys_file_storage'] || [])" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remark" type="textarea" :rows="2" placeholder="备注信息（选填）" />
        </el-form-item>
        <el-form-item label="主配置">
          <el-switch v-model="form.master" active-text="是" inactive-text="否" />
        </el-form-item>
        <el-form-item label="配置" prop="config">
          <el-input v-model="form.config" type="textarea" :rows="10" :placeholder="configPlaceholder" />
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
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listFileConfigs, createFileConfig, updateFileConfig, deleteFileConfig, setMasterFileConfig } from '@/api/file-config'
import { dictToOptions, dictLabel, dictColorType } from '@/utils'
import { useDictStore } from '@/stores/dict'
import type { FileConfigResponse } from '@/types'

const dictStore = useDictStore()

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<FileConfigResponse[]>([])

const dialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref<number>(0)
const formRef = ref<FormInstance>()
const form = reactive({
  name: '',
  storage: 0,
  remark: '',
  config: '',
  master: false,
})

const configExamples: Record<number, string> = {
  0: '{"base_path": "./uploads", "base_url": "http://localhost:8080/files"}',
  1: '{"base_path": "/", "base_url": "", "s3": {"bucket": "my-bucket", "region": "ap-southeast-1", "endpoint": "http://localhost:9000", "access_key": "admin", "secret_key": "admin", "force_path_style": true}}',
  2: '{}',
}

const configPlaceholder = computed(() => `示例: ${configExamples[form.storage]}`)

const formRules: FormRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  storage: [{ required: true, message: '请选择存储类型', trigger: 'change' }],
  config: [{ required: true, message: '请输入配置JSON', trigger: 'blur' }],
}

function onStorageChange() {
  form.config = ''
}

async function loadData() {
  loading.value = true
  try {
    const res = await listFileConfigs()
    tableData.value = res.data
  } catch {
    /* 错误由拦截器统一处理 */
  } finally {
    loading.value = false
  }
}

function openDialog(row?: FileConfigResponse) {
  isEdit.value = !!row
  if (row) {
    editingId.value = row.id
    form.name = row.name
    form.storage = row.storage
    form.remark = row.remark || ''
    form.config = row.config
    form.master = row.master === 1
  } else {
    editingId.value = 0
    form.name = ''
    form.storage = 0
    form.remark = ''
    form.config = ''
    form.master = false
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    let configId: number
    if (isEdit.value) {
      const res = await updateFileConfig(editingId.value, {
        id: editingId.value,
        name: form.name,
        storage: form.storage,
        remark: form.remark || undefined,
        config: form.config,
      })
      configId = res.data.id
      ElMessage.success('更新成功')
    } else {
      const res = await createFileConfig({
        name: form.name,
        storage: form.storage,
        remark: form.remark || undefined,
        config: form.config,
      })
      configId = res.data.id
      ElMessage.success('创建成功')
    }
    if (form.master) {
      await setMasterFileConfig(configId)
      ElMessage.success('已设为主配置')
    }
    dialogVisible.value = false
    loadData()
  } catch {
    /* 错误由拦截器统一处理 */
  } finally {
    submitLoading.value = false
  }
}

async function handleDelete(row: FileConfigResponse) {
  await ElMessageBox.confirm(`确认删除配置 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteFileConfig(row.id)
  ElMessage.success('删除成功')
  loadData()
}

async function handleSetMaster(row: FileConfigResponse) {
  await setMasterFileConfig(row.id)
  ElMessage.success('已设为主配置')
  loadData()
}

onMounted(async () => {
  await Promise.all([
    dictStore.getDictData('sys_file_storage'),
    dictStore.getDictData('sys_file_master'),
  ])
  loadData()
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
