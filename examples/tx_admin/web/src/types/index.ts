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
// proto, camelCase
export interface LoginRequest {
  username: string
  password: string
  loginIp: string
}

// proto, camelCase, u64 fields as string
export interface LoginResponse {
  userId: string
  username: string
  nickname: string
  tenantId: string
  roleIds: string[]
  permissions: string[]
  deptIds: string[]
  token: string
  roleCodes: string[]
}

// proto, camelCase, u64 fields as string
export interface UserInfoResponse {
  userId: string
  username: string
  nickname: string
  email: string | null
  mobile: string | null
  avatar: string | null
  roles: string[]
  permissions: string[]
  tenantId: string
}

// ==================== 用户 (proto, camelCase) ====================
// ID 字段为 string（u64 溢出），时间戳为 string（proto i64 + FlexibleDisplayFromStr 序列化为 JSON 字符串）
export interface UserResponse {
  id: string
  username: string
  nickname: string
  email: string | null
  mobile: string | null
  sex: number
  status: number
  remark: string | null
  roleIds: string[]
  deptIds: string[]
  avatar: string | null
  loginIp: string | null
  /** Unix 毫秒时间戳（proto i64 + FlexibleDisplayFromStr 序列化为 string），0 = 未登录 */
  loginDate: string
  tenantId: string
  /** Unix 毫秒时间戳（proto i64 + FlexibleDisplayFromStr 序列化为 string） */
  createTime: string
  /** Unix 毫秒时间戳（proto i64 + FlexibleDisplayFromStr 序列化为 string） */
  updateTime: string
}

// proto, camelCase, u64 fields as string
export interface CreateUserRequest {
  username: string
  password: string
  nickname: string
  email?: string
  mobile?: string
  sex?: number
  remark?: string
  roleIds: string[]
  deptIds: string[]
}

export interface UpdateUserRequest {
  userId?: string
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
  deptId?: string
  pageInfo?: { page: number; size: number }
}

// proto, camelCase, u64 fields as string
export interface ChangePasswordRequest {
  userId: string
  newPassword: string
}

// proto, camelCase, u64 fields as string
export interface AssignRolesRequest {
  userId: string
  roleIds: string[]
}

// proto, camelCase, u64 fields as string
export interface AssignDeptsRequest {
  userId: string
  deptIds: string[]
}

// proto, camelCase, u64 fields as string
export interface UserIdRequest {
  userId: string
}

// ==================== 角色 (proto, camelCase) ====================
// u64 fields: id, menuIds
export interface RoleResponse {
  id: string
  name: string
  code: string
  sort: number
  dataScope: number
  status: number
  remark: string | null
  menuIds: string[]
}

// proto, camelCase, u64 fields as string
export interface CreateRoleRequest {
  name: string
  code: string
  sort: number
  remark?: string
  menuIds: string[]
}

// proto, camelCase
export interface UpdateRoleRequest {
  roleId?: string
  name: string
  code: string
  sort: number
  dataScope: number
  remark?: string
}

export interface ListRolesRequest {
  name?: string
  code?: string
  status?: number
  page: number
  pageSize: number
}

// proto, camelCase, u64 fields as string
export interface AssignMenusRequest {
  roleId: string
  menuIds: string[]
}

// ==================== 菜单 (proto, camelCase) ====================
// u64 fields: id, parentId
export interface MenuTreeNode {
  id: string
  name: string
  permission: string
  types: number
  sort: number
  parentId: string
  path: string | null
  icon: string | null
  component: string | null
  componentName: string | null
  status: number
  visible: number
  keepAlive: number
  children: MenuTreeNode[]
}

// proto, u64 fields as string
export interface CreateMenuRequest {
  name: string
  permission: string
  types: number
  sort: number
  parentId: string
  path?: string
  icon?: string
  component?: string
  componentName?: string
}

// proto, u64 fields as string
export interface UpdateMenuRequest {
  menuId?: string
  name: string
  permission: string
  types: number
  sort: number
  parentId: string
  path?: string
  icon?: string
  component?: string
  componentName?: string
  visible: number
  keepAlive: number
}

export interface ListMenusRequest {
  name?: string
  status?: number
}

// ==================== 部门 (proto, camelCase) ====================
// u64 fields: id, parentId, leaderUserId
export interface DeptTreeNode {
  id: string
  name: string
  parentId: string
  sort: number
  leaderUserId: string | null
  status: number
  children: DeptTreeNode[]
}

// u64 fields as string
export interface CreateDeptRequest {
  name: string
  parentId: string
  sort: number
  leaderUserId?: string
  phone?: string
  email?: string
}

// u64 fields as string
export interface UpdateDeptRequest {
  deptId?: string
  name: string
  parentId: string
  sort: number
  leaderUserId?: string
  phone?: string
  email?: string
}

export interface ListDeptsRequest {
  name?: string
  status?: number
}

// ==================== 配置 (proto, camelCase) ====================
// u64 fields: id
export interface ConfigResponse {
  id: string
  category: string
  configType: number
  name: string
  configKey: string
  value: string
  visible: number
  remark: string | null
}

// proto, camelCase
export interface CreateConfigRequest {
  category: string
  configType: number
  name: string
  configKey: string
  value: string
  remark?: string
}

