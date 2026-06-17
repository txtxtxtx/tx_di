<template>
  <div class="dashboard">
    <el-row :gutter="16">
      <el-col :span="12">
        <el-card header="服务器信息">
          <el-descriptions :column="1" border v-if="serverInfo">
            <el-descriptions-item label="操作系统">{{ serverInfo.osName }} {{ serverInfo.osVersion }}</el-descriptions-item>
            <el-descriptions-item label="主机名">{{ serverInfo.hostname }}</el-descriptions-item>
            <el-descriptions-item label="CPU 核心">{{ serverInfo.cpuCores }}</el-descriptions-item>
            <el-descriptions-item label="CPU 使用率">
              <el-progress :percentage="serverInfo.cpuUsage" :color="getColor(serverInfo.cpuUsage)" />
            </el-descriptions-item>
            <el-descriptions-item label="内存">
              {{ formatBytes(serverInfo.usedMemory) }} / {{ formatBytes(serverInfo.totalMemory) }}
              <el-progress :percentage="serverInfo.memoryUsage" :color="getColor(serverInfo.memoryUsage)" />
            </el-descriptions-item>
            <el-descriptions-item label="磁盘">
              {{ formatBytes(serverInfo.usedDisk) }} / {{ formatBytes(serverInfo.totalDisk) }}
              <el-progress :percentage="serverInfo.diskUsage" :color="getColor(serverInfo.diskUsage)" />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card header="缓存统计">
          <el-descriptions :column="1" border v-if="cacheStats">
            <el-descriptions-item label="总键数">{{ cacheStats.totalKeys }}</el-descriptions-item>
            <el-descriptions-item label="内存使用">{{ formatBytes(cacheStats.usedMemory) }}</el-descriptions-item>
            <el-descriptions-item label="命中次数">{{ cacheStats.hitCount }}</el-descriptions-item>
            <el-descriptions-item label="未命中次数">{{ cacheStats.missCount }}</el-descriptions-item>
            <el-descriptions-item label="命中率">
              <el-progress :percentage="cacheStats.hitRate" :color="'#67c23a'" />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getServerInfo } from '@/api/monitor'
import { getCacheStats } from '@/api/tool'
import { formatBytes } from '@/utils'
import type { ServerInfo, CacheStatsResponse } from '@/types'

const serverInfo = ref<ServerInfo | null>(null)
const cacheStats = ref<CacheStatsResponse | null>(null)

function getColor(val: number): string {
  if (val < 50) return '#67c23a'
  if (val < 80) return '#e6a23c'
  return '#f56c6c'
}

onMounted(async () => {
  try {
    const res = await getServerInfo()
    serverInfo.value = res.data
  } catch {}
  try {
    const res = await getCacheStats()
    cacheStats.value = res.data
  } catch {}
})
</script>

<style scoped>
.dashboard {
  min-height: calc(100vh - 130px);
}
</style>
