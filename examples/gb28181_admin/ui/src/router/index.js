import { createRouter, createWebHistory } from 'vue-router'
import { useGb28181Store } from '../stores/gb28181.js'

const routes = [
  { path: '/',        redirect: '/dashboard' },
  { path: '/login',   name: 'Login', component: () => import('../views/LoginView.vue'), meta: { public: true } },
  { path: '/dashboard',       name: 'Dashboard',  component: () => import('../views/DashboardView.vue'),
    meta: { title: '概览', icon: '📊' } },
  { path: '/devices',        name: 'Devices',   component: () => import('../views/DevicesView.vue'),
    meta: { title: '设备管理', icon: '📷' } },
  { path: '/devices/:id',     name: 'DeviceDetail', component: () => import('../views/DeviceDetailView.vue'),
    meta: { title: '设备详情', hidden: true } },
  { path: '/groups',         name: 'Groups',    component: () => import('../views/GroupsView.vue'),
    meta: { title: '设备分组', icon: '📁' } },
  { path: '/audit',          name: 'Audit',     component: () => import('../views/AuditView.vue'),
    meta: { title: '注册审核', icon: '📋' } },
  { path: '/audit-logs',     name: 'AuditLogs', component: () => import('../views/AuditLogsView.vue'),
    meta: { title: '审计日志', icon: '📝' } },
  { path: '/sessions',      name: 'Sessions',  component: () => import('../views/SessionsView.vue'),
    meta: { title: '会话管理', icon: '🎬' } },
  { path: '/events',         name: 'Events',    component: () => import('../views/EventsView.vue'),
    meta: { title: '事件日志', icon: '🔔' } },
]

const router = createRouter({
  history: createWebHistory('/admin/'),
  routes,
})

// —— 全局路由守卫：未登录跳转登录页 ——
router.beforeEach((to) => {
  const token = localStorage.getItem('satoken')
  if (!to.meta?.public && !token) {
    return { path: '/login', query: { redirect: to.fullPath } }
  }
})

export default router
