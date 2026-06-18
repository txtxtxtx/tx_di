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
        <!-- 仪表盘（始终显示） -->
        <el-menu-item index="/dashboard">
          <el-icon><Odometer /></el-icon>
          <template #title>仪表盘</template>
        </el-menu-item>

        <!-- 后端菜单树 -->
        <template v-for="item in menuTree" :key="item.fullPath">
          <!-- 叶子节点（无子菜单或只有一个子菜单） -->
          <el-menu-item
            v-if="!item.children || item.children.length <= 1"
            :index="getLeafPath(item)"
          >
            <el-icon v-if="getLeafIcon(item)">
              <component :is="getLeafIcon(item)" />
            </el-icon>
            <template #title>{{ getLeafTitle(item) }}</template>
          </el-menu-item>

          <!-- 有多个子菜单 -->
          <el-sub-menu v-else :index="item.fullPath">
            <template #title>
              <el-icon v-if="item.icon">
                <component :is="item.icon" />
              </el-icon>
              <span>{{ item.name }}</span>
            </template>
            <el-menu-item
              v-for="child in item.children"
              :key="child.fullPath"
              :index="child.fullPath"
            >
              <el-icon v-if="child.icon">
                <component :is="child.icon" />
              </el-icon>
              <template #title>{{ child.name }}</template>
            </el-menu-item>
          </el-sub-menu>
        </template>
      </el-menu>
    </el-scrollbar>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useMenuStore } from '@/stores/menu'
import type { MenuTreeNode } from '@/types'

interface MenuNodeWithPath extends MenuTreeNode {
  fullPath: string
  children: MenuNodeWithPath[]
}

const route = useRoute()
const appStore = useAppStore()
const menuStore = useMenuStore()

const menuTree = computed<MenuNodeWithPath[]>(() => {
  return buildMenuTree(menuStore.menus, '')
})

function buildMenuTree(nodes: MenuTreeNode[], parentPath: string): MenuNodeWithPath[] {
  return nodes
    .filter(n => n.types !== 2 && n.status === 0 && n.visible === 0)
    .sort((a, b) => a.sort - b.sort)
    .map(node => {
      const fullPath = parentPath
        ? `${parentPath}/${node.path || ''}`
        : `/${node.path || ''}`
      return {
        ...node,
        fullPath,
        children: node.children ? buildMenuTree(node.children, fullPath) : [],
      }
    })
}

function getLeafPath(item: MenuNodeWithPath): string {
  if (item.children && item.children.length === 1) {
    return item.children[0].fullPath
  }
  return item.fullPath
}

function getLeafIcon(item: MenuNodeWithPath): string | null {
  if (item.children && item.children.length === 1) {
    return item.children[0].icon || null
  }
  return item.icon || null
}

function getLeafTitle(item: MenuNodeWithPath): string {
  if (item.children && item.children.length === 1) {
    return item.children[0].name
  }
  return item.name
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
