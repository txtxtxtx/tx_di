<template>
  <div class="page">
    <el-row :gutter="16">
      <el-col :span="12">
        <el-card header="CPU 信息">
          <el-descriptions :column="1" border v-if="info">
            <el-descriptions-item label="核心数">{{ info.cpuCores }}</el-descriptions-item>
            <el-descriptions-item label="使用率">
              <el-progress :percentage="info.cpuUsage" :color="getColor(info.cpuUsage)" />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card header="内存信息">
          <el-descriptions :column="1" border v-if="info">
            <el-descriptions-item label="总内存">{{ formatBytes(info.totalMemory) }}</el-descriptions-item>
            <el-descriptions-item label="已使用">{{ formatBytes(info.usedMemory) }}</el-descriptions-item>
            <el-descriptions-item label="使用率">
              <el-progress :percentage="info.memoryUsage" :color="getColor(info.memoryUsage)" />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" style="margin-top: 16px">
      <el-col :span="12">
        <el-card header="磁盘信息">
          <el-descriptions :column="1" border v-if="info">
            <el-descriptions-item label="总磁盘">{{ formatBytes(info.totalDisk) }}</el-descriptions-item>
            <el-descriptions-item label="已使用">{{ formatBytes(info.usedDisk) }}</el-descriptions-item>
            <el-descriptions-item label="使用率">
              <el-progress :percentage="info.diskUsage" :color="getColor(info.diskUsage)" />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card header="系统信息">
          <el-descriptions :column="1" border v-if="info">
            <el-descriptions-item label="操作系统">{{ info.osName }}</el-descriptions-item>
            <el-descriptions-item label="系统版本">{{ info.osVersion }}</el-descriptions-item>
            <el-descriptions-item label="主机名">{{ info.hostname }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getServerInfo } from '@/api/monitor'
import { formatBytes } from '@/utils'
import type { ServerInfo } from '@/types'

const info = ref<ServerInfo | null>(null)

function getColor(val: number): string {
  if (val < 50) return '#67c23a'
  if (val < 80) return '#e6a23c'
  return '#f56c6c'
}

onMounted(async () => {
  try {
    const res = await getServerInfo()
    info.value = res.data
  } catch {}
})
</script>
