import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '../api'

export const useDeviceStore = defineStore('devices', () => {
  const stats = ref({ total: 0, online: 0, channels: 0 })
  const devices = ref([])
  const events = ref([])
  const loading = ref(false)

  async function fetchStats() {
    const { data } = await api.stats()
    if (data.code === 0) stats.value = data.data
  }

  async function fetchDevices() {
    loading.value = true
    try {
      const { data } = await api.devices()
      if (data.code === 0) devices.value = data.data
    } finally {
      loading.value = false
    }
  }

  async function fetchDevice(id) {
    const { data } = await api.device(id)
    return data.code === 0 ? data.data : null
  }

  async function generateDevices(opts) {
    const { data } = await api.generate(opts)
    return data
  }

  async function removeDevice(id) {
    const { data } = await api.remove(id)
    return data
  }

  function connectSSE() {
    const source = new EventSource('/api/gb_cams/events')
    source.onmessage = (e) => {
      try {
        const ev = JSON.parse(e.data)
        events.value.unshift(ev)
        if (events.value.length > 200) events.value.pop()
        // 刷新统计
        fetchStats()
      } catch {}
    }
    source.onerror = () => {
      setTimeout(() => connectSSE(), 3000)
    }
  }

  return { stats, devices, events, loading, fetchStats, fetchDevices, fetchDevice, generateDevices, removeDevice, connectSSE }
})
