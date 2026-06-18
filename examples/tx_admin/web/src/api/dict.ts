import request from './request'
import type { ApiRes, PageData, DictTypeResponse, DictDataResponse, CreateDictTypeRequest, UpdateDictTypeRequest, ListDictTypesRequest, CreateDictDataRequest, UpdateDictDataRequest, ListDictDataRequest } from '@/types'

// 字典类型
export function createDictType(data: CreateDictTypeRequest) {
  return request.post<ApiRes<DictTypeResponse>>('/api/dict/type', data).then(r => r.data)
}

export function getDictType(id: string) {
  return request.get<ApiRes<DictTypeResponse>>(`/api/dict/type/${id}`).then(r => r.data)
}

export function updateDictType(id: string, data: UpdateDictTypeRequest) {
  return request.put<ApiRes<DictTypeResponse>>(`/api/dict/type/${id}`, { ...data, id }).then(r => r.data)
}

export function deleteDictType(id: string) {
  return request.delete<ApiRes<null>>(`/api/dict/type/${id}`).then(r => r.data)
}

export function listDictTypes(data: ListDictTypesRequest) {
  return request.post<ApiRes<PageData<DictTypeResponse>>>('/api/dict/type/list', data).then(r => r.data)
}

// 字典数据
export function createDictData(data: CreateDictDataRequest) {
  return request.post<ApiRes<DictDataResponse>>('/api/dict/data', data).then(r => r.data)
}

export function getDictData(id: string) {
  return request.get<ApiRes<DictDataResponse>>(`/api/dict/data/${id}`).then(r => r.data)
}

export function updateDictData(id: string, data: UpdateDictDataRequest) {
  return request.put<ApiRes<DictDataResponse>>(`/api/dict/data/${id}`, { ...data, id }).then(r => r.data)
}

export function deleteDictData(id: string) {
  return request.delete<ApiRes<null>>(`/api/dict/data/${id}`).then(r => r.data)
}

export function listDictData(data: ListDictDataRequest) {
  return request.post<ApiRes<PageData<DictDataResponse>>>('/api/dict/data/list', data).then(r => r.data)
}

export function getDictDataByType(dictType: string) {
  return request.get<ApiRes<DictDataResponse[]>>(`/api/dict/data/type/${dictType}`).then(r => r.data)
}
