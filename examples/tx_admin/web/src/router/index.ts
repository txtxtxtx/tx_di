import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Layout from '@/layout/index.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { title: '登录' },
  },
  {
    path: '/',
    component: Layout,
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/dashboard/index.vue'),
        meta: { title: '仪表盘', icon: 'Odometer' },
      },
    ],
  },
  {
    path: '/system',
    component: Layout,
    redirect: '/system/user',
    meta: { title: '系统管理', icon: 'Setting' },
    children: [
      {
        path: 'user',
        name: 'User',
        component: () => import('@/views/system/user/index.vue'),
        meta: { title: '用户管理', icon: 'User' },
      },
      {
        path: 'role',
        name: 'Role',
        component: () => import('@/views/system/role/index.vue'),
        meta: { title: '角色管理', icon: 'UserFilled' },
      },
      {
        path: 'menu',
        name: 'Menu',
        component: () => import('@/views/system/menu/index.vue'),
        meta: { title: '菜单管理', icon: 'Menu' },
      },
      {
        path: 'dept',
        name: 'Dept',
        component: () => import('@/views/system/dept/index.vue'),
        meta: { title: '部门管理', icon: 'OfficeBuilding' },
      },
      {
        path: 'permission',
        name: 'Permission',
        component: () => import('@/views/system/permission/index.vue'),
        meta: { title: '权限管理', icon: 'Lock' },
      },
    ],
  },
  {
    path: '/config',
    component: Layout,
    redirect: '/config/index',
    meta: { title: '系统配置', icon: 'Tools' },
    children: [
      {
        path: 'index',
        name: 'Config',
        component: () => import('@/views/config/config/index.vue'),
        meta: { title: '参数设置', icon: 'Document' },
      },
      {
        path: 'dict-type',
        name: 'DictType',
        component: () => import('@/views/config/dict/type.vue'),
        meta: { title: '字典类型', icon: 'Collection' },
      },
      {
        path: 'dict-data',
        name: 'DictData',
        component: () => import('@/views/config/dict/data.vue'),
        meta: { title: '字典数据', icon: 'Tickets' },
      },
    ],
  },
  {
    path: '/log',
    component: Layout,
    redirect: '/log/operate',
    meta: { title: '日志管理', icon: 'Notebook' },
    children: [
      {
        path: 'operate',
        name: 'OperateLog',
        component: () => import('@/views/log/operate.vue'),
        meta: { title: '操作日志', icon: 'List' },
      },
      {
        path: 'login',
        name: 'LoginLog',
        component: () => import('@/views/log/login.vue'),
        meta: { title: '登录日志', icon: 'Promotion' },
      },
    ],
  },
  {
    path: '/file',
    component: Layout,
    children: [
      {
        path: '',
        name: 'File',
        component: () => import('@/views/file/index.vue'),
        meta: { title: '文件管理', icon: 'FolderOpened' },
      },
    ],
  },
  {
    path: '/monitor',
    component: Layout,
    redirect: '/monitor/server',
    meta: { title: '系统监控', icon: 'Monitor' },
    children: [
      {
        path: 'server',
        name: 'Server',
        component: () => import('@/views/monitor/server.vue'),
        meta: { title: '服务器信息', icon: 'Cpu' },
      },
      {
        path: 'online',
        name: 'Online',
        component: () => import('@/views/monitor/online.vue'),
        meta: { title: '在线用户', icon: 'Connection' },
      },
    ],
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

// 路由守卫
router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('token')
  if (to.path === '/login') {
    next()
  } else if (!token) {
    next('/login')
  } else {
    next()
  }
})

export default router
