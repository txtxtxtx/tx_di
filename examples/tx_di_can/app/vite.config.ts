import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 期望固定的 dev server 端口；clearScreen=false 避免刷掉 Rust 日志
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: false,
  },
})
