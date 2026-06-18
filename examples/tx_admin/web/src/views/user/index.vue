<template>
  <div class="user-center">
    <el-row :gutter="16">
      <el-col :span="8">
        <el-card>
          <div class="user-avatar">
            <el-avatar :size="100" :src="userInfo?.avatar || undefined">
              {{ userInfo?.nickname?.charAt(0) || 'U' }}
            </el-avatar>
            <h2>{{ userInfo?.nickname || userInfo?.username }}</h2>
            <el-tag v-if="userInfo?.roles?.length" v-for="role in userInfo.roles" :key="role" class="role-tag">
              {{ role }}
            </el-tag>
          </div>
        </el-card>
      </el-col>
      <el-col :span="16">
        <el-card header="基本信息">
          <template #header>
            <div class="card-header">
              <span>基本信息</span>
              <div>
                <el-button type="primary" size="small" @click="openEditDialog">编辑信息</el-button>
                <el-button type="warning" size="small" @click="openPwdDialog">修改密码</el-button>
              </div>
            </div>
          </template>
          <el-descriptions :column="2" border>
            <el-descriptions-item label="用户名">{{ userInfo?.username }}</el-descriptions-item>
            <el-descriptions-item label="昵称">{{ userInfo?.nickname }}</el-descriptions-item>
            <el-descriptions-item label="邮箱">{{ userInfo?.email || '-' }}</el-descriptions-item>
            <el-descriptions-item label="手机号">{{ userInfo?.mobile || '-' }}</el-descriptions-item>
            <el-descriptions-item label="租户ID">{{ userInfo?.tenantId }}</el-descriptions-item>
            <el-descriptions-item label="权限数">{{ userInfo?.permissions?.length || 0 }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>

    <!-- 编辑信息弹窗 -->
    <el-dialog v-model="editDialogVisible" title="编辑信息" width="500px" destroy-on-close>
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-width="80px">
        <el-form-item label="昵称" prop="nickname">
          <el-input v-model="editForm.nickname" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="editForm.email" />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input v-model="editForm.mobile" />
        </el-form-item>
        <el-form-item label="性别">
          <el-select v-model="editForm.sex" placeholder="请选择">
            <el-option v-for="o in sexOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleEditSubmit">确定</el-button>
      </template>
    </el-dialog>

    <!-- 修改密码弹窗 -->
    <el-dialog v-model="pwdDialogVisible" title="修改密码" width="400px" destroy-on-close>
      <el-form ref="pwdFormRef" :model="pwdForm" :rules="pwdRules" label-width="80px">
        <el-form-item label="新密码" prop="newPassword">
          <el-input v-model="pwdForm.newPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="确认密码" prop="confirmPassword">
          <el-input v-model="pwdForm.confirmPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="pwdDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handlePwdSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { useUserStore } from '@/stores/user'
import { updateUser, changePassword } from '@/api/user'
import { dictToOptions } from '@/utils'
import { useDictStore } from '@/stores/dict'

const userStore = useUserStore()
const dictStore = useDictStore()
const userInfo = computed(() => userStore.userInfo)
const submitLoading = ref(false)

const sexOptions = computed(() => dictToOptions(dictStore.dictMap['sys_sex'] || []))

// ── 编辑信息 ──
const editDialogVisible = ref(false)
const editFormRef = ref<FormInstance>()
const editForm = reactive({ nickname: '', email: '', mobile: '', sex: undefined as number | undefined })
const editRules: FormRules = {
  nickname: [{ required: true, message: '请输入昵称', trigger: 'blur' }],
}

function openEditDialog() {
  editForm.nickname = userInfo.value?.nickname || ''
  editForm.email = userInfo.value?.email || ''
  editForm.mobile = userInfo.value?.mobile || ''
  editForm.sex = undefined // UserInfoResponse 没有 sex 字段，需 getUser 获取
  editDialogVisible.value = true
}

async function handleEditSubmit() {
  const valid = await editFormRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    await updateUser(userInfo.value!.userId, {
      nickname: editForm.nickname,
      email: editForm.email || undefined,
      mobile: editForm.mobile || undefined,
      sex: editForm.sex,
    })
    await userStore.fetchUserInfo()
    editDialogVisible.value = false
    ElMessage.success('更新成功')
  } catch {} finally {
    submitLoading.value = false
  }
}

// ── 修改密码 ──
const pwdDialogVisible = ref(false)
const pwdFormRef = ref<FormInstance>()
const pwdForm = reactive({ newPassword: '', confirmPassword: '' })
const pwdRules: FormRules = {
  newPassword: [{ required: true, message: '请输入新密码', trigger: 'blur' }],
  confirmPassword: [
    { required: true, message: '请确认密码', trigger: 'blur' },
    {
      validator: (_r, v, cb) => v === pwdForm.newPassword ? cb() : cb(new Error('两次密码不一致')),
      trigger: 'blur',
    },
  ],
}

function openPwdDialog() {
  pwdForm.newPassword = ''
  pwdForm.confirmPassword = ''
  pwdDialogVisible.value = true
}

async function handlePwdSubmit() {
  const valid = await pwdFormRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    await changePassword({ userId: userInfo.value!.userId, newPassword: pwdForm.newPassword })
    pwdDialogVisible.value = false
    ElMessage.success('密码修改成功')
  } catch {} finally {
    submitLoading.value = false
  }
}

onMounted(async () => {
  if (!userInfo.value) {
    await userStore.fetchUserInfo()
  }
  await dictStore.getDictData('sys_sex')
})
</script>

<style scoped>
.user-center {
  padding: 16px;
}

.user-avatar {
  text-align: center;
  padding: 20px 0;
}

.user-avatar h2 {
  margin-top: 16px;
  margin-bottom: 8px;
}

.role-tag {
  margin: 0 4px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
