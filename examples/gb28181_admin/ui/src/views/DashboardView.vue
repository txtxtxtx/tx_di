<template>
  <div>
    <div class="page-header">
      <h1>概览</h1>
      <p>GB28181 平台运行状态一览</p>
    </div>

    <!-- 统计卡片行 1：设备 + 会话 -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon">📷</div>
        <div class="stat-body">
          <div class="stat-label">全部设备</div>
          <div class="stat-value">{{ db.total_devices || store.stats.total }}</div>
        </div>
      </div>
      <div class="stat-card stat-card--green">
        <div class="stat-icon">🟢</div>
        <div class="stat-body">
          <div class="stat-label">在线设备</div>
          <div class="stat-value">{{ db.online_devices || store.stats.online }}</div>
        </div>
      </div>
      <div class="stat-card stat-card--blue">
        <div class="stat-icon">📡</div>
        <div class="stat-body">
          <div class="stat-label">活跃会话</div>
          <div class="stat-value">{{ db.active_sessions || store.stats.sessions }}</div>
        </div>
      </div>

      <!-- 报警统计卡片 -->
      <div class="stat-card stat-card--amber">
        <div class="stat-icon">🔔</div>
        <div class="stat-body">
          <div class="stat-label">报警总数</div>
          <div class="stat-value">{{ db.total_alarms }}</div>
        </div>
      </div>
      <div class="stat-card stat-card--red" @click="goAlarms(0)" style="cursor:pointer">
        <div class="stat-icon">🚨</div>
        <div class="stat-body">
          <div class="stat-label">未处理报警</div>
          <div class="stat-value">{{ db.pending_alarms }}</div>
          <div v-if="db.pending_alarms > 0" class="stat-hint">点击处理 →</div>
        </div>
      </div>
      <div class="stat-card stat-card--teal">
        <div class="stat-icon">✅</div>
        <div class="stat-body">
          <div class="stat-label">已处理报警</div>
          <div class="stat-value">{{ db.handled_alarms }}</div>
        </div>
      </div>

      <!-- 实时事件计数 -->
      <div class="stat-card">
        <div class="stat-icon">📌</div>
        <div class="stat-body">
          <div class="stat-label">最新事件</div>
          <div class="stat-value">{{ store.events.length }}</div>
        </div>
      </div>
    </div>

    <!-- 报警警告横幅（有未处理报警时显示） -->
    <div v-if="db.pending_alarms > 0" class="alarm-banner mt-16">
      <span class="alarm-banner-icon">⚠️</span>
      <span>当前有 <strong>{{ db.pending_alarms }}</strong> 条未处理报警</span>
      <RouterLink to="/alarms" class="alarm-banner-link">立即处理</RouterLink>
    </div>

    <!-- 报警趋势：近7日（简易柱状图） -->
    <div class="card mt-16" v-if="db.total_alarms > 0">
      <div class="flex items-center justify-between" style="margin-bottom:14px">
        <h2 class="section-title">报警状态分布</h2>
        <RouterLink to="/alarms" class="btn btn-sm">全部报警</RouterLink>
      </div>
      <div class="alarm-progress-row">
        <div class="alarm-progress-label">未处理</div>
        <div class="alarm-progress-bar">
          <div
            class="alarm-progress-fill alarm-progress-fill--red"
            :style="{ width: alarmPercent(db.pending_alarms) }"
          ></div>
        </div>
        <div class="alarm-progress-value danger">{{ db.pending_alarms }}</div>
      </div>
      <div class="alarm-progress-row">
        <div class="alarm-progress-label">已处理</div>
        <div class="alarm-progress-bar">
          <div
            class="alarm-progress-fill alarm-progress-fill--green"
            :style="{ width: alarmPercent(db.handled_alarms) }"
          ></div>
        </div>
        <div class="alarm-progress-value success">{{ db.handled_alarms }}</div>
      </div>
    </div>

    <!-- 最近事件 + 在线设备 -->
    <div class="dashboard-grid mt-16">
      <!-- 在线设备 -->
      <div class="card">
        <div class="flex items-center justify-between" style="margin-bottom:14px">
          <h2 class="section-title">在线设备</h2>
          <RouterLink to="/devices" class="btn btn-sm">全部</RouterLink>
        </div>
        <div v-if="onlineDevices.length === 0" class="empty-state">
          <div class="icon">📷</div>
          <p>暂无在线设备</p>
        </div>
        <div v-else class="device-mini-list">
          <div v-for="d in onlineDevices.slice(0,8)" :key="d.device_id" class="device-mini-item">
            <span class="dot dot-green" style="margin-right:8px;flex-shrink:0"></span>
            <div class="truncate">
              <RouterLink :to="`/devices/${d.device_id}`" class="device-mini-id">
                {{ d.device_id }}
              </RouterLink>
              <div class="device-mini-meta">{{ d.manufacturer || '未知厂商' }} · {{ d.channel_count }} 通道</div>
            </div>
            <span style="margin-left:auto;color:var(--text-muted);font-size:12px;flex-shrink:0">{{ d.remote_addr }}</span>
          </div>
        </div>
      </div>

      <!-- 最近事件 -->
      <div class="card">
        <div class="flex items-center justify-between" style="margin-bottom:14px">
          <h2 class="section-title">实时事件</h2>
          <RouterLink to="/events" class="btn btn-sm">全部</RouterLink>
        </div>
        <div v-if="store.events.length === 0" class="empty-state">
          <div class="icon">🔔</div>
          <p>暂无事件</p>
        </div>
        <div v-else class="event-mini-list">
          <div v-for="(ev, i) in store.events.slice(0, 10)" :key="i" class="event-mini-item">
            <span :class="['event-dot', eventColor(ev.type)]"></span>
            <div class="truncate">
              <span class="event-type">{{ ev.type }}</span>
              <span class="event-id">{{ ev.device_id }}</span>
            </div>
            <span class="event-time">{{ ev._time }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useGb28181Store } from '../stores/gb28181.js'

