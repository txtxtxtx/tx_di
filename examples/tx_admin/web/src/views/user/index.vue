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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useUserStore } from '@/stores/user'

const userStore = useUserStore()
const userInfo = computed(() => userStore.userInfo)

onMounted(async () => {
  if (!userInfo.value) {
    await userStore.fetchUserInfo()
  }
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
</style>
