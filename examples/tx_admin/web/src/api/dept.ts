import request from './request'
import type { ApiRes, DeptTreeNode, CreateDeptRequest, UpdateDeptRequest, ListDeptsRequest } from '@/types'

export function createDept(data: CreateDeptRequest) {
  return request.post<ApiRes<DeptTreeNode>>('/api/dept/', data).then(r => r.data)
}

export function getDept(deptId: string) {
  return request.get<ApiRes<DeptTreeNode>>(`/api/dept/${deptId}`).then(r => r.data)
}

export function updateDept(deptId: string, data: UpdateDeptRequest) {
  return request.put<ApiRes<DeptTreeNode>>(`/api/dept/${deptId}`, data).then(r => r.data)
}

export function deleteDept(deptId: string) {
  return request.delete<ApiRes<null>>(`/api/dept/${deptId}`).then(r => r.data)
}

export function listDepts(data?: ListDeptsRequest) {
  return request.post<ApiRes<DeptTreeNode[]>>('/api/dept/list', data || {}).then(r => r.data)
}
