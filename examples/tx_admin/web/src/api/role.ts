import request from './request'
import type { ApiRes, PageData, RoleResponse, CreateRoleRequest, UpdateRoleRequest, ListRolesRequest, AssignMenusRequest, UserResponse } from '@/types'

export function createRole(data: CreateRoleRequest) {
  return request.post<ApiRes<RoleResponse>>('/api/role/', data).then(r => r.data)
}

export function getRole(roleId: number) {
  return request.get<ApiRes<RoleResponse>>(`/api/role/${roleId}`).then(r => r.data)
}

export function updateRole(roleId: number, data: UpdateRoleRequest) {
  return request.put<ApiRes<RoleResponse>>(`/api/role/${roleId}`, data).then(r => r.data)
}

export function deleteRole(roleId: number) {
  return request.delete<ApiRes<null>>(`/api/role/${roleId}`).then(r => r.data)
}

export function listRoles(data: ListRolesRequest) {
  return request.post<ApiRes<PageData<RoleResponse>>>('/api/role/list', data).then(r => r.data)
}

export function assignMenus(data: AssignMenusRequest) {
  return request.post<ApiRes<null>>('/api/role/assign-menus', data).then(r => r.data)
}

export function getAllRoles() {
  return request.get<ApiRes<RoleResponse[]>>('/api/role/all').then(r => r.data)
}

export function getRoleUsers(roleId: number) {
  return request.get<ApiRes<UserResponse[]>>(`/api/role/${roleId}/users`).then(r => r.data)
}

export function addUsersToRole(roleId: number, userIds: number[]) {
  return request.post<ApiRes<null>>(`/api/role/${roleId}/users`, userIds).then(r => r.data)
}

export function removeUsersFromRole(roleId: number, userIds: number[]) {
  return request.delete<ApiRes<null>>(`/api/role/${roleId}/users`, { data: userIds }).then(r => r.data)
}
