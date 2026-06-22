<template>
  <div class="page">
    <!-- 搜索栏 -->
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="用户名">
          <el-input v-model="query.username" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="昵称">
          <el-input v-model="query.nickname" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input v-model="query.mobile" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in dictToOptions(dictStore.dictMap['sys_user_status'] || [])" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="loadData">搜索</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 工具栏 -->
    <el-card shadow="never" class="table-card">
      <template #header>
        <div class="card-header">
          <span>用户列表</span>
          <el-button type="primary" @click="openDialog()">新增用户</el-button>
        </div>
      </template>

      <el-table :data="tableData" v-loading="loading" border stripe>
        <el-table-column prop="id" label="ID" width="60" show-overflow-tooltip />
        <el-table-column prop="username" label="用户名" width="120" />
        <el-table-column prop="nickname" label="昵称" width="120" />
        <el-table-column prop="email" label="邮箱" min-width="160" show-overflow-tooltip />
        <el-table-column prop="mobile" label="手机号" width="130" />
        <el-table-column prop="sex" label="性别" width="70">
          <template #default="{ row }">{{ dictLabel(dictStore.dictMap['sys_sex'] || [], row.sex) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="dictColorType(dictStore.dictMap['sys_user_status'] || [], row.status) as any">{{ dictLabel(dictStore.dictMap['sys_user_status'] || [], row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createTime" label="创建时间" width="130">
          <template #default="{ row }">{{ formatTimestamp(row.createTime) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="290" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="warning" size="small" @click="openResetPwd(row)">重置密码</el-button>
            <el-button v-if="row.status === 0" link type="danger" size="small" @click="handleDisable(row)">禁用</el-button>
            <el-button v-else link type="success" size="small" @click="handleEnable(row)">启用</el-button>
            <el-button link type="primary" size="small" @click="openAssignRoles(row)">角色</el-button>
            <el-button link type="primary" size="small" @click="openAssignDepts(row)">部门</el-button>
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
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑用户' : '新增用户'" width="500px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="80px">
        <el-form-item label="用户名" prop="username">
          <el-input v-model="form.username" :disabled="isEdit" />
        </el-form-item>
        <el-form-item v-if="!isEdit" label="密码" prop="password">
          <el-input v-model="form.password" type="password" show-password />
        </el-form-item>
        <el-form-item label="昵称" prop="nickname">
          <el-input v-model="form.nickname" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="form.email" />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input v-model="form.mobile" />
        </el-form-item>
        <el-form-item label="性别">
          <el-select v-model="form.sex" placeholder="请选择">
            <el-option v-for="o in dictToOptions(dictStore.dictMap['sys_sex'] || [])" :key="o.value" :label="o.label" :value="o.value" />
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

    <!-- 分配角色对话框 -->
    <el-dialog v-model="rolesDialogVisible" title="分配角色" width="500px" destroy-on-close>
      <el-transfer v-model="selectedRoleIds" :data="allRoleOptions" :titles="['可选角色', '已选角色']" />
      <template #footer>
        <el-button @click="rolesDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleAssignRoles">确定</el-button>
      </template>
    </el-dialog>

    <!-- 重置密码对话框 -->
    <el-dialog v-model="resetPwdVisible" title="重置密码" width="400px" destroy-on-close>
      <el-form ref="resetPwdFormRef" :model="resetPwdForm" :rules="resetPwdRules" label-width="80px">
        <el-form-item label="用户">
          <el-input :model-value="resetPwdUsername" disabled />
        </el-form-item>
        <el-form-item label="新密码" prop="newPassword">
          <el-input v-model="resetPwdForm.newPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resetPwdVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleResetPwd">确定</el-button>
      </template>
    </el-dialog>

    <!-- 分配部门对话框 -->
    <el-dialog v-model="deptsDialogVisible" title="分配部门" width="500px" destroy-on-close>
      <el-tree
        ref="deptTreeRef"
        :data="deptTreeData"
        :props="{ label: 'name', children: 'children' }"
        node-key="id"
        show-checkbox
        check-strictly
        :default-checked-keys="selectedDeptIds"
      />
      <template #footer>
        <el-button @click="deptsDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleAssignDepts">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listUsers, createUser, updateUser, deleteUser, assignRoles, assignDepts, changePassword, enableUser, disableUser } from '@/api/user'
import { getAllRoles } from '@/api/role'
import { listDepts } from '@/api/dept'
import { formatTimestamp, toPageData, dictToOptions, dictLabel, dictColorType } from '@/utils'
import { useDictStore } from '@/stores/dict'
import type { UserResponse, RoleResponse, DeptTreeNode } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const tableData = ref<UserResponse[]>([])
const page = ref(1)
const size = ref(10)
const total = ref(0)
const query = reactive({ username: '', nickname: '', mobile: '', status: undefined as number | undefined })
const dictStore = useDictStore()

// 用户表单
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  id: '',
  username: '',
  password: '',
  nickname: '',
  email: '',
  mobile: '',
  sex: undefined as number | undefined,
  remark: '',
})
const formRules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
  nickname: [{ required: true, message: '请输入昵称', trigger: 'blur' }],
}

