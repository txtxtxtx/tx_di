<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="部门名称">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in statusOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="loadData">搜索</el-button>
          <el-button @click="resetQuery">重置</el-button>
          <el-button type="success" @click="openDialog()">新增部门</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="never">
      <el-table :data="treeData" v-loading="loading" row-key="id" border default-expand-all :tree-props="{ children: 'children' }">
        <el-table-column prop="name" label="部门名称" min-width="200" />
        <el-table-column prop="sort" label="排序" width="80" />
        <el-table-column prop="leader_user_id" label="负责人ID" width="100" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="primary" size="small" @click="openDialog(null, row.id)">新增子部门</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑部门' : '新增部门'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="上级部门">
          <el-tree-select v-model="form.parent_id" :data="deptTreeForSelect" :props="{ label: 'name', value: 'id', children: 'children' }" check-strictly clearable placeholder="顶级部门" />
        </el-form-item>
        <el-form-item label="部门名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="排序" prop="sort">
          <el-input-number v-model="form.sort" :min="0" />
        </el-form-item>
        <el-form-item label="负责人">
          <el-input v-model="form.leader_user_id" placeholder="负责人用户ID" />
        </el-form-item>
        <el-form-item label="电话">
          <el-input v-model="form.phone" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="form.email" />
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
import { listDepts, createDept, updateDept, deleteDept } from '@/api/dept'
import { statusOptions, statusLabel } from '@/utils'
import type { DeptTreeNode } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const treeData = ref<DeptTreeNode[]>([])
const query = reactive({ name: '', status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  id: '',
  name: '',
  parent_id: '',
  sort: 0,
  leader_user_id: undefined as string | undefined,
  phone: '',
  email: '',
})
const formRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
}

const deptTreeForSelect = computed(() => {
  const root: DeptTreeNode = { id: '0', name: '顶级部门', parent_id: '0', sort: 0, leader_user_id: null, status: 0, children: treeData.value }
  return [root]
})

async function loadData() {
  loading.value = true
  try {
    treeData.value = (await listDepts({ name: query.name || undefined, status: query.status })).data
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.name = ''; query.status = undefined; loadData() }

function openDialog(row?: DeptTreeNode | null, parentId?: string) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, {
      id: row.id, name: row.name, parent_id: row.parent_id, sort: row.sort,
      leader_user_id: row.leader_user_id || undefined, phone: '', email: '',
    })
  } else {
    Object.assign(form, { id: '', name: '', parent_id: parentId || '', sort: 0, leader_user_id: undefined, phone: '', email: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    const data = {
      name: form.name, parent_id: form.parent_id, sort: form.sort,
      leader_user_id: form.leader_user_id, phone: form.phone || undefined, email: form.email || undefined,
    }
    if (isEdit.value) {
      await updateDept(form.id, data)
      ElMessage.success('更新成功')
    } else {
      await createDept(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: DeptTreeNode) {
  await ElMessageBox.confirm(`确认删除部门 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteDept(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
</style>
