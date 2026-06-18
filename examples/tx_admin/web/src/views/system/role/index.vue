<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="角色名">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="角色编码">
          <el-input v-model="query.code" placeholder="请输入" clearable />
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

    <el-card shadow="never" class="table-card">
      <template #header>
        <div class="card-header">
          <span>角色列表</span>
          <el-button type="primary" @click="openDialog()">新增角色</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="name" label="角色名" width="150" />
        <el-table-column prop="code" label="角色编码" width="150" />
        <el-table-column prop="sort" label="排序" width="80" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="250" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="primary" size="small" @click="openAssignMenus(row)">分配菜单</el-button>
            <el-button link type="primary" size="small" @click="openRoleUsers(row)">用户列表</el-button>
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

    <!-- 新增/编辑 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑角色' : '新增角色'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="角色名" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="角色编码" prop="code">
          <el-input v-model="form.code" />
        </el-form-item>
        <el-form-item label="排序" prop="sort">
          <el-input-number v-model="form.sort" :min="0" />
        </el-form-item>
        <el-form-item v-if="isEdit" label="数据范围">
          <el-select v-model="form.data_scope">
            <el-option v-for="o in dataScopeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
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

    <!-- 分配菜单 -->
    <el-dialog v-model="menuDialogVisible" title="分配菜单权限" width="500px" destroy-on-close>
      <el-tree
        ref="menuTreeRef"
        :data="menuTreeData"
        :props="{ label: 'name', children: 'children' }"
        node-key="id"
        show-checkbox
        :default-checked-keys="selectedMenuIds"
      />
      <template #footer>
        <el-button @click="menuDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleAssignMenus">确定</el-button>
      </template>
    </el-dialog>

    <!-- 角色用户列表 -->
    <el-dialog v-model="roleUsersVisible" title="角色用户" width="600px" destroy-on-close>
      <el-table :data="roleUsers" border>
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="username" label="用户名" />
        <el-table-column prop="nickname" label="昵称" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listRoles, createRole, updateRole, deleteRole, assignMenus, getRoleUsers } from '@/api/role'
import { listMenus } from '@/api/menu'
import { statusOptions, statusLabel, dataScopeOptions } from '@/utils'
import type { RoleResponse, MenuTreeNode, UserResponse } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<RoleResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ name: '', code: '', status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({ id: '', name: '', code: '', sort: 0, data_scope: 1, remark: '' })
const formRules: FormRules = {
  name: [{ required: true, message: '请输入角色名', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色编码', trigger: 'blur' }],
}

const menuDialogVisible = ref(false)
const menuTreeData = ref<MenuTreeNode[]>([])
const selectedMenuIds = ref<string[]>([])
const menuTreeRef = ref()
const currentRoleId = ref('')

const roleUsersVisible = ref(false)
const roleUsers = ref<UserResponse[]>([])

async function loadData() {
  loading.value = true
  try {
    const res = await listRoles({
      name: query.name || undefined,
      code: query.code || undefined,
      status: query.status,
      page: page.value,
      pageSize: size.value,
    })
    tableData.value = res.data.list
    total.value = res.data.total
  } catch {} finally {
    loading.value = false
  }
}

function resetQuery() {
  query.name = ''; query.code = ''; query.status = undefined
  page.value = 1; loadData()
}

function openDialog(row?: RoleResponse) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, { id: row.id, name: row.name, code: row.code, sort: row.sort, data_scope: row.dataScope, remark: row.remark || '' })
  } else {
    Object.assign(form, { id: '', name: '', code: '', sort: 0, data_scope: 1, remark: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    if (isEdit.value) {
      await updateRole(form.id, { name: form.name, code: form.code, sort: form.sort, dataScope: form.data_scope, remark: form.remark || undefined })
      ElMessage.success('更新成功')
    } else {
      await createRole({ name: form.name, code: form.code, sort: form.sort, remark: form.remark || undefined, menuIds: [] })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: RoleResponse) {
  await ElMessageBox.confirm(`确认删除角色 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteRole(row.id)
  ElMessage.success('删除成功'); loadData()
}

async function openAssignMenus(row: RoleResponse) {
  currentRoleId.value = row.id
  selectedMenuIds.value = [...(row.menuIds || [])]
  try { menuTreeData.value = (await listMenus()).data } catch { menuTreeData.value = [] }
  menuDialogVisible.value = true
}

async function handleAssignMenus() {
  const checkedKeys = menuTreeRef.value?.getCheckedKeys(false) || []
  const halfCheckedKeys = menuTreeRef.value?.getHalfCheckedKeys() || []
  submitLoading.value = true
  try {
    await assignMenus({ roleId: currentRoleId.value, menuIds: [...checkedKeys, ...halfCheckedKeys] })
    ElMessage.success('分配菜单成功')
    menuDialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function openRoleUsers(row: RoleResponse) {
  try {
    const res = await getRoleUsers(row.id)
    roleUsers.value = res.data
  } catch { roleUsers.value = [] }
  roleUsersVisible.value = true
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
