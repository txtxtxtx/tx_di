<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>设备管理</h1>
        <p>共 {{ store.devices.length }} 台设备，在线 {{ onlineCount }} 台</p>
      </div>
      <button class="btn btn-primary" @click="refresh" :disabled="store.loading">
        <span v-if="store.loading" class="spinner" style="width:14px;height:14px;border-width:2px"></span>
        <span>刷新</span>
      </button>
    </div>

    <!-- 过滤栏 -->
    <div class="card filter-bar">
      <input v-model="search" class="search-input" placeholder="搜索设备 ID / 厂商 / 地址..." />
      <div class="flex gap-8">
        <button
          v-for="f in filters" :key="f.value"
          :class="['btn btn-sm', filter === f.value ? 'btn-primary' : '']"
          @click="filter = f.value"
        >{{ f.label }}</button>
      </div>
    </div>

    <!-- 设备表格 -->
    <div class="card mt-16">
      <div v-if="filteredDevices.length === 0" class="empty-state">
        <div class="icon">📷</div>
        <p>{{ store.loading ? '加载中...' : '暂无设备' }}</p>
      </div>
      <div v-else class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>状态</th>
              <th>设备 ID</th>
              <th>厂商 / 型号</th>
              <th>地址</th>
              <th>通道数</th>
              <th>注册时间</th>
              <th style="text-align:right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="d in filteredDevices" :key="d.device_id">
              <td>
                <span :class="['badge', d.online ? 'badge-online' : 'badge-offline']">
                  <span :class="['dot', d.online ? 'dot-green' : 'dot-red']"></span>
                  {{ d.online ? '在线' : '离线' }}
                </span>
              </td>
              <td>
                <RouterLink :to="`/devices/${d.device_id}`" class="device-link">
                  {{ d.device_id }}
                </RouterLink>
              </td>
              <td>
                <span>{{ d.manufacturer || '—' }}</span>
                <span v-if="d.model" class="text-muted"> / {{ d.model }}</span>
              </td>
              <td>{{ d.remote_addr }}</td>
              <td>
                <span class="badge badge-session">{{ d.channel_count }}</span>
              </td>
              <td class="text-muted">{{ d.registered_at }}</td>
              <td>
                <div class="flex gap-8" style="justify-content:flex-end">
                  <button class="btn btn-sm" @click="queryCatalog(d.device_id)" :disabled="actionLoading[d.device_id]">
                    目录查询
                  </button>
                  <button class="btn btn-sm" @click="queryInfo(d.device_id)" :disabled="actionLoading[d.device_id]">
                    设备信息
                  </button>
                  <RouterLink :to="`/devices/${d.device_id}`" class="btn btn-sm btn-primary">
                    详情
                  </RouterLink>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useGb28181Store } from '../stores/gb28181.js'
import api from '../api/index.js'

const store = useGb28181Store()
const search = ref('')
const filter = ref('all')
const actionLoading = ref({})

const filters = [
  { label: '全部', value: 'all' },
  { label: '在线', value: 'online' },
  { label: '离线', value: 'offline' },
]

const onlineCount = computed(() => store.devices.filter(d => d.online).length)

const filteredDevices = computed(() => {
  let list = store.devices
  if (filter.value === 'online')  list = list.filter(d => d.online)
  if (filter.value === 'offline') list = list.filter(d => !d.online)
  if (search.value.trim()) {
    const q = search.value.toLowerCase()
    list = list.filter(d =>
      d.device_id.toLowerCase().includes(q) ||
      (d.manufacturer || '').toLowerCase().includes(q) ||
      (d.remote_addr || '').toLowerCase().includes(q)
    )
  }
  return list
})

async function refresh() {
  await Promise.all([store.fetchDevices(), store.fetchStats()])
}

async function queryCatalog(id) {
  actionLoading.value[id] = true
  try { await api.queryCatalog(id) } finally { actionLoading.value[id] = false }
}
async function queryInfo(id) {
  actionLoading.value[id] = true
  try { await api.queryInfo(id) } finally { actionLoading.value[id] = false }
}

onMounted(refresh)
</script>

<style scoped>
.filter-bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 16px; }
.search-input {
  flex: 1; max-width: 320px; padding: 7px 12px; border: 1px solid var(--border);
  border-radius: 6px; font-size: 13px; outline: none;
  transition: border-color .15s;
}
.search-input:focus { border-color: var(--primary); }
.device-link { color: var(--primary); font-weight: 500; }
.device-link:hover { text-decoration: underline; }
.text-muted { color: var(--text-muted); font-size: 12px; }
</style>
