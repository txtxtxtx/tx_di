import axios from 'axios'

const http = axios.create({
  baseURL: '/api/gb28181',
  timeout: 10000,
})

export const api = {
  // 统计
  stats: ()                         => http.get('/stats'),

  // 设备
  devices: ()                       => http.get('/devices'),
  device: (id)                      => http.get(`/devices/${id}`),
  queryCatalog: (id)                => http.post(`/devices/${id}/catalog`),
  queryInfo: (id)                   => http.post(`/devices/${id}/info`),
  queryStatus: (id)                 => http.post(`/devices/${id}/status`),
  teleboot: (id)                    => http.post(`/devices/${id}/teleboot`),
  alarmReset: (id, alarmType = '0') => http.post(`/devices/${id}/alarm_reset`, { alarm_type: alarmType }),
  ptz: (id, body)                   => http.post(`/devices/${id}/ptz`, body),

  // 会话
  sessions: ()                      => http.get('/sessions'),
  invite: (device_id, channel_id)   => http.post('/sessions', { device_id, channel_id }),
  hangup: (call_id)                 => http.delete(`/sessions/${call_id}`),
}

export default api
