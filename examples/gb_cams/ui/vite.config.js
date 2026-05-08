import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  base: '/cam/',
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8889',
        changeOrigin: true,
      },
    },
  },
})
