import request from './request'
import type { ApiRes, LoginRequest, LoginResponse, UserInfoResponse } from '@/types'

export function loginApi(data: LoginRequest) {
  return request.post<ApiRes<LoginResponse>>('/api/auth/login', data).then(r => r.data)
}

export function getUserInfoApi() {
  return request.get<ApiRes<UserInfoResponse>>('/api/auth/user-info').then(r => r.data)
}

export function logoutApi() {
  return request.post<ApiRes<null>>('/api/auth/logout').then(r => r.data)
}
