import request from './request'
import type { ApiRes, LoginRequest, LoginResponse, UserInfoResponse, MenuTreeNode } from '@/types'

export function loginApi(data: LoginRequest) {
  return request.post<ApiRes<LoginResponse>>('/api/v1/auth/login', data).then(r => r.data)
}

export function getUserInfoApi() {
  return request.get<ApiRes<UserInfoResponse>>('/api/v1/auth/user_info').then(r => r.data)
}

export function getUserMenusApi() {
  return request.get<ApiRes<MenuTreeNode[]>>('/api/v1/auth/menus').then(r => r.data)
}

export function logoutApi() {
  return request.post<ApiRes<null>>('/api/v1/auth/logout').then(r => r.data)
}
