import { defineStore } from 'pinia'
import { ref } from 'vue'
import { loginApi, getUserInfoApi, logoutApi } from '@/api/auth'
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

  async function logout() {
    try {
      await logoutApi()
    } finally {
      token.value = ''
      userInfo.value = null
      permissions.value = []
      localStorage.removeItem('token')
    }
  }

  function hasPermission(perm: string): boolean {
    if (permissions.value.includes('*')) return true
    return permissions.value.includes(perm)
  }

  return { token, userInfo, permissions, login, fetchUserInfo, logout, hasPermission }
})
