import axios from 'axios'

const http = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
})

// —— Token 拦截器 ——
// 自动在请求头注入 token
http.interceptors.request.use(config => {
  const token = localStorage.getItem('satoken')
  if (token) {
    config.headers['Authorization'] = `Bearer ${token}`
  }
  return config
})

// 自动处理 401 → 跳登录页
http.interceptors.response.use(
  res => res,
  err => {
    if (err.response?.status === 401) {
      localStorage.removeItem('satoken')
      if (window.location.pathname !== '/login') {
        window.location.href = '/login'
      }
    }
    return Promise.reject(err)
  }
)

export const api = {
  // ═════════════
  //  认证
  // ═════════════
  login:    (username, password) => http.post('/auth/login', { username, password }),
  logout:   ()                      => http.post('/auth/logout'),
  authInfo: ()                      => http.get('/auth/info'),

  // ═════════════
  //  统计
  // ═════════════
  stats:      ()                        => http.get('/gb28181/stats'),
  dashboard:  ()                        => http.get('/gb28181/dashboard'),

  // ═════════════
  //  设备
  // ═════════════
  devices:         ()                       => http.get('/gb28181/devices'),
  device:          (id)                     => http.get(`/gb28181/devices/${id}`),
  queryCatalog:    (id)                    => http.post(`/gb28181/devices/${id}/catalog`),
  queryInfo:       (id)                    => http.post(`/gb28181/devices/${id}/info`),
  queryStatus:     (id)                    => http.post(`/gb28181/devices/${id}/status`),
  timeSync:        (id)                    => http.post(`/gb28181/devices/${id}/time_sync`),
  syncTime:        (id)                    => http.post(`/gb28181/devices/${id}/sync_time`),
  queryConfig:     (id, type)             => http.post(`/gb28181/devices/${id}/config`, { config_type: type }),
  teleboot:        (id)                    => http.post(`/gb28181/devices/${id}/teleboot`),
  alarmReset:      (id, alarmType = '0') => http.post(`/gb28181/devices/${id}/alarm_reset`, { alarm_type: alarmType }),
  ptz:             (id, body)              => http.post(`/gb28181/devices/${id}/ptz`, body),
  gotoPreset:     (id, channelId, idx)  => http.post(`/gb28181/devices/${id}/preset/goto`,    { channel_id: channelId, preset_index: idx }),
  setPreset:      (id, channelId, idx)  => http.post(`/gb28181/devices/${id}/preset/set`,     { channel_id: channelId, preset_index: idx }),
  startCruise:    (id, channelId, no)   => http.post(`/gb28181/devices/${id}/cruise/start`,   { channel_id: channelId, cruise_no: no }),
  stopCruise:     (id, channelId, no)   => http.post(`/gb28181/devices/${id}/cruise/stop`,    { channel_id: channelId, cruise_no: no }),
  zoomIn:          (id, channelId, rect)  => http.post(`/gb28181/devices/${id}/zoom/in`,       { channel_id: channelId, ...rect }),
  zoomOut:         (id, channelId, rect)  => http.post(`/gb28181/devices/${id}/zoom/out`,      { channel_id: channelId, ...rect }),

  // ═════════════
  //  会话
  // ═════════════
  sessions: ()                       => http.get('/gb28181/sessions'),
  invite:   (device_id, channel_id) => http.post('/gb28181/sessions', { device_id, channel_id }),
  hangup:   (call_id)               => http.delete(`/gb28181/sessions/${call_id}`),

  // ═════════════
  //  设备分组管理
  // ═════════════
  groupList:       ()                => http.get('/gb28181/groups'),
  groupCreate:     (body)           => http.post('/gb28181/groups', body),
  groupUpdate:     (id, body)      => http.put(`/gb28181/groups/${id}`, body),
  groupDelete:     (id)             => http.delete(`/gb28181/groups/${id}`),
  groupMembers:    (id)             => http.get(`/gb28181/groups/${id}/members`),
  groupAddMembers: (id, dids)      => http.post(`/gb28181/groups/${id}/members`, { device_ids: dids }),
  groupRemoveMember: (id, did)      => http.delete(`/gb28181/groups/${id}/members/${did}`),

  // ═════════════
  //  注册审核
  // ═════════════
  auditList:    ()                       => http.get('/gb28181/register_audit'),
  auditGet:     (id)                    => http.get(`/gb28181/register_audit/${id}`),
  auditApprove: (id)                    => http.post(`/gb28181/register_audit/${id}/approve`),
  auditReject:  (id, reason = '')      => http.post(`/gb28181/register_audit/${id}/reject`, { reason }),
  auditDelete:  (id)                    => http.delete(`/gb28181/register_audit/${id}`),
  auditAutoApprove: ()                   => http.post('/gb28181/register_audit/auto_approve'),

  // ═════════════
  //  审计日志
  // ═════════════
  auditLogs:    (params = {})           => http.get('/gb28181/audit_logs', { params }),
  auditLog:     (id)                    => http.get(`/gb28181/audit_logs/${id}`),
}

export default api