// 重置密码
const resetPwdVisible = ref(false)
const resetPwdUsername = ref('')
const resetPwdUserId = ref('')
const resetPwdFormRef = ref<FormInstance>()
const resetPwdForm = reactive({ newPassword: '' })
const resetPwdRules: FormRules = {
  newPassword: [{ required: true, message: '请输入新密码', trigger: 'blur' }],
}

// 角色分配
const rolesDialogVisible = ref(false)
const allRoleOptions = ref<{ key: string; label: string }[]>([])
const selectedRoleIds = ref<string[]>([])
const currentUserId = ref('')

// 部门分配
const deptsDialogVisible = ref(false)
const deptTreeData = ref<DeptTreeNode[]>([])
const selectedDeptIds = ref<string[]>([])
const deptTreeRef = ref()

async function loadData() {
  loading.value = true
  try {
    const res = await listUsers({
      username: query.username || undefined,
      nickname: query.nickname || undefined,
      mobile: query.mobile || undefined,
      status: query.status,
      pageInfo: { page: page.value, size: size.value },
    })
    const pageData = toPageData(res.data)
    tableData.value = pageData.list
    total.value = pageData.total
  } catch {} finally {
    loading.value = false
  }
}

function resetQuery() {
  query.username = ''
  query.nickname = ''
  query.mobile = ''
  query.status = undefined
  page.value = 1
  loadData()
}

function openDialog(row?: UserResponse) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, {
      id: row.id,
      username: row.username,
      password: '',
      nickname: row.nickname,
      email: row.email || '',
      mobile: row.mobile || '',
      sex: row.sex,
      remark: row.remark || '',
    })
  } else {
    Object.assign(form, { id: '', username: '', password: '', nickname: '', email: '', mobile: '', sex: undefined, remark: '' })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    if (isEdit.value) {
      await updateUser(form.id, {
        nickname: form.nickname,
        email: form.email || undefined,
        mobile: form.mobile || undefined,
        sex: form.sex,
        remark: form.remark || undefined,
      })
      ElMessage.success('更新成功')
    } else {
      await createUser({
        username: form.username,
        password: form.password,
        nickname: form.nickname,
        email: form.email || undefined,
        mobile: form.mobile || undefined,
        sex: form.sex,
        remark: form.remark || undefined,
        roleIds: [],
        deptIds: [],
      })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadData()
  } catch {} finally {
    submitLoading.value = false
  }
}

async function handleDelete(row: UserResponse) {
  await ElMessageBox.confirm(`确认删除用户 "${row.username}" 吗？`, '提示', { type: 'warning' })
  await deleteUser(row.id)
  ElMessage.success('删除成功')
  loadData()
}

function openResetPwd(row: UserResponse) {
  resetPwdUserId.value = row.id
  resetPwdUsername.value = row.username
  resetPwdForm.newPassword = ''
  resetPwdVisible.value = true
}

async function handleResetPwd() {
  const valid = await resetPwdFormRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    await changePassword({ userId: resetPwdUserId.value, newPassword: resetPwdForm.newPassword })
    resetPwdVisible.value = false
    ElMessage.success('密码重置成功')
  } catch {} finally {
    submitLoading.value = false
  }
}

async function handleEnable(row: UserResponse) {
  await ElMessageBox.confirm(`确认启用用户 "${row.username}" 吗？`, '提示', { type: 'warning' })
  await enableUser({ userId: row.id })
  ElMessage.success('启用成功')
  loadData()
}

async function handleDisable(row: UserResponse) {
  await ElMessageBox.confirm(`确认禁用用户 "${row.username}" 吗？`, '提示', { type: 'warning' })
  await disableUser({ userId: row.id })
  ElMessage.success('禁用成功')
  loadData()
}

async function openAssignRoles(row: UserResponse) {
  currentUserId.value = row.id
  selectedRoleIds.value = [...row.roleIds]
  // 加载所有角色
  try {
    const res = await getAllRoles()
    allRoleOptions.value = res.data.map(r => ({ key: r.id, label: r.name }))
  } catch { allRoleOptions.value = [] }
  rolesDialogVisible.value = true
}

async function handleAssignRoles() {
  submitLoading.value = true
  try {
    await assignRoles({ userId: currentUserId.value, roleIds: selectedRoleIds.value })
    ElMessage.success('分配角色成功')
    rolesDialogVisible.value = false
    loadData()
  } catch {} finally {
    submitLoading.value = false
  }
}

async function openAssignDepts(row: UserResponse) {
  currentUserId.value = row.id
  selectedDeptIds.value = [...row.deptIds]
  try {
    deptTreeData.value = (await listDepts()).data
  } catch { deptTreeData.value = [] }
  deptsDialogVisible.value = true
}

async function handleAssignDepts() {
  const checkedKeys = deptTreeRef.value?.getCheckedKeys(false) || []
  submitLoading.value = true
  try {
    await assignDepts({ userId: currentUserId.value, deptIds: checkedKeys })
    ElMessage.success('分配部门成功')
    deptsDialogVisible.value = false
    loadData()
  } catch {} finally {
    submitLoading.value = false
  }
}

onMounted(async () => {
  await Promise.all([
    dictStore.getDictData('sys_user_status'),
    dictStore.getDictData('sys_sex'),
  ])
  loadData()
})
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.pagination-wrap { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
