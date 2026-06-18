<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="名称">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="query.category" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="键名">
          <el-input v-model="query.configKey" placeholder="请输入" clearable />
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
          <span>配置列表</span>
          <el-button type="primary" @click="openDialog()">新增配置</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="name" label="名称" width="150" />
        <el-table-column prop="configKey" label="键名" width="200" show-overflow-tooltip />
        <el-table-column prop="value" label="值" min-width="200" show-overflow-tooltip />
        <el-table-column prop="category" label="分类" width="100" />
        <el-table-column prop="configType" label="类型" width="80">
          <template #default="{ row }">{{ configTypeLabel(row.configType) }}</template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" width="150" show-overflow-tooltip />
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

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑配置' : '新增配置'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="键名" prop="configKey">
          <el-input v-model="form.configKey" />
        </el-form-item>
        <el-form-item label="值" prop="value">
          <el-input v-model="form.value" type="textarea" />
        </el-form-item>
        <el-form-item label="分类" prop="category">
          <el-input v-model="form.category" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="form.configType">
            <el-option v-for="o in configTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="isEdit" label="是否可见">
          <el-radio-group v-model="form.visible">
            <el-radio :value="0">显示</el-radio>
            <el-radio :value="1">隐藏</el-radio>
          </el-radio-group>
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
import { listConfigs, createConfig, updateConfig, deleteConfig } from '@/api/config'
import { configTypeLabel, configTypeOptions } from '@/utils'
import type { ConfigResponse } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<ConfigResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ name: '', category: '', configKey: '' })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({ id: '', name: '', configKey: '', value: '', category: '', configType: 1, visible: 0, remark: '' })
const formRules: FormRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  configKey: [{ required: true, message: '请输入键名', trigger: 'blur' }],
  value: [{ required: true, message: '请输入值', trigger: 'blur' }],
  category: [{ required: true, message: '请输入分类', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listConfigs({ name: query.name || undefined, category: query.category || undefined, configKey: query.configKey || undefined, page: page.value, pageSize: size.value })
    tableData.value = res.data.list
    total.value = res.data.total
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.name = ''; query.category = ''; query.configKey = ''; page.value = 1; loadData() }

function openDialog(row?: ConfigResponse) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, { id: row.id, name: row.name, configKey: row.configKey, value: row.value, category: row.category, configType: row.configType, visible: row.visible, remark: row.remark || '' })
  } else {
    Object.assign(form, { id: '', name: '', configKey: '', value: '', category: '', configType: 1, visible: 0, remark: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    if (isEdit.value) {
      await updateConfig(form.id, { name: form.name, configKey: form.configKey, value: form.value, category: form.category, configType: form.configType, visible: form.visible, remark: form.remark || undefined })
      ElMessage.success('更新成功')
    } else {
      await createConfig({ name: form.name, configKey: form.configKey, value: form.value, category: form.category, configType: form.configType, remark: form.remark || undefined })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: ConfigResponse) {
  await ElMessageBox.confirm(`确认删除配置 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteConfig(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