// proto, camelCase
export interface UpdateConfigRequest {
  configId?: string
  category: string
  configType: number
  name: string
  configKey: string
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
// u64 fields: id
export interface DictTypeResponse {
  id: string
  name: string
  dictType: string
  status: number
  remark: string | null
}

// proto, camelCase
export interface CreateDictTypeRequest {
  name: string
  dictType: string
  remark?: string
}

// proto, camelCase
export interface UpdateDictTypeRequest {
  id?: string
  name: string
  dictType: string
  remark?: string
}

export interface ListDictTypesRequest {
  name?: string
  dictType?: string
  status?: number
  page: number
  pageSize: number
}

// u64 fields: id
export interface DictDataResponse {
  id: string
  sort: number
  label: string
  value: string
  dictType: string
  status: number
  colorType: string | null
  cssClass: string | null
  remark: string | null
}

// proto, camelCase
export interface CreateDictDataRequest {
  sort: number
  label: string
  value: string
  dictType: string
  colorType?: string
  cssClass?: string
  remark?: string
}

// proto, camelCase
export interface UpdateDictDataRequest {
  id?: string
  sort: number
  label: string
  value: string
  dictType: string
  colorType?: string
  cssClass?: string
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
// u64 fields: id, userId, bizId
export interface OperateLogResponse {
  id: string
  traceId: string
  userId: string
  userType: number
  logType: string
  subType: string
  bizId: string
  action: string
  success: number
  extra: string
  requestMethod: string | null
  requestUrl: string | null
  userIp: string | null
}

// u64 fields as string
export interface ListOperateLogsRequest {
  userId?: string
  logType?: string
  subType?: string
  success?: number
  beginTime?: string
  endTime?: string
  page: number
  pageSize: number
}

// u64 fields: id, userId
export interface LoginLogResponse {
  id: string
  userId: string
  userType: number
  username: string
  loginIp: string
  loginType: string
  result: number
  msg: string | null
}

// u64 fields as string
export interface ListLoginLogsRequest {
  userId?: string
  username?: string
  loginIp?: string
  loginType?: string
  result?: number
  beginTime?: string
  endTime?: string
  page: number
  pageSize: number
}

// u64 fields as string
export interface DeleteLogsRequest {
  ids: string[]
}

// ==================== 文件 (proto, camelCase) ====================
// u64 fields: id; int32 fields: configId, size
export interface FileResponse {
  id: string
  configId: number | null
  name: string
  path: string
  url: string
  fileType: string | null
  size: number
}

export interface ListFilesRequest {
  name?: string
  fileType?: string
  configId?: number
  page: number
  pageSize: number
}

// u64 fields: id
export interface FileConfigResponse {
  id: number
  name: string
  storage: number      // 0=本地, 1=S3, 2=数据库
  remark: string | null
  master: number        // 0=普通, 1=主配置
  config: string        // JSON 配置
}

export interface CreateFileConfigRequest {
  name: string
  storage: number
  remark?: string
  config: string
}

export interface UpdateFileConfigRequest {
  id: number
  name: string
  storage: number
  remark?: string
  config: string
}

export interface PreviewUrlRes {
  url: string
  type: string              // "permanent" | "temporary"
  expiresAt: string | null  // ISO datetime, null for permanent
}

// ==================== 监控 (proto, camelCase) ====================
export interface DiskInfo {
  name: string
  totalSpace: number
  availableSpace: number
  usage: number
}

export interface NetworkInfo {
  name: string
  macAddress: string
  ipAddresses: string[]
  receivedBytes: number
  transmittedBytes: number
  receivedPackets: number
  transmittedPackets: number
}

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
  disks: DiskInfo[]
  networks: NetworkInfo[]
}

// u64 fields: userId
export interface OnlineUser {
  userId: string
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

// ==================== 定时任务 (proto, camelCase) ====================
// u64 fields: id, jobId
export interface JobResponse {
  id: string
  name: string
  status: number            // 0=暂停, 1=运行
  handlerName: string
  handlerParam: string | null
  cronExpression: string
  retryCount: number
  retryInterval: number     // 秒
  monitorTimeout: number    // 秒
}

export interface JobLogResponse {
  id: string
  jobId: string
  handlerName: string
  handlerParam: string | null
  executeIndex: number
  beginTime: string          // Unix 毫秒时间戳（后端 FlexibleDisplayFromStr 序列化为字符串）
  endTime: string | null     // Unix 毫秒时间戳
  duration: number | null    // 毫秒
  status: number            // 0=失败(Failed), 1=成功(Success), 2=超时(Timeout), 3=重试中(Retrying)
  result: string | null
}

export interface CreateJobRequest {
  name: string
  handlerName: string
  handlerParam?: string
  cronExpression: string
  retryCount?: number
  retryInterval?: number
  monitorTimeout?: number
}

export interface UpdateJobRequest {
  id: string
  name: string
  handlerName: string
  handlerParam?: string
  cronExpression: string
  retryCount?: number
  retryInterval?: number
  monitorTimeout?: number
}

export interface ListJobsRequest {
  name?: string
  status?: number
  page: number
  pageSize: number
}

export interface ListJobLogsRequest {
  jobId?: string
  status?: number
  page: number
  pageSize: number
}

export interface ChangeJobStatusRequest {
  id: string
  status: number          // 0=暂停, 1=运行
}

export interface CleanJobLogsRequest {
  jobId?: string | null   // 为空则清空所有
}
