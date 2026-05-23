import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '../api/index.js'

export const useGb28181Store = defineStore('gb28181', () => {
  // —— 现有状态 ——
  const stats    = ref({ total: 0, online: 0, sessions: 0 })
  const devices  = ref([])
  const sessions = ref([])
  const events   = ref([])
  const loading  = ref(false)

  // —— 新增：仪表盘增强数据 ——
  const dashboard = ref({
    total_devices:   0,
    online_devices:   0,
    active_sessions:  0,
    total_alarms:     0,
    pending_alarms:   0,
    handled_alarms:   0,
  })

  // —— 新增：分组（树形）──
  const groups  = ref([])
  const members = ref([])

  // —— 新增：注册审核 ——
  const audits = ref([])

  // SSE 连接
  let eventSource = null

  // ═════════════
  //  SSE
  // ═════════════
  function connectSSE() {
    if (eventSource) return
    eventSource = new EventSource('/api/v1/gb28181/events')
    eventSource.onmessage = (e) => {
      try {
        const ev = JSON.parse(e.data)
        ev._time = new Date().toLocaleTimeString()
        events.value.unshift(ev)
        if (events.value.length > 200) events.value.pop()
        if (['DeviceRegistered','DeviceUnregistered','DeviceOffline','DeviceOnline'].includes(ev.type)) {
          fetchDevices()
        }
        if (['SessionStarted','SessionEnded'].includes(ev.type)) {
          fetchSessions()
        }
        fetchStats()
        fetchDashboard()
      } catch {}
    }
    eventSource.onerror = () => {
      setTimeout(() => { eventSource?.close(); eventSource = null; connectSSE() }, 3000)
    }
  }

  function disconnectSSE() {
    eventSource?.close()
    eventSource = null
  }

  // ═════════════
  //  统计
  // ═════════════
  async function fetchStats() {
    const res = await api.stats()
    if (res.data.code === 200) stats.value = res.data.data
  }

  async function fetchDashboard() {
    try {
      const res = await api.dashboard()
      if (res.data.code === 200) dashboard.value = res.data.data
    } catch {}
  }

  // ═════════════
  //  设备
  // ═════════════
  async function fetchDevices() {
    loading.value = true
    try {
      const res = await api.devices()
      if (res.data.code === 200) devices.value = res.data.data
    } finally { loading.value = false }
  }

  // ═════════════
  //  会话
  // ═════════════
  async function fetchSessions() {
    try {
      const res = await api.sessions()
      if (res.data.code === 200) sessions.value = res.data.data
    } catch {}
  }

  // ═════════════
  //  分组管理
  // ═════════════
  async function fetchGroups() {
    try {
      const res = await api.groupList()
      if (res.data.code === 200) groups.value = res.data.data
    } catch {}
  }

  async function fetchMembers(groupId) {
    try {
      const res = await api.groupMembers(groupId)
      if (res.data.code === 200) members.value = res.data.data
    } catch {}
  }

  // ═════════════
  //  注册审核
  // ═════════════
  async function fetchAudits() {
    try {
      const res = await api.auditList()
      if (res.data.code === 200) audits.value = res.data.data
    } catch {}
  }

  return {
    stats, devices, sessions, events, loading, dashboard,
    groups, members, audits,
    connectSSE, disconnectSSE,
    fetchStats, fetchDashboard, fetchDevices, fetchSessions,
    fetchGroups, fetchMembers, fetchAudits,
  }
})
