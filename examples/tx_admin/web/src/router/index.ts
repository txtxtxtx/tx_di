import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import Layout from '@/layout/index.vue'
import type { MenuTreeNode } from '@/types'

// 静态路由：登录页和 Layout 壳子（用户中心始终可见）
const staticRoutes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { title: '登录' },
  },
  {
    path: '/',
    component: Layout,
    redirect: '/user',
    children: [
      {
        path: 'user',
        name: 'UserCenter',
        component: () => import('@/views/user/index.vue'),
        meta: { title: '用户中心', icon: 'User' },
      },
    ],
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes: staticRoutes,
})

// ── 动态路由 ──────────────────────────────────────────────────────

// Vite glob import：按需加载所有 views 下的 .vue 文件
const viewModules = import.meta.glob('@/views/**/*.vue')

// 记录动态添加的路由名称，用于登出时移除
const dynamicRouteNames: string[] = []

/**
 * 将后端菜单树转为 Vue Router 路由并动态注册
 *
 * 菜单树结构：目录节点（types=0）作为父路由，菜单节点（types=1）作为子路由。
 * component 字段存储相对路径，如 "system/user/index"，映射到 @/views/system/user/index.vue。
 */
export function addDynamicRoutes(menus: MenuTreeNode[]) {
  for (const menu of menus) {
    if (menu.types === 2) continue // 按钮不生成路由

    // 目录节点：作为 Layout 子路由的父级
    const parentPath = `/${menu.path || menu.id}`
    const children: RouteRecordRaw[] = []

    if (menu.children && menu.children.length > 0) {
      for (const child of menu.children) {
        if (child.types === 2) continue // 跳过按钮

        const childPath = child.path || child.id
        const componentPath = child.component
          ? `/src/views/${child.component}.vue`
          : undefined

        if (componentPath && viewModules[componentPath]) {
          children.push({
            path: childPath,
            name: child.componentName || `menu_${child.id}`,
            component: viewModules[componentPath] as () => Promise<any>,
            meta: {
              title: child.name,
              icon: child.icon || undefined,
              menuId: child.id,
              keepAlive: child.keepAlive === 1,
            },
          } as RouteRecordRaw)
        }
      }
    }

    // 如果目录自身也是菜单（有 component），也加入
    if (menu.types === 1 && menu.component) {
      const componentPath = `/src/views/${menu.component}.vue`
      if (viewModules[componentPath]) {
        children.unshift({
          path: '',
          name: menu.componentName || `menu_${menu.id}`,
          component: viewModules[componentPath] as () => Promise<any>,
          meta: {
            title: menu.name,
            icon: menu.icon || undefined,
            menuId: menu.id,
            keepAlive: menu.keepAlive === 1,
          },
        } as RouteRecordRaw)
      }
    }

    if (children.length === 0) continue

    const route: RouteRecordRaw = {
      path: parentPath,
      component: Layout,
      redirect: children[0] ? `${parentPath}/${children[0].path}` : undefined,
      meta: { title: menu.name, icon: menu.icon || undefined, menuId: menu.id },
      children,
    }

    router.addRoute(route)
    dynamicRouteNames.push(route.name as string)
  }
}

/**
 * 移除所有动态路由（登出时调用）
 */
export function resetDynamicRoutes() {
  for (const name of dynamicRouteNames) {
    router.removeRoute(name)
  }
  dynamicRouteNames.length = 0
}

// ── 路由守卫 ──────────────────────────────────────────────────────

// 白名单路径：不需要登录即可访问
const whiteList = ['/login']

router.beforeEach(async (to, _from, next) => {
  const token = localStorage.getItem('token')

  if (!token) {
    // 未登录：白名单放行，否则跳登录
    if (whiteList.includes(to.path)) {
      next()
    } else {
      next('/login')
    }
    return
  }

  // 已登录访问登录页 → 跳首页
  if (to.path === '/login') {
    next('/')
    return
  }

  // 动态路由是否已加载（通过 menuStore.loaded 判断）
  const { useMenuStore } = await import('@/stores/menu')
  const menuStore = useMenuStore()

  if (!menuStore.loaded) {
    try {
      const menus = await menuStore.fetchMenus()
      addDynamicRoutes(menus)
      // 动态路由已注册，用 replace 重新导航到目标路径
      // 使用 fullPath 确保完整路径匹配（to.path 可能缺少子路径）
      next({ path: to.fullPath, replace: true })
    } catch (err) {
      // 区分：菜单接口本身失败 vs 动态路由注册失败
      // 只有菜单接口失败（token 过期等）才清空状态跳登录
      console.error('[router] 加载菜单或注册动态路由失败:', err)
      if (!menuStore.loaded) {
        // fetchMenus 就失败了 → token 可能过期，清空跳登录
        const { useUserStore } = await import('@/stores/user')
        const userStore = useUserStore()
        userStore.logout()
        next('/login')
      } else {
        // fetchMenus 成功但 addDynamicRoutes 失败 → 菜单数据有问题，仍然放行到首页
        console.warn('[router] 动态路由注册失败，将使用静态路由')
        next({ path: to.fullPath, replace: true })
      }
    }
    return
  }

  // 动态路由已加载，检查目标路由是否存在
  if (to.matched.length === 0) {
    // 路由不存在（可能是旧的书签/链接），跳首页
    next('/')
    return
  }

  next()
})

export default router
