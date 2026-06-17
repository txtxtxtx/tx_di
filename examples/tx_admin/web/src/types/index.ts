// ==================== 通用 ====================
export interface ApiRes<T = any> {
  code: number
  msg: string
  data: T
}

export interface PageData<T> {
  list: T[]
  page: number
  size: number
  total: number
}

// ==================== 认证 ====================
export interface LoginRequest {
  username: string
  password: string
  login_ip: string
}

// LoginResponse 来自 admin_app (snake_case)
export interface LoginResponse {
  user_id: number
  username: string
  nickname: string
  tenant_id: number
  role_ids: number[]
  permissions: string[]
  dept_ids: number[]
}

// UserInfoResponse 来自 admin_app (snake_case)
export interface UserInfoResponse {
  user_id: number
  username: string
  nickname: string
  email: string | null
  mobile: string | null
  avatar: string | null
  roles: string[]
  permissions: string[]
}

// ==================== 用户 (proto, camelCase) ====================
export interface UserResponse {
  id: number
  username: string
  nickname: string
  email: string | null
  mobile: string | null
  sex: number
  status: number
  remark: string | null
  roleIds: number[]
  deptIds: number[]
  avatar: string | null
  loginIp: string | null
  loginDate: number
  tenantId: number
  createTime: number
  updateTime: number
}

export interface CreateUserRequest {
  username: string
  password: string
  nickname: string
  email?: string
  mobile?: string
  sex?: number
  remark?: string
  role_ids: number[]
  dept_ids: number[]
}

export interface UpdateUserRequest {
  nickname?: string
  email?: string
  mobile?: string
  sex?: number
  remark?: string
  status?: number
}

export interface ListUsersRequest {
  username?: string
  nickname?: string
  mobile?: string
  status?: number
  deptId?: number
  pageInfo?: { page: number; size: number }
}

export interface ChangePasswordRequest {
  user_id: number
  new_password: string
}

export interface AssignRolesRequest {
  user_id: number
  role_ids: number[]
}

export interface AssignDeptsRequest {
  user_id: number
  dept_ids: number[]
}

export interface UserIdRequest {
  user_id: number
}

// ==================== 角色 (proto, camelCase) ====================
export interface RoleResponse {
  id: number
  name: string
  code: string
  sort: number
  dataScope: number
  status: number
  remark: string | null
  menuIds: number[]
}

export interface CreateRoleRequest {
  name: string
  code: string
  sort: number
  remark?: string
  menu_ids: number[]
}

export interface UpdateRoleRequest {
  name: string
  code: string
  sort: number
  data_scope: number
  remark?: string
}

export interface ListRolesRequest {
  name?: string
  code?: string
  status?: number
  page: number
  pageSize: number
}

export interface AssignMenusRequest {
  role_id: number
  menu_ids: number[]
}

// ==================== 菜单 (domain, snake_case) ====================
export interface MenuTreeNode {
  id: number
  name: string
  permission: string
  types: number
  sort: number
  parent_id: number
  path: string | null
  icon: string | null
  component: string | null
  component_name: string | null
  status: number
  visible: number
  keep_alive: number
  children: MenuTreeNode[]
}

export interface CreateMenuRequest {
  name: string
  permission: string
  types: number
  sort: number
  parent_id: number
  path?: string
  icon?: string
  component?: string
  component_name?: string
}

export interface UpdateMenuRequest {
  name: string
  permission: string
  types: number
  sort: number
  parent_id: number
  path?: string
  icon?: string
  component?: string
  component_name?: string
  visible: number
  keep_alive: number
}

export interface ListMenusRequest {
  name?: string
  status?: number
}

// ==================== 部门 (domain, snake_case) ====================
export interface DeptTreeNode {
  id: number
  name: string
  parent_id: number
  sort: number
  leader_user_id: number | null
  status: number
  children: DeptTreeNode[]
}

export interface CreateDeptRequest {
  name: string
  parent_id: number
  sort: number
  leader_user_id?: number
  phone?: string
  email?: string
}

export interface UpdateDeptRequest {
  name: string
  parent_id: number
  sort: number
  leader_user_id?: number
  phone?: string
  email?: string
}

export interface ListDeptsRequest {
  name?: string
  status?: number
}

// ==================== 权限 (proto, camelCase) ====================
export interface PermissionDetail {
  id: number
  name: string
  permissionCode: string
  type: number
  parentId: number
  sort: number
  description: string
  status: number
}

