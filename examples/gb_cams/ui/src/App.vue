<template>
  <div class="app">
    <nav class="navbar">
      <div class="nav-brand">📡 GB28181 设备模拟器</div>
      <div class="nav-links">
        <router-link to="/">总览</router-link>
        <router-link to="/devices">设备管理</router-link>
      </div>
    </nav>
    <main class="content">
      <router-view />
    </main>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import { useDeviceStore } from './stores/devices'

const store = useDeviceStore()
onMounted(() => {
  store.fetchStats()
  store.connectSSE()
})
</script>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f0f2f5; color: #333; }
.app { min-height: 100vh; }
.navbar {
  background: #1a1a2e;
  color: white;
  padding: 0 24px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
}
.nav-brand { font-size: 18px; font-weight: 600; }
.nav-links a {
  color: rgba(255,255,255,0.7);
  text-decoration: none;
  margin-left: 24px;
  font-size: 14px;
  transition: color 0.2s;
}
.nav-links a:hover, .nav-links a.router-link-active { color: white; }
.content { max-width: 1200px; margin: 0 auto; padding: 24px; }
</style>
