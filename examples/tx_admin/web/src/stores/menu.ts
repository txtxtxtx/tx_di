import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getUserMenusApi } from '@/api/auth'
import type { MenuTreeNode } from '@/types'

export const useMenuStore = defineStore('menu', () => {
  const menus = ref<MenuTreeNode[]>([])
  const loaded = ref(false)

  async function fetchMenus(): Promise<MenuTreeNode[]> {
    const res = await getUserMenusApi()
    menus.value = res.data ?? []
    loaded.value = true
    return menus.value
  }

  function clearMenus() {
    menus.value = []
    loaded.value = false
  }

  return { menus, loaded, fetchMenus, clearMenus }
})
