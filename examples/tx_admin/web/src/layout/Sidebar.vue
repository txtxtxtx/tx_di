<template>
  <div class="sidebar">
    <div class="sidebar-title">
      <span v-if="!appStore.sidebarCollapsed">TX Admin</span>
      <span v-else>TX</span>
    </div>
    <el-scrollbar>
      <el-menu
        :default-active="route.path"
        :collapse="appStore.sidebarCollapsed"
        :collapse-transition="false"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409EFF"
        router
      >
        <template v-for="item in menuList" :key="item.path">
          <!-- 单个菜单项（无子菜单或只有一个子菜单） -->
          <el-menu-item
            v-if="!item.children || item.children.length === 1"
            :index="resolveMenuPath(item)"
          >
            <el-icon v-if="item.children ? item.children[0].meta?.icon : item.meta?.icon">
              <component :is="item.children ? item.children[0].meta?.icon : item.meta?.icon" />
            </el-icon>
            <template #title>{{ item.children ? item.children[0].meta?.title : item.meta?.title }}</template>
          </el-menu-item>

          <!-- 有多个子菜单 -->
          <el-sub-menu v-else :index="item.path">
            <template #title>
              <el-icon v-if="item.meta?.icon">
                <component :is="item.meta.icon" />
              </el-icon>
              <span>{{ item.meta?.title }}</span>
            </template>
            <el-menu-item
              v-for="child in item.children"
              :key="child.path"
              :index="`${item.path}/${child.path}`"
            >
              <el-icon v-if="child.meta?.icon">
                <component :is="child.meta.icon" />
              </el-icon>
              <template #title>{{ child.meta?.title }}</template>
            </el-menu-item>
          </el-sub-menu>
        </template>
      </el-menu>
    </el-scrollbar>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()

const menuList = computed(() => {
  return router.options.routes.filter(r => r.path !== '/login' && r.path !== '/')
    .concat(router.options.routes.filter(r => r.path === '/'))
    .filter(r => r.children)
})

/** 拼接出绝对路径，避免 el-menu router 模式下相对路径拼错 */
function resolveMenuPath(item: any): string {
  if (!item.children || item.children.length === 0) return item.path
  const child = item.children[0]
  // child.path 为空字符串时直接用父路径（如 /file）
  if (!child.path) return item.path
  // 父路径为 '/' 时避免双斜杠
  return item.path === '/' ? `/${child.path}` : `${item.path}/${child.path}`
}
</script>

<style scoped>
.sidebar {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.sidebar-title {
  height: 50px;
  line-height: 50px;
  text-align: center;
  font-size: 18px;
  font-weight: bold;
  color: #fff;
  background: #2b2f3a;
  white-space: nowrap;
  overflow: hidden;
}
.el-menu {
  border-right: none;
}
</style>
