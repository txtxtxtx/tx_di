//! 设备分组管理 API
//!
//! ## 功能清单
//!
//! ### 分组 CRUD
//! - `GET  /gb28181/groups`           — 分组树列表
//! - `POST /gb28181/groups`           — 创建分组
//! - `GET  /gb28181/groups/:id`      — 分组详情
//! - `PUT  /gb28181/groups/:id`      — 更新分组
//! - `DEL  /gb28181/groups/:id`      — 删除分组（递归删除子分组）
//!
//! ### 成员管理
//! - `GET  /gb28181/groups/:id/members`     — 查询分组内设备列表
//! - `POST /gb28181/groups/:id/members`     — 添加设备到分组
//! - `DEL  /gb28181/groups/:id/members/:did` — 从分组移除设备

use axum::{
    extract::{Path, Query, State, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::R;
use tx_di_sa_token::LoginIdExtractor;
use toasty::Db;

use crate::dto::PageData;
use crate::models::{GbDeviceGroup, GbDeviceGroupMember, GbDeviceRecord};

// ══════════════════════════════════
//  DTO
// ══════════════════════════════════

/// 分组 DTO（含子分组 + 成员数量）
#[derive(Serialize, Clone)]
pub struct GroupDto {
    pub id: u64,
    pub name: String,
    pub parent_id: u64,
    pub description: String,
    pub sort_order: i32,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    /// 子分组（递归）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<GroupDto>,
    /// 成员设备数量
    pub member_count: u64,
}

impl From<GbDeviceGroup> for GroupDto {
    fn from(g: GbDeviceGroup) -> Self {
        Self {
            id: g.id,
            name: g.name,
            parent_id: g.parent_id,
            description: g.description,
            sort_order: g.sort_order,
            created_by: g.created_by,
            created_at: g.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: g.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            children: vec![],
            member_count: 0,
        }
    }
}

/// 创建/更新分组请求体
#[derive(Deserialize)]
pub struct SaveGroupReq {
    pub name: String,
    #[serde(default)]
    pub parent_id: u64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sort_order: i32,
}

/// 添加成员请求体
#[derive(Deserialize)]
pub struct AddMemberReq {
    /// 设备国标编码列表
    pub device_ids: Vec<String>,
}

// ══════════════════════════════════
//  分组 CRUD
// ══════════════════════════════════

/// GET /api/v1/gb28181/groups — 分组树列表
pub async fn list_groups(
    State(mut db): State<Db>,
) -> R<Vec<GroupDto>> {
    let groups = match GbDeviceGroup::all()
        .exec(&mut db)
        .await
    {
        Ok(gs) => gs,
        Err(e) => return R::error(500, format!("查询分组失败: {}", e)),
    };

    // 构建树形结构
    let dtos: Vec<GroupDto> = groups.into_iter().map(GroupDto::from).collect();
    let tree = build_group_tree(dtos, 0);

    // 填充每个分组的成员数量
    let result = fill_member_counts(&mut db, tree).await;
    R::ok(result)
}

/// 递归构建分组树（注意：需要 owned 返回，不能用引用）
fn build_group_tree(mut all: Vec<GroupDto>, parent_id: u64) -> Vec<GroupDto> {
    let mut children: Vec<GroupDto> = vec![];
    let mut rest: Vec<GroupDto> = vec![];

    for g in all.drain(..) {
        if g.parent_id == parent_id {
            children.push(g);
        } else {
            rest.push(g);
        }
    }

    for child in &mut children {
        child.children = build_group_tree(rest.clone(), child.id);
    }
    children
}

/// 填充成员数量（简单实现：逐个查询）
/// 使用 Box::pin 打包递归 async 调用，避免无限大小的 future
async fn fill_member_counts(db: &mut Db, mut groups: Vec<GroupDto>) -> Vec<GroupDto> {
    for g in &mut groups {
        let cnt = GbDeviceGroupMember::filter_by_group_id(g.id)
            .count()
            .exec(db)
            .await
            .unwrap_or(0);
        g.member_count = cnt;
        g.children = Box::pin(fill_member_counts(db, std::mem::take(&mut g.children))).await;
    }
    groups
}

/// POST /api/v1/gb28181/groups — 创建分组
pub async fn create_group(
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<SaveGroupReq>,
) -> R<GroupDto> {
    if req.name.is_empty() {
        return R::error(400, "分组名称不能为空".to_string());
    }
    // 检查父分组是否存在（parent_id > 0 时）
    if req.parent_id > 0 {
        if GbDeviceGroup::get_by_id(&mut db, req.parent_id).await.is_err() {
            return R::error(400, format!("父分组不存在: {}", req.parent_id));
        }
    }

    // 用 login_id (Option<String>) 提取操作人
    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };

    // 用 toasty create! 宏插入（与 auth.rs create_user 模式一致）
    let group = toasty::create!(GbDeviceGroup {
        name: req.name,
        parent_id: req.parent_id,
        description: req.description,
        sort_order: req.sort_order,
        created_by: operator.clone(),
    })
    .exec(&mut db)
    .await;

    let saved = match group {
        Ok(g) => g,
        Err(e) => return R::error(500, format!("创建分组失败: {}", e)),
    };

    // 写审计日志
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "group_create",
        &saved.name, &format!("创建分组: {}", saved.name),
    ).await;

    let mut dto = GroupDto::from(saved);
    dto.member_count = 0;
    R::ok(dto)
}

