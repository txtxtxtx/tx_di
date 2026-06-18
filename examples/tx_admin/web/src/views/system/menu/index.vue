<template>
  <div class="page">
    <el-card shadow="never" class="search-card">
      <el-form :model="query" inline>
        <el-form-item label="菜单名称">
          <el-input v-model="query.name" placeholder="请输入" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="query.status" placeholder="全部" clearable>
            <el-option v-for="o in statusOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="loadData">搜索</el-button>
          <el-button @click="resetQuery">重置</el-button>
          <el-button type="success" @click="openDialog()">新增菜单</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="never">
      <el-table :data="treeData" v-loading="loading" row-key="id" border default-expand-all :tree-props="{ children: 'children' }">
        <el-table-column prop="name" label="菜单名称" min-width="180" />
        <el-table-column prop="icon" label="图标" width="80">
          <template #default="{ row }">
            <el-icon v-if="row.icon"><component :is="row.icon" /></el-icon>
          </template>
        </el-table-column>
        <el-table-column prop="sort" label="排序" width="70" />
        <el-table-column prop="permission" label="权限标识" width="180" show-overflow-tooltip />
        <el-table-column prop="types" label="类型" width="80">
          <template #default="{ row }">
            <el-tag :type="row.types === 0 ? '' : row.types === 1 ? 'success' : 'warning'">{{ menuTypeLabel(row.types) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="path" label="路由路径" width="180" show-overflow-tooltip />
        <el-table-column prop="component" label="组件路径" width="200" show-overflow-tooltip />
        <el-table-column prop="visible" label="可见" width="70">
          <template #default="{ row }">{{ visibleLabel(row.visible) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="70">
          <template #default="{ row }">
            <el-tag :type="row.status === 0 ? 'success' : 'danger'">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="openDialog(row)">编辑</el-button>
            <el-button link type="primary" size="small" @click="openDialog(null, row.id)">新增子菜单</el-button>
            <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 新增/编辑 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑菜单' : '新增菜单'" width="600px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="上级菜单">
          <el-tree-select v-model="form.parent_id" :data="menuTreeForSelect" :props="{ label: 'name', value: 'id', children: 'children' }" check-strictly clearable placeholder="顶级菜单" />
        </el-form-item>
        <el-form-item label="菜单类型" prop="types">
          <el-radio-group v-model="form.types">
            <el-radio v-for="o in menuTypeOptions" :key="o.value" :value="o.value">{{ o.label }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="菜单名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item v-if="form.types !== 2" label="图标">
          <el-input v-model="form.icon" placeholder="图标名称" />
        </el-form-item>
        <el-form-item label="排序" prop="sort">
          <el-input-number v-model="form.sort" :min="0" />
        </el-form-item>
        <el-form-item v-if="form.types !== 2" label="路由路径">
          <el-input v-model="form.path" />
        </el-form-item>
        <el-form-item v-if="form.types === 1" label="组件路径">
          <el-input v-model="form.component" />
        </el-form-item>
        <el-form-item v-if="form.types === 1" label="组件名称">
          <el-input v-model="form.component_name" />
        </el-form-item>
        <el-form-item label="权限标识">
          <el-input v-model="form.permission" placeholder="如: user:create" />
        </el-form-item>
        <el-form-item v-if="isEdit" label="是否可见">
          <el-radio-group v-model="form.visible">
            <el-radio :value="0">显示</el-radio>
            <el-radio :value="1">隐藏</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="isEdit" label="是否缓存">
          <el-radio-group v-model="form.keep_alive">
            <el-radio :value="0">不缓存</el-radio>
            <el-radio :value="1">缓存</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listMenus, createMenu, updateMenu, deleteMenu } from '@/api/menu'
import { statusOptions, statusLabel, menuTypeOptions, menuTypeLabel, visibleLabel } from '@/utils'
import type { MenuTreeNode } from '@/types'

const loading = ref(false)
const submitLoading = ref(false)
const treeData = ref<MenuTreeNode[]>([])
const query = reactive({ name: '', status: undefined as number | undefined })

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  id: '',
  name: '',
  permission: '',
  types: 0,
  sort: 0,
  parent_id: '',
  path: '',
  icon: '',
  component: '',
  component_name: '',
  visible: 0,
  keep_alive: 0,
})
const formRules: FormRules = {
  name: [{ required: true, message: '请输入菜单名称', trigger: 'blur' }],
  permission: [{ required: true, message: '请输入权限标识', trigger: 'blur' }],
}

const menuTreeForSelect = computed(() => {
  const root: MenuTreeNode = { id: '0', name: '顶级菜单', parent_id: '0', permission: '', types: 0, sort: 0, path: null, icon: null, component: null, component_name: null, status: 0, visible: 0, keep_alive: 0, children: treeData.value }
  return [root]
})

async function loadData() {
  loading.value = true
  try {
    treeData.value = (await listMenus({ name: query.name || undefined, status: query.status })).data
  } catch {} finally { loading.value = false }
}

function resetQuery() { query.name = ''; query.status = undefined; loadData() }

function openDialog(row?: MenuTreeNode | null, parentId?: string) {
  isEdit.value = !!row
  if (row) {
    Object.assign(form, {
      id: row.id, name: row.name, permission: row.permission, types: row.types, sort: row.sort,
      parent_id: row.parent_id, path: row.path || '', icon: row.icon || '',
      component: row.component || '', component_name: row.component_name || '',
      visible: row.visible, keep_alive: row.keep_alive,
    })
  } else {
    Object.assign(form, {
      id: '', name: '', permission: '', types: 0, sort: 0, parent_id: parentId || '',
      path: '', icon: '', component: '', component_name: '', visible: 0, keep_alive: 0,
    })
  }
  dialogVisible.value = true
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  submitLoading.value = true
  try {
    if (isEdit.value) {
      await updateMenu(form.id, {
        name: form.name, permission: form.permission, types: form.types, sort: form.sort,
        parent_id: form.parent_id, path: form.path || undefined, icon: form.icon || undefined,
        component: form.component || undefined, component_name: form.component_name || undefined,
        visible: form.visible, keep_alive: form.keep_alive,
      })
      ElMessage.success('更新成功')
    } else {
      await createMenu({
        name: form.name, permission: form.permission, types: form.types, sort: form.sort,
        parent_id: form.parent_id, path: form.path || undefined, icon: form.icon || undefined,
        component: form.component || undefined, component_name: form.component_name || undefined,
      })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false; loadData()
  } catch {} finally { submitLoading.value = false }
}

async function handleDelete(row: MenuTreeNode) {
  await ElMessageBox.confirm(`确认删除菜单 "${row.name}" 吗？`, '提示', { type: 'warning' })
  await deleteMenu(row.id)
  ElMessage.success('删除成功'); loadData()
}

onMounted(loadData)
</script>

<style scoped>
.search-card { margin-bottom: 16px; }
</style>
