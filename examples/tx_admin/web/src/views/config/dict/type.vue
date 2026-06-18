<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="字典名称">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="字典类型">
          <el-input v-model="query.dictType" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in statusOptions" :key="o.value" :label="o.label" :value="o.value" />
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
          <span>字典类型</span>
          <el-button type="primary" @click="openDialog()">新增字典类型</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="name" label="字典名称" width="200" />
        <el-table-column prop="dictType" label="字典类型" width="200" show-overflow-tooltip />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDictData(row)">字典数据</el-button>
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrap">
        <el-pagination v-model:current-page="page" v-model:page-size="size" :total="total" :page-sizes="[10, 20, 50]" layout="total, sizes, prev, pager, next" @current-change="loadData" @size-change="loadData" />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑字典类型' : '新增字典类型'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="字典名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="字典类型" prop="dictType">
          <el-input v-model="form.dictType" :disabled="isEdit" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remark" type="textarea" />
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
import { useRouter } from 'vue-router'
import { listDictTypes, createDictType, updateDictType, deleteDictType } from '@/api/dict'
import { statusLabel, statusOptions, toPageData } from '@/utils'
import type { DictTypeResponse } from '@/types'

const router = useRouter()
const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<DictTypeResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ name: '', dictType: '' as string, status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({ id: '', name: '', dictType: '', remark: '' })
const formRules: FormRules = {
  name: [{ required: true, message: '请输入字典名称', trigger: 'blur' }],
  dictType: [{ required: true, message: '请输入字典类型', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listDictTypes({ name: query.name || undefined, dictType: query.dictType || undefined, status: query.status, page: page.value, pageSize: size.value })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.name = ''; query.dictType = ''; query.status = undefined; page.value = 1; loadData() }

function openDialog(row?: DictTypeResponse) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, { id: row.id, name: row.name, dictType: row.dictType, remark: row.remark || '' })
  } else {
    Object.assign(form, { id: '', name: '', dictType: '', remark: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    const data = { name: form.name, dictType: form.dictType, remark: form.remark || undefined }
    if (isEdit.value) {
      await updateDictType(form.id, data)
      ElMessage.success('更新成功')
    } else {
      await createDictType(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: DictTypeResponse) {
  await ElMessageBox.confirm(`确认删除字典类型 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteDictType(row.id)
  ElMessage.success('删除成功'); loadData()
}

function openDictData(row: DictTypeResponse) {
  router.push({ path: '/config/dict-data', query: { dictType: row.dictType } })
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