/// GET /api/v1/gb28181/groups/:id — 分组详情
pub async fn get_group(
    Path(id): Path<u64>,
    State(mut db): State<Db>,
) -> R<GroupDto> {
    let group = match GbDeviceGroup::get_by_id(&mut db, id).await {
        Ok(g) => g,
        Err(_) => return R::error(404, format!("分组不存在: {}", id)),
    };
    let mut dto = GroupDto::from(group);
    dto.member_count = GbDeviceGroupMember::filter_by_group_id(id)
        .count()
        .exec(&mut db)
        .await
        .unwrap_or(0);
    R::ok(dto)
}

/// PUT /api/v1/gb28181/groups/:id — 更新分组
pub async fn update_group(
    Path(id): Path<u64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<SaveGroupReq>,
) -> R<GroupDto> {
    let mut group = match GbDeviceGroup::get_by_id(&mut db, id).await {
        Ok(g) => g,
        Err(_) => return R::error(404, format!("分组不存在: {}", id)),
    };
    if req.parent_id > 0 && req.parent_id == id {
        return R::error(400, "不能将分组设为自己的子分组".to_string());
    }
    group.name = req.name;
    group.parent_id = req.parent_id;
    group.description = req.description;
    group.sort_order = req.sort_order;

    // update().exec(&mut db).await 模式（与 auth.rs update_user 一致）
    match group.update().exec(&mut db).await {
        Ok(_) => {}
        Err(e) => return R::error(500, format!("更新分组失败: {}", e)),
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "group_update",
        &group.name, &format!("更新分组: {}", group.name),
    ).await;

    // 重新查询获取更新后的数据
    let updated = match GbDeviceGroup::get_by_id(&mut db, id).await {
        Ok(g) => g,
        Err(e) => return R::error(500, format!("查询更新后数据失败: {}", e)),
    };
    let mut dto = GroupDto::from(updated);
    dto.member_count = GbDeviceGroupMember::filter_by_group_id(id)
        .count()
        .exec(&mut db)
        .await
        .unwrap_or(0);
    R::ok(dto)
}

/// DEL /api/v1/gb28181/groups/:id — 删除分组（递归）
pub async fn delete_group(
    Path(id): Path<u64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> R<String> {
    // 递归删除子分组
    if let Err(e) = delete_group_recursive(&mut db, id).await {
        return R::error(500, format!("删除分组失败: {}", e));
    }
    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "group_delete",
        &id.to_string(), &format!("删除分组: {}", id),
    ).await;
    R::ok("分组已删除".to_string())
}

/// 递归删除分组及其子分组 + 成员关联
/// 使用 Box::pin 打包递归 async 调用，避免无限大小的 future
async fn delete_group_recursive(db: &mut Db, id: u64) -> Result<(), String> {
    // 删除成员关联
    let members = match GbDeviceGroupMember::filter_by_group_id(id).exec(db).await {
        Ok(m) => m,
        Err(_) => vec![],
    };
    for m in members {
        // 用 delete_by_id 删除成员记录
        if let Err(e) = GbDeviceGroupMember::delete_by_id(db, m.id).await {
            tracing::warn!("删除分组成员失败 id={}: {}", m.id, e);
        }
    }

    // 查找子分组（parent_id 有 #[index]，支持 filter_by_parent_id）
    let children = match GbDeviceGroup::filter_by_parent_id(id).exec(db).await {
        Ok(c) => c,
        Err(_) => vec![],
    };
    for child in children {
        if let Err(e) = Box::pin(delete_group_recursive(db, child.id)).await {
            tracing::warn!("递归删除子分组失败 id={}: {}", child.id, e);
        }
    }

    // 删除自己
    if let Err(e) = GbDeviceGroup::delete_by_id(db, id).await {
        return Err(format!("删除分组 {} 失败: {}", id, e));
    }
    Ok(())
}