export interface CreatePermissionRequest {
  name: string
  permission_code: string
  type: number
  parent_id: number
  sort: number
  description: string
}

export interface UpdatePermissionRequest {
  name: string
  permission_code: string
  type: number
  parent_id: number
  sort: number
  description: string
}

export interface PermissionCheckRequest {
  user_id: number
  permission: string
}

export interface PermissionCheckResponse {
  hasPermission: boolean
}

export interface UserPermissionsResponse {
  userId: number
  permissions: string[]
}

// ==================== 配置 (proto, camelCase) ====================
export interface ConfigResponse {
  id: number
  category: string
  configType: number
  name: string
  configKey: string
  value: string
  visible: number
  remark: string | null
}

export interface CreateConfigRequest {
  category: string
  config_type: number
  name: string
  config_key: string
  value: string
  remark?: string
}

export interface UpdateConfigRequest {
  category: string
  config_type: number
  name: string
  config_key: string
  value: string
  visible: number
  remark?: string
}

export interface ListConfigsRequest {
  name?: string
  category?: string
  configKey?: string
  configType?: number
  page: number
  pageSize: number
}

// ==================== 字典 (proto, camelCase) ====================
export interface DictTypeResponse {
  id: number
  name: string
  dictType: string
  status: number
  remark: string | null
}

export interface CreateDictTypeRequest {
  name: string
  dict_type: string
  remark?: string
}

export interface UpdateDictTypeRequest {
  name: string
  dict_type: string
  remark?: string
}

export interface ListDictTypesRequest {
  name?: string
  dictType?: string
  status?: number
  page: number
  pageSize: number
}

export interface DictDataResponse {
  id: number
  sort: number
  label: string
  value: string
  dictType: string
  status: number
  colorType: string | null
  cssClass: string | null
  remark: string | null
}

export interface CreateDictDataRequest {
  sort: number
  label: string
  value: string
  dict_type: string
  color_type?: string
  css_class?: string
  remark?: string
}

export interface UpdateDictDataRequest {
  sort: number
  label: string
  value: string
  dict_type: string
  color_type?: string
  css_class?: string
  remark?: string
}

export interface ListDictDataRequest {
  dictType?: string
  label?: string
  status?: number
  page: number
  pageSize: number
}

// ==================== 日志 (proto, camelCase) ====================
export interface OperateLogResponse {
  id: number
  traceId: string
  userId: number
  userType: number
  logType: string
  subType: string
  bizId: number
  action: string
  success: number
  extra: string
  requestMethod: string | null
  requestUrl: string | null
  userIp: string | null
}

export interface ListOperateLogsRequest {
  userId?: number
  logType?: string
  subType?: string
  success?: number
  beginTime?: string
  endTime?: string
  page: number
  pageSize: number
}

export interface LoginLogResponse {
  id: number
  userId: number
  userType: number
  username: string
  loginIp: string
  loginType: string
  result: number
  msg: string | null
}

export interface ListLoginLogsRequest {
  userId?: number
  username?: string
  loginIp?: string
  loginType?: string
  result?: number
  beginTime?: string
  endTime?: string
  page: number
  pageSize: number
}

export interface DeleteLogsRequest {
  ids: number[]
}

// ==================== 文件 (proto, camelCase) ====================
export interface FileResponse {
  id: number
  configId: number | null
  name: string
  path: string
  url: string
  fileType: string | null
  size: number
}

export interface UploadFileRequest {
  name: string
  path: string
  url: string
  file_type?: string
  size: number
  config_id?: number
}

export interface ListFilesRequest {
  name?: string
  fileType?: string
  configId?: number
  page: number
  pageSize: number
}

// ==================== 监控 (proto, camelCase) ====================
export interface ServerInfo {
  osName: string
  osVersion: string
  hostname: string
  cpuCores: number
  cpuUsage: number
  totalMemory: number
  usedMemory: number
  memoryUsage: number
  totalDisk: number
  usedDisk: number
  diskUsage: number
}

export interface OnlineUser {
  userId: number
  username: string
  loginIp: string
  loginTime: string
}

export interface OnlineUserListResponse {
  users: OnlineUser[]
  total: number
}

// ==================== 工具 (proto, camelCase) ====================
export interface CacheStatsResponse {
  totalKeys: number
  usedMemory: number
  hitCount: number
  missCount: number
  hitRate: number
}
