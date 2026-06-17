<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>权限列表</span>
          <el-button type="primary" @click="openDialog()">新增权限</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" row-key="id" border default-expand-all :tree-props="{ children: 'children' }">
        <el-table-column prop="name" label="权限名称" min-width="180" />
        <el-table-column prop="permissionCode" label="权限编码" width="200" show-overflow-tooltip />
        <el-table-column prop="type" label="类型" width="80">
          <template #default="{ row }">{{ permissionTypeLabel(row.type) }}</template>
        </el-table-column>
        <el-table-column prop="sort" label="排序" width="80" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑权限' : '新增权限'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="权限名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="权限编码" prop="permissionCode">
          <el-input v-model="form.permissionCode" placeholder="如: user:create" />
        </el-form-item>
        <el-form-item label="类型" prop="type">
          <el-select v-model="form.type">
            <el-option v-for="o in permissionTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="上级权限">
          <el-select v-model="form.parentId" clearable placeholder="顶级">
            <el-option :key="0" label="顶级" :value="0" />
            <el-option v-for="p in allPermissions" :key="p.id" :label="p.name" :value="p.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="form.sort" :min="0" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" />
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
import { listPermissions, createPermission, updatePermission, deletePermission } from '@/api/permission'
import { statusLabel, permissionTypeOptions, permissionTypeLabel } from '@/utils'
import type { PermissionDetail } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<PermissionDetail[]>([])
const allPermissions = ref<PermissionDetail[]>([])

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({ id: 0, name: '', permissionCode: '', type: 0, parentId: 0, sort: 0, description: '' })
const formRules: FormRules = {
  name: [{ required: true, message: '请输入权限名称', trigger: 'blur' }],
  permissionCode: [{ required: true, message: '请输入权限编码', trigger: 'blur' }],
}

async function loadData() {
  loading.value = true
  try {
    const res = await listPermissions()
    tableData.value = res.data.permissions
    allPermissions.value = res.data.permissions
  } catch {} finally { loading.value = false }
}

function openDialog(row?: PermissionDetail) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, { id: row.id, name: row.name, permissionCode: row.permissionCode, type: row.type, parentId: row.parentId, sort: row.sort, description: row.description })
  } else {
    Object.assign(form, { id: 0, name: '', permissionCode: '', type: 0, parentId: 0, sort: 0, description: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    const data = { name: form.name, permissionCode: form.permissionCode, type: form.type, parentId: form.parentId, sort: form.sort, description: form.description }
    if (isEdit.value) {
      await updatePermission(form.id, data)
      ElMessage.success('更新成功')
    } else {
      await createPermission(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: PermissionDetail) {
  await ElMessageBox.confirm(`确认删除权限 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deletePermission(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.card-header { display: flex; justify-content: space-between; align-items: center; }
</style>
