import axios from 'axios'

const http = axios.create({
  baseURL: '/api/gb28181',
  timeout: 15000,
})

export const api = {
  stats: ()                       => http.get('/stats'),
  devices: ()                     => http.get('/devices'),
  device: (id)                    => http.get(`/devices/${id}`),
  createDevice: (body)            => http.post('/devices', body),
  generate: (body)                => http.post('/devices/generate', body),
  remove: (id)                    => http.delete(`/devices/${id}`),
}

export default api
