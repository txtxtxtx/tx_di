<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>在线用户</span>
          <el-tag>在线人数: {{ total }}</el-tag>
        </div>
      </template>

      <el-table :data="users" v-loading="loading" border stripe>
        <el-table-column prop="userId" label="用户ID" width="100" />
        <el-table-column prop="username" label="用户名" width="150" />
        <el-table-column prop="loginIp" label="登录IP" width="150" />
        <el-table-column prop="loginTime" label="登录时间" min-width="200" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getOnlineUsers } from '@/api/monitor'
import type { OnlineUser } from '@/types'

const loading = ref(false)
const users = ref<OnlineUser[]>([])
const total = ref(0)

onMounted(async () => {
  loading.value = true
  try {
    const res = await getOnlineUsers()
    users.value = res.data.users
    total.value = res.data.total
  } catch {} finally { loading.value = false }
})
</script>

<style scoped>
.card-header { display: flex; justify-content: space-between; align-items: center; }
</style>
