import axios from 'axios'

const http = axios.create({
  baseURL: '/api/v1',
  timeout: 15000,
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
      if (window.location.pathname !== '/admin/login') {
        window.location.href = '/admin/login'
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
  cruiseList:     (id, channelId)       => http.post(`/gb28181/devices/${id}/cruise/list`,    { channel_id: channelId }),
  cruiseTrack:    (id, channelId, cruiseId) => http.post(`/gb28181/devices/${id}/cruise_track`, { channel_id: channelId, cruise_id: cruiseId }),
  zoomIn:          (id, channelId, rect)  => http.post(`/gb28181/devices/${id}/zoom/in`,       { channel_id: channelId, ...rect }),
  zoomOut:         (id, channelId, rect)  => http.post(`/gb28181/devices/${id}/zoom/out`,      { channel_id: channelId, ...rect }),
  makeKeyFrame:   (id, channelId)       => http.post(`/gb28181/devices/${id}/make_key_frame`, { channel_id: channelId }),
  ptzPrecise:     (id, channelId, pan, tilt, zoom) =>
    http.post(`/gb28181/devices/${id}/ptz_precise`, { channel_id: channelId, pan, tilt, zoom }),
  ptzPreciseStatus: (id)               => http.post(`/gb28181/devices/${id}/ptz_precise_status`),
  targetTrack:    (id, channelId, start) =>
    http.post(`/gb28181/devices/${id}/target_track`, { channel_id: channelId, start }),
  storageFormat:  (id, channelId)      => http.post(`/gb28181/devices/${id}/storage/format`,  { channel_id: channelId }),
  storageStatus:  (id, channelId)      => http.post(`/gb28181/devices/${id}/storage/status`,  { channel_id: channelId }),
  guardControl:   (id, channelId, mode, presetIndex = 0) =>
    http.post(`/gb28181/devices/${id}/guard/control`, { channel_id: channelId, mode, preset_index: presetIndex }),
  guardInfo:      (id)                  => http.post(`/gb28181/devices/${id}/guard/info`),
  guardBasic:     (id, channelId, guard) =>
    http.post(`/gb28181/devices/${id}/guard/basic`, { channel_id: channelId, guard }),

  // ═════════════
  //  报警
  // ═════════════
  alarmSubscribe:   (id, alarmType = 0, expire = 3600) =>
    http.post(`/gb28181/devices/${id}/alarm/subscribe`, { alarm_type: alarmType, expire }),
  alarmResetDev:    (id, alarmType = 'All') =>
    http.post(`/gb28181/devices/${id}/alarm/reset`, { alarm_type: alarmType }),
  alarms:           (params = {})   => http.get('/gb28181/alarms', { params }),
  alarm:            (id)            => http.get(`/gb28181/alarms/${id}`),
  handleAlarm:      (id, body)     => http.put(`/gb28181/alarms/${id}`, body),

  // ═════════════
  //  移动位置
  // ═════════════
  mobilePositionQuery:       (id, interval)  =>
    http.post(`/gb28181/devices/${id}/mobile_position/query`, { interval }),
  mobilePositionUnsubscribe: (id)            =>
    http.post(`/gb28181/devices/${id}/mobile_position/unsubscribe`),

  // ═════════════
  //  录像 / 回放 / 下载
  // ═════════════
  queryRecords:    (id, body)   => http.post(`/gb28181/devices/${id}/records/query`, body),
  startPlayback:   (id, body)   => http.post(`/gb28181/devices/${id}/playback/start`, body),
  playbackControl: (id, body)   => http.post(`/gb28181/devices/${id}/playback/control`, body),
  recordControl:   (id, body)   => http.post(`/gb28181/devices/${id}/record/control`, body),
  startDownload:   (id, body)   => http.post(`/gb28181/devices/${id}/download/start`, body),
  playbackCtrl:    (id, body)   => http.post(`/gb28181/devices/${id}/playback_ctrl`, body),

  // ═════════════
  //  语音广播 / 对讲
  // ═════════════
  broadcastInvite: (id)         => http.post(`/gb28181/devices/${id}/broadcast/invite`),
  broadcastAccept: (id, port)   => http.post(`/gb28181/devices/${id}/broadcast/accept`, { audio_port: port }),
  broadcastStop:   (id)         => http.post(`/gb28181/devices/${id}/broadcast/stop`),
  startTalkback:   (id, body)   => http.post(`/gb28181/devices/${id}/talkback/start`, body),

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
