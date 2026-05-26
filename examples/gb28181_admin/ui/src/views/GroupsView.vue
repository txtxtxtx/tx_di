<template>
  <div>
    <div class="page-header flex items-center justify-between">
      <div>
        <h1>设备分组管理</h1>
        <p>树形分组，批量管理设备</p>
      </div>
      <div class="flex gap-8">
        <button class="btn btn-primary" @click="openGroupModal()">+ 新建分组</button>
        <button class="btn" @click="refresh">刷新</button>
      </div>
    </div>

    <!-- 分组树 + 成员面板 -->
    <div class="card mt-16" style="display:flex;min-height:420px">
      <!-- 左侧：分组树 -->
      <div style="width:320px;border-right:1px solid var(--border);padding:16px;overflow-y:auto">
        <div v-if="groups.length === 0" class="empty-state" style="padding:24px 0">
          <p>暂无分组</p>
        </div>
        <div v-else>
          <!-- 根目录 -->
          <div
            :class="['tree-item', selectedGroupId === null ? 'tree-item--active' : '']"
            @click="selectGroup(null)"
            style="padding-left:8px"
          >
            📂 全部设备
            <span class="badge badge-session" style="margin-left:auto">{{ totalMemberCount }}</span>
          </div>
          <div v-for="g in groupTree" :key="g.id">
            <GroupTreeNode
              :node="g"
              :selected="selectedGroupId === g.id"
              @select="selectGroup"
              @edit="openGroupModal"
              @delete="handleDeleteGroup"
            />
          </div>
        </div>
      </div>

      <!-- 右侧：成员列表 -->
      <div style="flex:1;padding:16px;overflow-y:auto">
        <div v-if="selectedGroupId === null" class="empty-state">
          <p>请选择左侧分组查看成员</p>
        </div>
        <div v-else>
          <div class="flex items-center justify-between" style="margin-bottom:14px">
            <h2 style="font-size:15px;font-weight:600">{{ currentGroupName }}（{{ members.length }} 个成员）</h2>
            <div class="flex gap-8">
              <button class="btn btn-sm btn-primary" @click="openAddMemberModal">+ 添加设备</button>
              <button class="btn btn-sm" @click="loadMembers(selectedGroupId)">刷新</button>
            </div>
          </div>

          <div v-if="members.length === 0" class="empty-state">
            <p>分组内暂无设备</p>
          </div>
          <table v-else>
            <thead>
              <tr>
                <th>设备 ID</th>
                <th>加入时间</th>
                <th style="text-align:right">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="m in members" :key="m.device_id">
                <td>
                  <RouterLink :to="`/devices/${m.device_id}`" class="device-link">{{ m.device_id }}</RouterLink>
                </td>
                <td class="text-muted">{{ m.joined_at }}</td>
                <td style="text-align:right">
                  <button class="btn btn-sm btn-danger" @click="handleRemoveMember(m.device_id)">移除</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 新建/编辑分组弹窗 -->
    <Teleport to="body">
      <div v-if="showGroupModal" class="modal-mask" @click.self="showGroupModal = false">
        <div class="modal-box">
          <h3>{{ editingGroup ? '编辑分组' : '新建分组' }}</h3>
          <div class="form-group">
            <label>分组名称 *</label>
            <input v-model="groupForm.name" class="input" placeholder="请输入分组名称" />
          </div>
          <div class="form-group">
            <label>父分组</label>
            <select v-model.number="groupForm.parent_id" class="input">
              <option :value="0">— 根分组 —</option>
              <option v-for="g in flatGroups" :key="g.id" :value="g.id">{{ '　'.repeat(g._depth || 0) }}{{ g.name }}</option>
            </select>
          </div>
          <div class="form-group">
            <label>描述</label>
            <textarea v-model="groupForm.description" class="input" rows="3" placeholder="可选"></textarea>
          </div>
          <div class="modal-actions">
            <button class="btn" @click="showGroupModal = false">取消</button>
            <button class="btn btn-primary" @click="handleSaveGroup" :disabled="saving">
              <span v-if="saving" class="spinner" style="width:12px;height:12px;border-width:2px;margin-right:4px"></span>
              保存
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- 添加成员弹窗 -->
    <Teleport to="body">
      <div v-if="showMemberModal" class="modal-mask" @click.self="showMemberModal = false">
        <div class="modal-box">
          <h3>添加设备到「{{ currentGroupName }}」</h3>
          <div style="margin-bottom:12px">
            <input v-model="memberSearch" class="input" placeholder="搜索设备 ID / 厂商..." style="max-width:320px" />
          </div>
          <div v-if="availableDevices.length === 0" class="empty-state"><p>暂无可用设备</p></div>
          <div v-else style="max-height:360px;overflow-y:auto">
            <table>
              <thead>
                <tr>
                  <th style="width:40px"><input type="checkbox" @change="toggleSelectAll" :checked="allSelected" /></th>
                  <th>设备 ID</th>
                  <th>厂商</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="d in filteredAvailable" :key="d.device_id">
                  <td><input type="checkbox" :value="d.device_id" v-model="selectedDeviceIds" /></td>
                  <td>{{ d.device_id }}</td>
                  <td>{{ d.manufacturer || '—' }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="modal-actions">
            <button class="btn" @click="showMemberModal = false">取消</button>
            <button class="btn btn-primary" @click="handleAddMembers" :disabled="selectedDeviceIds.length === 0 || saving">
              添加（{{ selectedDeviceIds.length }}）
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { useGb28181Store } from '../stores/gb28181.js'
import api from '../api/index.js'

// —— 递归树节点组件（内联）──
import { h, computed as vueComputed } from 'vue'

const GroupTreeNode = {
  name: 'GroupTreeNode',
  props: { node: Object, selected: Boolean },
  emits: ['select', 'edit', 'delete'],
  template: `
    <div>
      <div
        :class="['tree-item', selected ? 'tree-item--active' : '']"
        :style="{ paddingLeft: (node._depth || 0) * 18 + 8 + 'px' }"
        @click="$emit('select', node.id)"
      >
        <span style="margin-right:4px">📁</span>
        <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ node.name }}</span>
        <span class="badge badge-session" style="font-size:11px;margin:0 4px">{{ node.member_count || 0 }}</span>
        <span class="tree-actions">
          <button class="btn btn-sm" @click.stop="$emit('edit', node)" title="编辑">✎</button>
          <button class="btn btn-sm btn-danger" @click.stop="$emit('delete', node.id)" title="删除">✕</button>
        </span>
      </div>
      <div v-if="node.children && node.children.length">
        <GroupTreeNode
          v-for="c in node.children" :key="c.id"
          :node="c" :selected="selected"
          @select="$emit('select', $event)"
          @edit="$emit('edit', $event)"
          @delete="$emit('delete', $event)"
        />
      </div>
    </div>
  `,
}

const store = useGb28181Store()

// —— 主状态 ——
const groups            = ref([])
const members           = ref([])
const selectedGroupId   = ref(null)
const loading           = ref(false)
const saving            = ref(false)

// —— 分组弹窗 ——
const showGroupModal    = ref(false)
const editingGroup      = ref(null)
const groupForm         = reactive({ name: '', parent_id: 0, description: '' })

// —— 成员弹窗 ——
const showMemberModal   = ref(false)
const memberSearch      = ref('')
const selectedDeviceIds = ref([])

// —— 计算属性 ——
const groupTree = vueComputed(() => buildTree(groups.value))
const flatGroups = vueComputed(() => flattenTree(groupTree.value))
const currentGroupName = vueComputed(() => groups.value.find(g => g.id === selectedGroupId.value)?.name || '')
const totalMemberCount = vueComputed(() => groups.value.reduce((s, g) => s + (g.member_count || 0), 0))
const availableDevices = vueComputed(() => store.devices)
const filteredAvailable = vueComputed(() => {
  if (!memberSearch.value.trim()) return availableDevices.value
  const q = memberSearch.value.toLowerCase()
  return availableDevices.value.filter(d =>
    d.device_id.toLowerCase().includes(q) || (d.manufacturer || '').toLowerCase().includes(q)
  )
})
const allSelected = vueComputed(() => filteredAvailable.value.length > 0 && selectedDeviceIds.value.length === filteredAvailable.value.length)

// —— 加载数据 ——
async function refresh() {
  loading.value = true
  try {
    await Promise.all([loadGroups(), store.fetchDevices()])
  } finally { loading.value = false }
}
async function loadGroups() {
  const res = await api.groupList()
  if (res.data.code === 200) groups.value = res.data.data
}
async function loadMembers(groupId) {
  const res = await api.groupMembers(groupId)
  if (res.data.code === 200) members.value = res.data.data
}

function selectGroup(id) {
  selectedGroupId.value = id
  if (id !== null) loadMembers(id)
  else members.value = []
}

// —— 分组 CRUD ——
function openGroupModal(group = null) {
  editingGroup.value = group
  if (group) {
    groupForm.name        = group.name
    groupForm.parent_id   = group.parent_id
    groupForm.description = group.description || ''
  } else {
    groupForm.name        = ''
    groupForm.parent_id   = 0
    groupForm.description = ''
  }
  showGroupModal.value = true
}
async function handleSaveGroup() {
  if (!groupForm.name.trim()) return alert('请填写分组名称')
  saving.value = true
  try {
    let res
    if (editingGroup.value) {
      res = await api.groupUpdate(editingGroup.value.id, { name: groupForm.name, parent_id: groupForm.parent_id, description: groupForm.description })
    } else {
      res = await api.groupCreate({ name: groupForm.name, parent_id: groupForm.parent_id, description: groupForm.description })
    }
    if (res.data.code === 200 || res.data.code === 0) {
      showGroupModal.value = false
      await loadGroups()
    } else {
      alert(res.data.message || '操作失败')
    }
  } catch (e) {
    alert(e.response?.data?.message || e.message)
  } finally { saving.value = false }
}
async function handleDeleteGroup(id) {
  if (!confirm('确定删除该分组？子分组将被一并删除。')) return
  try {
    const res = await api.groupDelete(id)
    if (res.data.code === 200 || res.data.code === 0) {
      if (selectedGroupId.value === id) selectedGroupId.value = null
      await loadGroups()
    }
  } catch (e) { alert(e.response?.data?.message || e.message) }
}

// —— 成员管理 ——
function openAddMemberModal() {
  selectedDeviceIds.value = []
  memberSearch.value      = ''
  showMemberModal.value = true
}
function toggleSelectAll(e) {
  if (e.target.checked) {
    selectedDeviceIds.value = filteredAvailable.value.map(d => d.device_id)
  } else {
    selectedDeviceIds.value = []
  }
}
async function handleAddMembers() {
  saving.value = true
  try {
    const res = await api.groupAddMembers(selectedGroupId.value, selectedDeviceIds.value)
    if (res.data.code === 200 || res.data.code === 0) {
      showMemberModal.value = false
      await Promise.all([loadGroups(), loadMembers(selectedGroupId.value)])
    } else { alert(res.data.message || '添加失败') }
  } catch (e) { alert(e.response?.data?.message || e.message) }
  finally { saving.value = false }
}
async function handleRemoveMember(deviceId) {
  if (!confirm(`确定将设备 ${deviceId} 从分组中移除？`)) return
  try {
    const res = await api.groupRemoveMember(selectedGroupId.value, deviceId)
    if (res.data.code === 200 || res.data.code === 0) {
      await Promise.all([loadGroups(), loadMembers(selectedGroupId.value)])
    }
  } catch (e) { alert(e.response?.data?.message || e.message) }
}

// —— 树形构建 ——
function buildTree(list) {
  const map = new Map()
  list.forEach(g => map.set(g.id, { ...g, children: [] }))
  const roots = []
  map.forEach(node => {
    if (node.parent_id && node.parent_id !== 0 && map.has(node.parent_id)) {
      map.get(node.parent_id).children.push(node)
    } else {
      roots.push(node)
    }
  })
  // 计算深度
  function setDepth(nodes, depth) {
    nodes.forEach(n => { n._depth = depth; if (n.children) setDepth(n.children, depth + 1) })
  }
  setDepth(roots, 0)
  return roots
}
function flattenTree(nodes) {
  const r = []
  nodes.forEach(n => { r.push(n); if (n.children) r.push(...flattenTree(n.children)) })
  return r
}

onMounted(refresh)
</script>

<style scoped>
.tree-item {
  display: flex; align-items: center; gap: 4px;
  padding: 5px 8px; border-radius: 4px; cursor: pointer;
  font-size: 13px; transition: background .1s;
}
.tree-item:hover { background: var(--bg); }
.tree-item--active { background: rgba(78,142,247,.12); color: var(--primary); font-weight:500; }
.tree-actions { display: none; gap: 2px; margin-left: auto; flex-shrink:0; }
.tree-item:hover .tree-actions { display: flex; }
.device-link { color: var(--primary); font-weight: 500; }
.device-link:hover { text-decoration: underline; }
.text-muted { color: var(--text-muted); font-size: 12px; }

/* 弹窗 */
.modal-mask {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0,0,0,.45);
  display: flex; align-items: center; justify-content: center;
}
.modal-box {
  background: var(--card-bg); border-radius: 10px;
  padding: 24px 28px; width: 520px; max-height: 85vh; overflow-y: auto;
  box-shadow: 0 8px 32px rgba(0,0,0,.18);
}
.modal-box h3 { font-size: 16px; font-weight: 600; margin-bottom: 18px; }
.form-group { margin-bottom: 14px; }
.form-group label { display: block; font-size: 12px; color: var(--text-muted); margin-bottom: 4px; }
.input {
  width: 100%; padding: 7px 10px; font-size: 13px;
  border: 1px solid var(--border); border-radius: 6px;
  outline: none; font-family: inherit;
}
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
</style>
