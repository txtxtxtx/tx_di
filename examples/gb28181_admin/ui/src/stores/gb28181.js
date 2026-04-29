import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../api/index.js'

export const useGb28181Store = defineStore('gb28181', () => {
  const stats    = ref({ total: 0, online: 0, sessions: 0 })
  const devices  = ref([])
  const sessions = ref([])
  const events   = ref([])   // 最近 200 条实时事件
  const loading  = ref(false)

  // SSE 连接
  let eventSource = null

  function connectSSE() {
    if (eventSource) return
    eventSource = new EventSource('/api/gb28181/events')
    eventSource.onmessage = (e) => {
      try {
        const ev = JSON.parse(e.data)
        ev._time = new Date().toLocaleTimeString()
        events.value.unshift(ev)
        if (events.value.length > 200) events.value.pop()

        // 收到设备相关事件时刷新设备列表
        if (['DeviceRegistered','DeviceUnregistered','DeviceOffline','DeviceOnline'].includes(ev.type)) {
          fetchDevices()
        }
        // 收到会话事件时刷新会话
        if (['SessionStarted','SessionEnded'].includes(ev.type)) {
          fetchSessions()
        }
        // 刷新统计
        fetchStats()
      } catch {}
    }
    eventSource.onerror = () => {
      setTimeout(() => {
        eventSource?.close()
        eventSource = null
        connectSSE()
      }, 3000)
    }
  }

  function disconnectSSE() {
    eventSource?.close()
    eventSource = null
  }

  async function fetchStats() {
    const res = await api.stats()
    if (res.data.code === 200) stats.value = res.data.data
  }

  async function fetchDevices() {
    loading.value = true
    try {
      const res = await api.devices()
      if (res.data.code === 200) devices.value = res.data.data
    } finally {
      loading.value = false
    }
  }

  async function fetchSessions() {
    const res = await api.sessions()
    if (res.data.code === 200) sessions.value = res.data.data
  }

  return {
    stats, devices, sessions, events, loading,
    connectSSE, disconnectSSE,
    fetchStats, fetchDevices, fetchSessions,
  }
})