const store  = useGb28181Store()
const router = useRouter()

// 仪表盘数据快捷引用
const db = computed(() => store.dashboard)

const onlineDevices = computed(() => store.devices.filter(d => d.online))

function eventColor(type) {
  if (type.includes('Alarm'))      return 'dot-danger'
  if (type.includes('Offline'))    return 'dot-warning'
  if (type.includes('Registered') || type.includes('Online')) return 'dot-success'
  return 'dot-info'
}

function goAlarms(status) {
  router.push({ path: '/alarms', query: { status } })
}

function alarmPercent(n) {
  const total = db.value.total_alarms
  if (!total) return '0%'
  return Math.round((n / total) * 100) + '%'
}

onMounted(() => {
  store.fetchStats()
  store.fetchDashboard()
  store.fetchDevices()
})
</script>

<style scoped>
/* ─── 统计卡片 ─── */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 12px;
}
.stat-card {
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px 18px;
  border-top: 3px solid var(--border);
  display: flex;
  gap: 12px;
  align-items: flex-start;
  transition: box-shadow .15s;
}
.stat-card:hover { box-shadow: 0 2px 12px rgba(0,0,0,.08); }
.stat-card--green { border-top-color: var(--success); }
.stat-card--blue  { border-top-color: var(--primary); }
.stat-card--amber { border-top-color: var(--warning); }
.stat-card--red   { border-top-color: var(--danger); }
.stat-card--teal  { border-top-color: #14b8a6; }
.stat-icon { font-size: 22px; margin-top: 2px; flex-shrink: 0; }
.stat-body { flex: 1; min-width: 0; }
.stat-label { font-size: 11px; color: var(--text-muted); margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.stat-value { font-size: 26px; font-weight: 700; color: var(--text); line-height: 1; }
.stat-hint  { font-size: 11px; color: var(--danger); margin-top: 4px; }

/* ─── 报警横幅 ─── */
.alarm-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  background: #fff7ed;
  border: 1px solid #fed7aa;
  border-left: 4px solid var(--danger);
  border-radius: var(--radius);
  padding: 12px 16px;
  font-size: 14px;
  color: #9a3412;
}
.alarm-banner-icon { font-size: 18px; }
.alarm-banner-link {
  margin-left: auto;
  color: var(--danger);
  font-weight: 600;
  text-decoration: none;
  padding: 4px 12px;
  border: 1px solid var(--danger);
  border-radius: 6px;
  font-size: 13px;
}
.alarm-banner-link:hover { background: var(--danger); color: #fff; }

/* ─── 报警进度条 ─── */
.alarm-progress-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}
.alarm-progress-label {
  width: 60px;
  font-size: 13px;
  color: var(--text-muted);
  flex-shrink: 0;
}
.alarm-progress-bar {
  flex: 1;
  height: 8px;
  background: var(--border);
  border-radius: 4px;
  overflow: hidden;
}
.alarm-progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width .4s ease;
}
.alarm-progress-fill--red   { background: var(--danger); }
.alarm-progress-fill--green { background: var(--success); }
.alarm-progress-value {
  width: 40px;
  text-align: right;
  font-size: 13px;
  font-weight: 600;
  flex-shrink: 0;
}
.alarm-progress-value.danger  { color: var(--danger); }
.alarm-progress-value.success { color: var(--success); }

/* ─── 主网格 ─── */
.dashboard-grid {
  display: grid; grid-template-columns: 1fr 1fr; gap: 20px;
}
.section-title { font-size: 15px; font-weight: 600; }

/* ─── 设备列表 ─── */
.device-mini-list { display: flex; flex-direction: column; gap: 2px; }
.device-mini-item {
  display: flex; align-items: center;
  padding: 8px 6px; border-radius: 6px;
  transition: background .1s;
}
.device-mini-item:hover { background: var(--bg); }
.device-mini-id   { font-size: 13px; font-weight: 500; color: var(--primary); }
.device-mini-meta { font-size: 12px; color: var(--text-muted); }

/* ─── 事件列表 ─── */
.event-mini-list { display: flex; flex-direction: column; gap: 2px; }
.event-mini-item {
  display: flex; align-items: center; gap: 8px;
  padding: 7px 6px; border-radius: 6px;
}
.event-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
.dot-danger  { background: var(--danger); }
.dot-warning { background: var(--warning); }
.dot-success { background: var(--success); }
.dot-info    { background: var(--primary); }
.event-type  { font-size: 13px; font-weight: 500; margin-right: 6px; }
.event-id    { font-size: 12px; color: var(--text-muted); }
.event-time  { margin-left: auto; font-size: 11px; color: var(--text-muted); white-space: nowrap; }

/* ─── 工具类 ─── */
.mt-16 { margin-top: 16px; }

@media (max-width: 1200px) {
  .stats-grid { grid-template-columns: repeat(4, 1fr); }
}
@media (max-width: 900px) {
  .stats-grid    { grid-template-columns: repeat(2, 1fr); }
  .dashboard-grid { grid-template-columns: 1fr; }
}
</style>
