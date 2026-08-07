import { defineStore } from 'pinia'
import { ref } from 'vue'
import { loginApi, getUserInfoApi, logoutApi } from '@/api/auth'
import { useMenuStore } from './menu'
import { resetDynamicRoutes } from '@/router'
import type { LoginRequest, UserInfoResponse } from '@/types'

export const useUserStore = defineStore('user', () => {
  // 阶段 E-2：token 由后端写入 HttpOnly Cookie（前端不可读、防 XSS 窃取），
  // 此处仅缓存 userInfo 判断登录态，不再存储 token。
  const token = ref('')
  const userInfo = ref<UserInfoResponse | null>(null)
  const permissions = ref<string[]>([])

  async function login(req: LoginRequest) {
    const res = await loginApi(req)
    // token 在 HttpOnly Cookie 中，前端不保存
    token.value = ''
    userInfo.value = res.data
    permissions.value = res.data.permissions || []
    return res
  }

  async function fetchUserInfo() {
    const res = await getUserInfoApi()
    userInfo.value = res.data
    permissions.value = res.data.permissions || []
    return res.data
  }

  /** 清除所有认证数据（不调接口，供 401 拦截器复用） */
  function clearAuthData() {
    token.value = ''
    userInfo.value = null
    permissions.value = []
    const menuStore = useMenuStore()
    menuStore.clearMenus()
    resetDynamicRoutes()
  }

  async function logout() {
    try {
      await logoutApi()
    } finally {
      clearAuthData()
    }
  }

  function hasPermission(perm: string): boolean {
    if (permissions.value.includes('*')) return true
    return permissions.value.includes(perm)
  }

  return { token, userInfo, permissions, login, fetchUserInfo, logout, clearAuthData, hasPermission }
})
