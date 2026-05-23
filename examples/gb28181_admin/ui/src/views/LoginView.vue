<template>
  <div class="login-page">
    <div class="login-card">
      <div class="login-logo">📡 GB28181 管理后台</div>

      <form @submit.prevent="handleLogin" class="login-form">
        <div class="form-group">
          <label>用户名</label>
          <input v-model="form.username" class="input" placeholder="请输入用户名" required />
        </div>
        <div class="form-group">
          <label>密码</label>
          <input v-model="form.password" type="password" class="input" placeholder="请输入密码" required />
        </div>

        <div v-if="error" class="alert alert-danger">{{ error }}</div>

        <button type="submit" class="btn btn-primary btn-block" :disabled="loading">
          <span v-if="loading" class="spinner" style="width:14px;height:14px;border-width:2px;margin-right:6px"></span>
          登 录
        </button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import api from '../api/index.js'

const router = useRouter()
const route  = useRoute()
const form   = reactive({ username: '', password: '' })
const loading = ref(false)
const error   = ref('')

async function handleLogin() {
  error.value   = ''
  loading.value = true
  try {
    const res = await api.login(form.username, form.password)
    if (res.data.code === 200) {
      // 后端返回 { code, data: { token, user } }
      const d = res.data.data
      localStorage.setItem('satoken', d.token || '')
      // 跳转到 redirect 或首页
      router.replace(route.query.redirect || '/dashboard')
    } else {
      error.value = res.data.message || '登录失败'
    }
  } catch (e) {
    error.value = e.response?.data?.message || e.message || '网络错误'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex; align-items: center; justify-content: center;
  background: var(--bg);
}
.login-card {
  width: 380px;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
  padding: 36px 32px;
}
.login-logo {
  font-size: 20px; font-weight: 700; color: var(--primary);
  text-align: center; margin-bottom: 28px;
}
.form-group { margin-bottom: 18px; }
.form-group label { display: block; font-size: 13px; color: var(--text-muted); margin-bottom: 6px; }
.input {
  width: 100%; padding: 9px 12px; font-size: 14px;
  border: 1px solid var(--border); border-radius: 6px;
  outline: none; transition: border-color .15s;
  font-family: inherit;
}
.input:focus { border-color: var(--primary); }
.btn-block { width: 100%; justify-content: center; padding: 10px; font-size: 15px; }
.alert { padding: 8px 12px; border-radius: 6px; font-size: 13px; margin-bottom: 14px; }
.alert-danger { background: #fee2e2; color: #dc2626; }
</style>
