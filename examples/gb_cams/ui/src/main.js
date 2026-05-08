import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import Dashboard from './views/Dashboard.vue'
import Devices from './views/Devices.vue'
import DeviceDetail from './views/DeviceDetail.vue'

const router = createRouter({
  history: createWebHistory('/cam/'),
  routes: [
    { path: '/', component: Dashboard },
    { path: '/devices', component: Devices },
    { path: '/devices/:id', component: DeviceDetail },
  ],
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