// ══════════════════════════════════
//  成员管理
// ══════════════════════════════════

/// GET /api/v1/gb28181/groups/:id/members — 分组内设备列表
pub async fn list_members(
    Path(id): Path<u64>,
    State(mut db): State<Db>,
    Query(p): Query<crate::api::devices::Pagination>,
) -> R<PageData<crate::dto::DeviceDto>> {
    let offset = p.offset();
    let members = match GbDeviceGroupMember::filter_by_group_id(id)
        .offset(offset as usize)
        .limit(p.page_size as usize)
        .exec(&mut db)
        .await
    {
        Ok(m) => m,
        Err(e) => return R::error(500, format!("查询成员失败: {}", e)),
    };

    let total = match GbDeviceGroupMember::filter_by_group_id(id).count().exec(&mut db).await {
        Ok(n) => n,
        Err(_) => 0u64,
    };

    let mut items = vec![];
    for m in members {
        // first() 返回 Option<GbDeviceRecord>
        match GbDeviceRecord::filter_by_device_id(m.device_id.clone()).first().exec(&mut db).await {
            Ok(Some(d)) => { items.push(crate::dto::DeviceDto::from(d)); }
            _ => {}
        }
    }

    let total_pages = if p.page_size == 0 { 0 } else { (total + p.page_size - 1) / p.page_size };
    R::ok(PageData {
        items,
        total,
        page: p.page,
        page_size: p.page_size,
        total_pages,
    })
}

/// POST /api/v1/gb28181/groups/:id/members — 添加设备到分组
pub async fn add_members(
    Path(id): Path<u64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<AddMemberReq>,
) -> R<String> {
    // 验证分组存在
    if GbDeviceGroup::get_by_id(&mut db, id).await.is_err() {
        return R::error(404, format!("分组不存在: {}", id));
    }
    let mut added = 0usize;
    let mut skipped = 0usize;
    for did in &req.device_ids {
        // 检查设备是否存在（first() 返回 Option）
        match GbDeviceRecord::filter_by_device_id(did.clone()).first().exec(&mut db).await {
            Ok(None) | Err(_) => { skipped += 1; continue; }
            Ok(Some(_)) => {}
        }
        // 检查是否已在该分组
        let exists = match GbDeviceGroupMember::filter_by_group_id(id)
            .filter_by_device_id(did.clone())
            .first()
            .exec(&mut db)
            .await
        {
            Ok(Some(_)) => true,
            _ => false,
        };
        if exists {
            skipped += 1;
            continue;
        }
        // 用 create! 插入成员记录
        let member = toasty::create!(GbDeviceGroupMember {
            group_id: id,
            device_id: did.clone(),
        });
        match member.exec(&mut db).await {
            Ok(_) => added += 1,
            Err(_) => skipped += 1,
        }
    }
    // 同步更新 GbDeviceRecord.group_id（单设备时更新主记录的归属分组标记）
    if added > 0 && req.device_ids.len() == 1 {
        match GbDeviceRecord::filter_by_device_id(req.device_ids[0].clone()).first().exec(&mut db).await {
            Ok(Some(mut d)) => {
                d.group_id = id;
                let _ = d.update().exec(&mut db).await;
            }
            _ => {}
        }
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "group_add_member",
        &id.to_string(), &format!("添加 {} 个设备到分组", added),
    ).await;

    R::ok(format!("已添加 {} 个设备，跳过 {} 个", added, skipped))
}

/// DEL /api/v1/gb28181/groups/:id/members/:did — 从分组移除设备
pub async fn remove_member(
    Path((id, did)): Path<(u64, String)>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> R<String> {
    let members = match GbDeviceGroupMember::filter_by_group_id(id)
        .filter_by_device_id(did.clone())
        .exec(&mut db)
        .await
    {
        Ok(m) => m,
        Err(_) => vec![],
    };
    for m in members {
        if let Err(e) = GbDeviceGroupMember::delete_by_id(&mut db, m.id).await {
            tracing::warn!("移除分组成员失败 id={}: {}", m.id, e);
        }
    }
    // 清除 GbDeviceRecord.group_id（first() 返回 Option，需解包）
    match GbDeviceRecord::filter_by_device_id(did.clone()).first().exec(&mut db).await {
        Ok(Some(mut d)) => {
            d.group_id = 0;
            let _ = d.update().exec(&mut db).await;
        }
        _ => {}
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "group_remove_member",
        &id.to_string(), &format!("从分组移除设备: {}", did),
    ).await;

    R::ok("设备已从分组移除".to_string())
}
