import axios from 'axios'
import type { AxiosResponse, InternalAxiosRequestConfig } from 'axios'
import { ElMessage } from 'element-plus'
import router from '@/router'
import type { ApiRes } from '@/types'
import {useUserStore} from "@/stores/user";

// 防止并发请求重复弹出 401 提示
let isRedirectingToLogin = false

const request = axios.create({
  baseURL: '',
  timeout: 30000,
})

// 请求拦截器：注入 token
request.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const token = localStorage.getItem('token')
    if (token) {
      config.headers['Authorization'] = token
    }
    return config
  },
  (error) => Promise.reject(error)
)

// 响应拦截器：统一错误处理
request.interceptors.response.use(
  (response: AxiosResponse<ApiRes>) => {
    const res = response.data
    if (res.code !== 0 && res.code !== 200) {
      // 认证失败
      if (res.code === 401) {
        useUserStore().clearAuthData()
        if (!isRedirectingToLogin) {
          isRedirectingToLogin = true
          ElMessage.error(res.msg || '认证失败，请重新登录')
          router.push('/login').finally(() => { isRedirectingToLogin = false })
        }
      } else {
        ElMessage.error(res.msg || '请求失败')
      }
      return Promise.reject(new Error(res.msg || '请求失败'))
    }
    return response
  },
  (error) => {
    if (error.response?.status === 401) {
      useUserStore().clearAuthData()
      if (!isRedirectingToLogin) {
        isRedirectingToLogin = true
        ElMessage.error('认证失败，请重新登录')
        router.push('/login').finally(() => { isRedirectingToLogin = false })
      }
      return Promise.reject(error)
    }
    ElMessage.error(error.response?.data?.msg || error.message || '网络错误')
    return Promise.reject(error)
  }
)

export default request
