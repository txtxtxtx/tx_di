import request from './request'
import type { ApiRes, PageData, UserResponse, CreateUserRequest, UpdateUserRequest, ListUsersRequest, ChangePasswordRequest, AssignRolesRequest, AssignDeptsRequest, UserIdRequest } from '@/types'

export function createUser(data: CreateUserRequest) {
  return request.post<ApiRes<UserResponse>>('/api/user/', data).then(r => r.data)
}

export function getUser(userId: number) {
  return request.get<ApiRes<UserResponse>>(`/api/user/${userId}`).then(r => r.data)
}

export function updateUser(userId: number, data: UpdateUserRequest) {
  return request.put<ApiRes<UserResponse>>(`/api/user/${userId}`, data).then(r => r.data)
}

export function deleteUser(userId: number) {
  return request.delete<ApiRes<null>>(`/api/user/${userId}`).then(r => r.data)
}

export function listUsers(data: ListUsersRequest) {
  return request.post<ApiRes<PageData<UserResponse>>>('/api/user/list', data).then(r => r.data)
}

export function changePassword(data: ChangePasswordRequest) {
  return request.post<ApiRes<null>>('/api/user/change_password', data).then(r => r.data)
}

export function assignRoles(data: AssignRolesRequest) {
  return request.post<ApiRes<null>>('/api/user/assign_roles', data).then(r => r.data)
}

export function assignDepts(data: AssignDeptsRequest) {
  return request.post<ApiRes<null>>('/api/user/assign_depts', data).then(r => r.data)
}

export function enableUser(data: UserIdRequest) {
  return request.post<ApiRes<null>>('/api/user/enable', data).then(r => r.data)
}

export function disableUser(data: UserIdRequest) {
  return request.post<ApiRes<null>>('/api/user/disable', data).then(r => r.data)
}

export function lockUser(data: UserIdRequest) {
  return request.post<ApiRes<null>>('/api/user/lock', data).then(r => r.data)
}

export function unlockUser(data: UserIdRequest) {
  return request.post<ApiRes<null>>('/api/user/unlock', data).then(r => r.data)
}
