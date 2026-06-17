# TX Admin Web

基于 Vue 3 + Element Plus 的后台管理系统前端。

## 技术栈

| 技术 | 版本 | 说明 |
|------|------|------|
| Vue 3 | 3.5+ | 组合式 API + `<script setup>` |
| Element Plus | 2.9+ | UI 组件库 |
| Vue Router 4 | 4.5+ | 路由管理 |
| Pinia | 2.3+ | 状态管理 |
| Axios | 1.7+ | HTTP 请求 |
| TypeScript | 5.7+ | 类型安全 |
| Vite | 6.0+ | 构建工具 |

## 快速开始

```bash
# 安装依赖
npm install

# 启动开发服务器 (http://localhost:3000)
npm run dev

# 生产构建
npm run build

# 预览构建产物
npm run preview
```

开发模式下，`/api` 请求自动代理到 `http://localhost:8000`（后端服务）。

## 项目结构

```
src/
├── main.ts                  # 应用入口
├── App.vue                  # 根组件
├── router/index.ts          # 路由配置 + 登录守卫
├── stores/
│   ├── user.ts              # 用户状态（登录/登出/权限）
│   └── app.ts               # 应用状态（侧边栏）
├── api/
│   ├── request.ts           # Axios 实例 + 拦截器
│   ├── auth.ts              # 认证接口
│   ├── user.ts              # 用户管理接口
│   ├── role.ts              # 角色管理接口
│   ├── menu.ts              # 菜单管理接口
│   ├── dept.ts              # 部门管理接口
│   ├── permission.ts        # 权限管理接口
│   ├── config.ts            # 系统配置接口
│   ├── dict.ts              # 字典管理接口
│   ├── log.ts               # 日志管理接口
│   ├── file.ts              # 文件管理接口
│   ├── monitor.ts           # 系统监控接口
│   └── tool.ts              # 系统工具接口
├── types/index.ts           # TypeScript 类型定义
├── utils/index.ts           # 工具函数
├── layout/
│   ├── index.vue            # 主布局
│   ├── Sidebar.vue          # 侧边栏菜单
│   ├── Header.vue           # 顶部栏
│   └── Breadcrumb.vue       # 面包屑
└── views/
    ├── login/               # 登录页
    ├── dashboard/           # 仪表盘
    ├── system/
    │   ├── user/            # 用户管理
    │   ├── role/            # 角色管理
    │   ├── menu/            # 菜单管理
    │   ├── dept/            # 部门管理
    │   └── permission/      # 权限管理
    ├── config/
    │   ├── config/          # 参数设置
    │   └── dict/            # 字典管理（类型 + 数据）
    ├── log/
    │   ├── operate.vue      # 操作日志
    │   └── login.vue        # 登录日志
    ├── file/                # 文件管理
    └── monitor/
        ├── server.vue       # 服务器信息
        └── online.vue       # 在线用户
```

## 功能模块

### 认证
- 用户名密码登录，基于 sa-token 的会话认证
- Token 通过 `Authorization` 请求头传递
- 路由守卫自动跳转登录页

### 系统管理
- **用户管理** — 增删改查、分配角色、分配部门、启用/停用/锁定
- **角色管理** — 增删改查、分配菜单权限、查看角色用户
- **菜单管理** — 树形结构增删改查，支持目录/菜单/按钮三种类型
- **部门管理** — 树形结构增删改查
- **权限管理** — 增删改查，支持菜单/按钮/API 三种类型

### 系统配置
- **参数设置** — 系统配置项增删改查
- **字典类型** — 字典分类管理
- **字典数据** — 字典条目管理，支持按类型筛选

### 日志管理
- **操作日志** — 查看、批量删除、清空
- **登录日志** — 查看、批量删除、清空

### 文件管理
- 文件上传、下载、删除、列表查询

### 系统监控
- **服务器信息** — CPU/内存/磁盘使用率
- **在线用户** — 当前在线用户列表
- **缓存统计** — 缓存命中率等指标（仪表盘展示）

## 对接后端

后端服务位于 `examples/tx_admin/admin_api`，默认端口 8000。API 响应格式：

```json
// 成功
{ "code": 0, "msg": "ok", "data": { ... } }

// 分页
{ "code": 0, "msg": "ok", "data": { "list": [], "page": 1, "size": 10, "total": 100 } }

// 错误
{ "code": 401, "msg": "未登录", "data": null }
```

字段命名约定：
- Protobuf 生成的响应使用 `camelCase`（如 `userId`、`configKey`）
- 领域层/应用层响应使用 `snake_case`（如 `user_id`、`parent_id`）
