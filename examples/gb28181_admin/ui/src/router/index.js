import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '../views/DashboardView.vue'

const routes = [
  { path: '/',        redirect: '/dashboard' },
  { path: '/dashboard', name: 'Dashboard', component: DashboardView,
    meta: { title: '概览', icon: '📊' } },
  { path: '/devices',   name: 'Devices',
    component: () => import('../views/DevicesView.vue'),
    meta: { title: '设备管理', icon: '📷' } },
  { path: '/devices/:id', name: 'DeviceDetail',
    component: () => import('../views/DeviceDetailView.vue'),
    meta: { title: '设备详情', hidden: true } },
  { path: '/sessions', name: 'Sessions',
    component: () => import('../views/SessionsView.vue'),
    meta: { title: '会话管理', icon: '🎬' } },
  { path: '/events',   name: 'Events',
    component: () => import('../views/EventsView.vue'),
    meta: { title: '事件日志', icon: '🔔' } },
]

export default createRouter({
  history: createWebHistory('/admin/'),
  routes,
})
