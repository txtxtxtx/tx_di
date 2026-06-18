<template>
  <div class="p-4">
    <el-card>
      <template #header>
        <div class="flex items-center justify-between">
          <span class="text-lg font-bold">服务器监控</span>
          <div class="flex items-center gap-2">
            <el-tag :type="isPolling ? 'success' : 'danger'" size="small">
              {{ isPolling ? '实时刷新中' : '已暂停' }}
            </el-tag>
            <el-button size="small" @click="togglePolling">
              {{ isPolling ? '暂停' : '恢复' }}
            </el-button>
          </div>
        </div>
      </template>

      <!-- CPU & 内存 折线图 -->
      <el-row :gutter="20" class="mb-4">
        <el-col :span="12">
          <div class="chart-title">CPU 使用率 (%)</div>
          <v-chart :option="cpuChartOption" autoresize style="height: 260px" />
        </el-col>
        <el-col :span="12">
          <div class="chart-title">内存使用率 (%)</div>
          <v-chart :option="memChartOption" autoresize style="height: 260px" />
        </el-col>
      </el-row>

      <!-- 当前数值概览 -->
      <el-row :gutter="20" class="mb-4">
        <el-col :span="8">
          <el-descriptions title="CPU 信息" :column="1" border size="small">
            <el-descriptions-item label="核心数">{{ server.cpuCores }}</el-descriptions-item>
            <el-descriptions-item label="使用率">{{ fmtPct(server.cpuUsage) }}</el-descriptions-item>
          </el-descriptions>
        </el-col>
        <el-col :span="8">
          <el-descriptions title="内存信息" :column="1" border size="small">
            <el-descriptions-item label="总内存">{{ fmtBytes(server.totalMemory) }}</el-descriptions-item>
            <el-descriptions-item label="已使用">{{ fmtBytes(server.usedMemory) }}</el-descriptions-item>
            <el-descriptions-item label="使用率">{{ fmtPct(server.memoryUsage) }}</el-descriptions-item>
          </el-descriptions>
        </el-col>
        <el-col :span="8">
          <el-descriptions title="系统信息" :column="1" border size="small">
            <el-descriptions-item label="主机名">{{ server.hostname || '-' }}</el-descriptions-item>
            <el-descriptions-item label="操作系统">{{ server.osName || '-' }}</el-descriptions-item>
            <el-descriptions-item label="版本">{{ server.osVersion || '-' }}</el-descriptions-item>
          </el-descriptions>
        </el-col>
      </el-row>

      <!-- 磁盘使用 -->
      <el-row :gutter="20">
        <el-col :span="24">
          <el-descriptions title="磁盘信息" :column="3" border size="small">
            <el-descriptions-item label="总容量">{{ fmtBytes(server.totalDisk) }}</el-descriptions-item>
            <el-descriptions-item label="已使用">{{ fmtBytes(server.usedDisk) }}</el-descriptions-item>
            <el-descriptions-item label="使用率">
              <el-progress
                :percentage="round2(server.diskUsage)"
                :color="getColor(server.diskUsage)"
                :stroke-width="15"
                striped
                striped-flow
              />
            </el-descriptions-item>
          </el-descriptions>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useMonitorStore } from '@/stores/monitor'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'

use([LineChart, GridComponent, TooltipComponent, CanvasRenderer])

const store = useMonitorStore()
const { server } = storeToRefs(store)

// ── 折线图历史数据 ────────────────────────────────────────────────
const MAX_POINTS = 60
const timeLabels = ref<string[]>([])
const cpuHistory = ref<number[]>([])
const memHistory = ref<number[]>([])

// ── 定时轮询 ──────────────────────────────────────────────────────
const POLL_INTERVAL = 1000
let timer: ReturnType<typeof setInterval> | null = null
const isPolling = ref(true)

function togglePolling() {
  isPolling.value ? stopPolling() : startPolling()
}

function startPolling() {
  if (timer) return
  isPolling.value = true
  fetchInitial()
  timer = setInterval(fetchData, POLL_INTERVAL)
}

/** 首次加载：获取全部缓存记录填充图表 */
async function fetchInitial() {
  const list = await store.fetchServer(true)
  for (const item of list) {
    appendHistory(item)
  }
}

function stopPolling() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
  isPolling.value = false
}

async function fetchData() {
  await store.fetchServer()
  appendHistory(server.value)
}

function appendHistory(item: { cpuUsage: number; memoryUsage: number }) {
  const now = new Date()
  const label = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`

  timeLabels.value.push(label)
  cpuHistory.value.push(round2(item.cpuUsage))
  memHistory.value.push(round2(item.memoryUsage))

  if (timeLabels.value.length > MAX_POINTS) {
    timeLabels.value.shift()
    cpuHistory.value.shift()
    memHistory.value.shift()
  }
}

// ── 格式化工具 ─────────────────────────────────────────────────────
function pad(n: number): string {
  return n.toString().padStart(2, '0')
}

function round2(v: number | undefined): number {
  return Math.round((v ?? 0) * 100) / 100
}

function fmtPct(v: number | undefined): string {
  return `${round2(v)}%`
}

function fmtBytes(bytes: number | undefined): string {
  if (bytes == null || bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${round2(bytes / Math.pow(1024, i))} ${units[i]}`
}

function getColor(usage: number | undefined): string {
  if (usage == null) return '#409EFF'
  if (usage >= 90) return '#F56C6C'
  if (usage >= 70) return '#E6A23C'
  return '#67C23A'
}

// ── 折线图配置 ─────────────────────────────────────────────────────
function buildLineOption(yData: number[], color: string) {
  return {
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) => {
        const p = params[0]
        return `${p.axisValue}<br/>${p.seriesName}: ${round2(p.value)}%`
      },
    },
    grid: { top: 10, right: 20, bottom: 24, left: 46 },
    xAxis: {
      type: 'category',
      data: timeLabels.value,
      axisLabel: { fontSize: 10, interval: 'auto' },
    },
    yAxis: {
      type: 'value',
      min: 0,
      max: 100,
      axisLabel: { formatter: '{value}%' },
    },
    series: [
      {
        name: '使用率',
        type: 'line',
        data: yData,
        smooth: true,
        symbol: 'none',
        lineStyle: { width: 2, color },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: `${color}66` },
              { offset: 1, color: `${color}0A` },
            ],
          },
        },
      },
    ],
  }
}

const cpuChartOption = computed(() => buildLineOption(cpuHistory.value, '#409EFF'))
const memChartOption = computed(() => buildLineOption(memHistory.value, '#67C23A'))

// ── 生命周期 ──────────────────────────────────────────────────────
onMounted(() => startPolling())
onUnmounted(() => stopPolling())
</script>

<style scoped>
.chart-title {
  font-size: 14px;
  font-weight: 600;
  text-align: center;
  margin-bottom: 8px;
}
</style>
