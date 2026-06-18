import { defineStore } from 'pinia'
import { ref } from 'vue'
import { loginApi, getUserInfoApi, logoutApi } from '@/api/auth'
import { useMenuStore } from './menu'
import { resetDynamicRoutes } from '@/router'
import type { LoginRequest, UserInfoResponse } from '@/types'

export const useUserStore = defineStore('user', () => {
  const token = ref(localStorage.getItem('token') || '')
  const userInfo = ref<UserInfoResponse | null>(null)
  const permissions = ref<string[]>([])

  async function login(req: LoginRequest) {
    const res = await loginApi(req)
    token.value = res.data.token
    localStorage.setItem('token', res.data.token)
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
    localStorage.removeItem('token')
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
