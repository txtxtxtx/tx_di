import request from './request'
import type { ApiRes, PermissionDetail, CreatePermissionRequest, UpdatePermissionRequest, PermissionCheckRequest, PermissionCheckResponse, UserPermissionsResponse } from '@/types'

export function createPermission(data: CreatePermissionRequest) {
  return request.post<ApiRes<PermissionDetail>>('/api/permission', data).then(r => r.data)
}

export function getPermission(id: string) {
  return request.get<ApiRes<PermissionDetail>>(`/api/permission/${id}`).then(r => r.data)
}

export function updatePermission(id: string, data: UpdatePermissionRequest) {
  return request.put<ApiRes<PermissionDetail>>(`/api/permission/${id}`, { ...data, id }).then(r => r.data)
}

export function deletePermission(id: string) {
  return request.delete<ApiRes<null>>(`/api/permission/${id}`).then(r => r.data)
}

export function listPermissions() {
  return request.get<ApiRes<{ permissions: PermissionDetail[] }>>('/api/permission/list').then(r => r.data)
}

export function getAllPermissions() {
  return request.get<ApiRes<{ code: string; name: string; permissionType: string }[]>>('/api/permission/all').then(r => r.data)
}

export function checkPermission(data: PermissionCheckRequest) {
  return request.post<ApiRes<PermissionCheckResponse>>('/api/permission/check', data).then(r => r.data)
}

export function getUserPermissions(userId: string) {
  return request.post<ApiRes<UserPermissionsResponse>>('/api/permission/user_permissions', { userId }).then(r => r.data)
}
