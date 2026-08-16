import request from './request'
import type { ApiRes, MenuTreeNode, CreateMenuRequest, UpdateMenuRequest, ListMenusRequest } from '@/types'

export function createMenu(data: CreateMenuRequest) {
  return request.post<ApiRes<MenuTreeNode>>('/api/v1/menu', data).then(r => r.data)
}

export function getMenu(menuId: string) {
  return request.get<ApiRes<MenuTreeNode>>(`/api/v1/menu/${menuId}`).then(r => r.data)
}

export function updateMenu(menuId: string, data: UpdateMenuRequest) {
  return request.put<ApiRes<MenuTreeNode>>(`/api/v1/menu/${menuId}`, { ...data, menuId }).then(r => r.data)
}

export function deleteMenu(menuId: string) {
  return request.delete<ApiRes<null>>(`/api/v1/menu/${menuId}`).then(r => r.data)
}

export function listMenus(data?: ListMenusRequest) {
  return request.post<ApiRes<MenuTreeNode[]>>('/api/v1/menu/list', data || {}).then(r => r.data)
}
