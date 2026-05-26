<template>
  <div class="layout">
    <!-- 侧边栏（仅登录后显示） -->
    <aside v-if="isLoggedIn" class="sidebar">
      <div class="sidebar-logo">
        <span class="logo-icon">📡</span>
        <span class="logo-text">GB28181</span>
      </div>
      <nav class="sidebar-nav">
        <RouterLink
          v-for="r in navRoutes" :key="r.path"
          :to="r.path"
          class="nav-item"
          active-class="nav-item--active"
        >
          <span class="nav-icon">{{ r.meta.icon }}</span>
          <span>{{ r.meta.title }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-footer">
        <span :class="['dot', sseConnected ? 'dot-green' : 'dot-red']" style="margin-right:6px"></span>
        <span>{{ sseConnected ? '实时连接中' : '连接断开' }}</span>
      </div>
    </aside>

    <!-- 主内容 -->
    <main :class="isLoggedIn ? 'main-content' : 'main-content--full'">
      <RouterView />
    </main>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useGb28181Store } from './stores/gb28181.js'

const router = useRouter()
const route  = useRoute()
const store = useGb28181Store()
const sseConnected = ref(false)

// 登录状态（ref，保证响应式）
const isLoggedIn = ref(!!localStorage.getItem('satoken'))

// 初始化：连接 SSE + 拉取数据（仅登录后执行）
function initApp() {
  if (!isLoggedIn.value) return
  store.connectSSE()
  store.fetchStats()
  store.fetchDevices()
  store.fetchSessions()
  sseConnected.value = true
}

// 启动时执行一次
onMounted(() => {
  initApp()
  window.addEventListener('storage', syncLoginState)
})
onUnmounted(() => {
  window.removeEventListener('storage', syncLoginState)
  store.disconnectSSE()
})

// 跨标签页 localStorage 变化同步
function syncLoginState() {
  isLoggedIn.value = !!localStorage.getItem('satoken')
}

// 路由变化时同步登录状态（同标签页登录后 router.replace 会触发）
watch(() => route.fullPath, () => {
  isLoggedIn.value = !!localStorage.getItem('satoken')
  // 如果刚登录成功，补执行初始化
  if (isLoggedIn.value && !sseConnected.value) {
    initApp()
  }
})

// 登录状态变为 true 时执行初始化
watch(isLoggedIn, (val) => {
  if (val) initApp()
})

const navRoutes = computed(() =>
  router.getRoutes().filter(r => r.meta?.icon)
)
</script>

<style>
.layout { display: flex; min-height: 100vh; }

.sidebar {
  width: 200px; flex-shrink: 0;
  background: var(--sidebar-bg);
  display: flex; flex-direction: column;
  position: sticky; top: 0; height: 100vh;
}

.sidebar-logo {
  display: flex; align-items: center; gap: 10px;
  padding: 20px 18px 16px;
  color: #fff; font-size: 16px; font-weight: 600;
  border-bottom: 1px solid rgba(255,255,255,.07);
}
.logo-icon { font-size: 22px; }

.sidebar-nav { flex: 1; padding: 12px 0; overflow-y: auto; }

.nav-item {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 18px; color: var(--sidebar-text);
  font-size: 13.5px; transition: all .15s;
  border-left: 3px solid transparent;
}
.nav-item:hover { color: #fff; background: rgba(255,255,255,.05); }
.nav-item--active {
  color: #fff; background: rgba(78,142,247,.15);
  border-left-color: var(--sidebar-active);
}
.nav-icon { width: 18px; text-align: center; }

.sidebar-footer {
  padding: 14px 18px; font-size: 12px;
  color: var(--sidebar-text);
  border-top: 1px solid rgba(255,255,255,.07);
  display: flex; align-items: center;
}

.main-content {
  flex: 1; min-width: 0;
  padding: 28px 32px;
  overflow-y: auto;
}

/* 全宽（登录页等场景） */
.main-content--full {
  flex: 1; min-width: 0;
}
</style>
