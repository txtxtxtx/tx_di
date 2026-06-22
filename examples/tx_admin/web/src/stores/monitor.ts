import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getServerInfo } from '@/api/monitor'
import type { ServerInfo } from '@/types'

export const useMonitorStore = defineStore('monitor', () => {
  const server = ref<ServerInfo>({
    osName: '',
    osVersion: '',
    hostname: '',
    cpuCores: 0,
    cpuUsage: 0,
    totalMemory: 0,
    usedMemory: 0,
    memoryUsage: 0,
    totalDisk: 0,
    usedDisk: 0,
    diskUsage: 0,
    disks: [],
    networks: [],
  })

  async function fetchServer(all?: boolean): Promise<ServerInfo[]> {
    const res = await getServerInfo(all)
    if (all) {
      // 返回列表，同时更新 server 为最新一条
      const list: ServerInfo[] = (res.data as any).list ?? []
      if (list.length > 0) server.value = list[list.length - 1]
      return list
    } else {
      server.value = res.data as unknown as ServerInfo
      return [server.value]
    }
  }

  return { server, fetchServer }
})
