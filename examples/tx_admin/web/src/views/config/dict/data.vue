<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="字典类型">
          <el-input v-model="query.dictType" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="标签">
          <el-input v-model="query.label" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in dictToOptions(dictStore.dictMap['sys_status'] || [])" :key="o.value" :label="o.label" :value="o.value" />
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
          <span>字典数据</span>
          <el-button type="primary" @click="openDialog()">新增字典数据</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="label" label="标签" width="150" />
        <el-table-column prop="value" label="值" width="120" />
        <el-table-column prop="dictType" label="字典类型" width="180" />
        <el-table-column prop="sort" label="排序" width="80" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ dictLabel(dictStore.dictMap['sys_status'] || [], row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrap">
        <el-pagination v-model:current-page="page" v-model:page-size="size" :total="total" :page-sizes="[10, 20, 50]" layout="total, sizes, prev, pager, next" @current-change="loadData" @size-change="loadData" />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑字典数据' : '新增字典数据'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="字典类型" prop="dictType">
          <el-input v-model="form.dictType" />
        </el-form-item>
        <el-form-item label="标签" prop="label">
          <el-input v-model="form.label" />
        </el-form-item>
        <el-form-item label="值" prop="value">
          <el-input v-model="form.value" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="form.sort" :min="0" />
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
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listDictData, createDictData, updateDictData, deleteDictData } from '@/api/dict'
import { toPageData, dictToOptions, dictLabel } from '@/utils'
import { useDictStore } from '@/stores/dict'
import type { DictDataResponse } from '@/types'

const route = useRoute()
const dictStore = useDictStore()
const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<DictDataResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ dictType: (route.query.dictType as string) || '', label: '', status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({ id: '', dictType: '', label: '', value: '', sort: 0, remark: '' })
const formRules: FormRules = {
  dictType: [{ required: true, message: '请输入字典类型', trigger: 'blur' }],
  label: [{ required: true, message: '请输入标签', trigger: 'blur' }],
  value: [{ required: true, message: '请输入值', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listDictData({ dictType: query.dictType || undefined, label: query.label || undefined, status: query.status, page: page.value, pageSize: size.value })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.dictType = ''; query.label = ''; query.status = undefined; page.value = 1; loadData() }

function openDialog(row?: DictDataResponse) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, { id: row.id, dictType: row.dictType, label: row.label, value: row.value, sort: row.sort, remark: row.remark || '' })
  } else {
    Object.assign(form, { id: '', dictType: query.dictType || '', label: '', value: '', sort: 0, remark: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    const data = { dictType: form.dictType, label: form.label, value: form.value, sort: form.sort, remark: form.remark || undefined }
    if (isEdit.value) {
      await updateDictData(form.id, data)
      ElMessage.success('更新成功')
    } else {
      await createDictData(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: DictDataResponse) {
  await ElMessageBox.confirm(`确认删除字典数据 "${row.label}" 吗？`, '提示', { type: 'warning' })
  await deleteDictData(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(async () => {
  await dictStore.getDictData('sys_status')
  loadData()
})
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
