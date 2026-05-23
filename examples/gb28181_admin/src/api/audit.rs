//! 设备注册审核 API
//!
//! ## 功能清单
//!
//! - `GET  /gb28181/register_audit`         — 审核列表（分页+状态筛选）
//! - `GET  /gb28181/register_audit/:id`    — 审核详情
//! - `POST /gb28181/register_audit/:id/approve` — 批准注册
//! - `POST /gb28181/register_audit/:id/reject`  — 拒绝注册
//! - `DEL  /gb28181/register_audit/:id`      — 删除审核记录
//! - `POST /gb28181/register_audit/auto_approve` — 自动批准（按设备厂商白名单）

use axum::{
    extract::{Path, Query, State, Json as ExtJson},
};
use serde::{Deserialize, Serialize};
use tx_di_axum::R;
use tx_di_sa_token::LoginIdExtractor;
use toasty::Db;

use crate::models::GbRegisterAudit;
use crate::dto::PageData;

// ════════════════════════════════
//  DTO
// ════════════════════════════════

/// 审核列表查询参数
#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_audit_page")]
    pub page: u64,
    #[serde(default = "default_audit_page_size")]
    pub page_size: u64,
}
fn default_audit_page() -> u64 { 1 }
fn default_audit_page_size() -> u64 { 20 }

/// 审核记录 DTO
#[derive(Serialize)]
pub struct RegisterAuditDto {
    pub id: i64,
    pub device_id: String,
    pub contact: String,
    pub remote_ip: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware: String,
    pub status: String,
    pub auditor: String,
    pub audit_remark: String,
    pub apply_remark: String,
    pub audited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GbRegisterAudit> for RegisterAuditDto {
    fn from(a: GbRegisterAudit) -> Self {
        Self {
            id: a.id,
            device_id: a.device_id,
            contact: a.contact,
            remote_ip: a.remote_ip,
            manufacturer: a.manufacturer,
            model: a.model,
            firmware: a.firmware,
            status: a.status,
            auditor: a.auditor,
            audit_remark: a.audit_remark,
            apply_remark: a.apply_remark,
            audited_at: a.audited_at.map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string()),
            created_at: a.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: a.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// 审核操作请求体（批准/拒绝共用）
#[derive(Deserialize)]
pub struct AuditActionReq {
    /// 审核备注
    #[serde(default)]
    pub remark: String,
}

// ════════════════════════════════
//  审核列表 & 详情
// ════════════════════════════════

/// GET /api/v1/gb28181/register_audit — 审核列表
pub async fn list_audits(
    State(mut db): State<Db>,
    Query(q): Query<AuditListQuery>,
) -> R<PageData<RegisterAuditDto>> {
    let total = if let Some(ref s) = q.status {
        match GbRegisterAudit::filter_by_status(s.clone()).count().exec(&mut db).await {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询总数失败: {}", e)),
        }
    } else {
        match GbRegisterAudit::all().count().exec(&mut db).await {
            Ok(n) => n,
            Err(e) => return R::error(500, format!("查询总数失败: {}", e)),
        }
    };

    let offset = (q.page.saturating_sub(1)) * q.page_size;
    let records = if let Some(ref s) = q.status {
        GbRegisterAudit::filter_by_status(s.clone())
            .offset(offset as usize)
            .limit(q.page_size as usize)
            .exec(&mut db)
            .await
    } else {
        GbRegisterAudit::all()
            .offset(offset as usize)
            .limit(q.page_size as usize)
            .exec(&mut db)
            .await
    };

    let records = match records {
        Ok(r) => r,
        Err(e) => return R::error(500, format!("查询审核记录失败: {}", e)),
    };

    let items: Vec<RegisterAuditDto> = records.into_iter().map(RegisterAuditDto::from).collect();
    let total_pages = if q.page_size == 0 { 0 } else { (total + q.page_size - 1) / q.page_size };

    R::ok(PageData {
        items,
        total,
        page: q.page,
        page_size: q.page_size,
        total_pages,
    })
}

/// GET /api/v1/gb28181/register_audit/:id — 审核详情
pub async fn get_audit(
    Path(id): Path<i64>,
    State(mut db): State<Db>,
) -> R<RegisterAuditDto> {
    match GbRegisterAudit::get_by_id(&mut db, id).await {
        Ok(a) => R::ok(RegisterAuditDto::from(a)),
        Err(_) => R::error(404, format!("审核记录不存在: {}", id)),
    }
}

// ════════════════════════════════
//  审核操作
// ════════════════════════════════

/// POST /api/v1/gb28181/register_audit/:id/approve — 批准注册
pub async fn approve(
    Path(id): Path<i64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<AuditActionReq>,
) -> R<String> {
    let mut audit = match GbRegisterAudit::get_by_id(&mut db, id).await {
        Ok(a) => a,
        Err(_) => return R::error(404, format!("审核记录不存在: {}", id)),
    };
    if audit.status == "approved" {
        return R::error(400, "该申请已批准，无需重复操作".to_string());
    }
    if audit.status == "rejected" {
        return R::error(400, "该申请已被拒绝，请重新提交".to_string());
    }

    // 从 login_id (Option<String>) 提取操作人
    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };

    // 更新审核记录（使用 update().exec(&mut db).await 模式）
    audit.status = "approved".to_string();
    audit.auditor = operator.clone();
    audit.audit_remark = req.remark;
    audit.audited_at = Some(jiff::Timestamp::now());

    if let Err(e) = audit.update().exec(&mut db).await {
        return R::error(500, format!("更新审核记录失败: {}", e));
    }

    // 写审计日志
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "register_approve",
        &audit.device_id, &format!("批准设备注册: {}", audit.device_id),
    ).await;

    // TODO: 将设备加入内存设备表（需通过 DiComp<Gb28181Server> 调用）
    // 目前仅更新数据库状态，实际设备上线由 SIP REGISTER 流程处理

    R::ok(format!("已批准设备 {} 的注册申请", audit.device_id))
}

/// POST /api/v1/gb28181/register_audit/:id/reject — 拒绝注册
pub async fn reject(
    Path(id): Path<i64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<AuditActionReq>,
) -> R<String> {
    let mut audit = match GbRegisterAudit::get_by_id(&mut db, id).await {
        Ok(a) => a,
        Err(_) => return R::error(404, format!("审核记录不存在: {}", id)),
    };
    if audit.status == "rejected" {
        return R::error(400, "该申请已被拒绝，无需重复操作".to_string());
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };

    audit.status = "rejected".to_string();
    audit.auditor = operator.clone();
    audit.audit_remark = req.remark;
    audit.audited_at = Some(jiff::Timestamp::now());

    if let Err(e) = audit.update().exec(&mut db).await {
        return R::error(500, format!("更新审核记录失败: {}", e));
    }

    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "register_reject",
        &audit.device_id, &format!("拒绝设备注册: {}", audit.device_id)
    ).await;

    R::ok(format!("已拒绝设备 {} 的注册申请", audit.device_id))
}

/// DEL /api/v1/gb28181/register_audit/:id — 删除审核记录
pub async fn delete_audit(
    Path(id): Path<i64>,
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> R<String> {
    // 先读取记录用于审计（delete_by_id 前需要知道 device_id）
    let audit = match GbRegisterAudit::get_by_id(&mut db, id).await {
        Ok(a) => a,
        Err(_) => return R::error(404, format!("审核记录不存在: {}", id)),
    };
    let device_id = audit.device_id.clone();

    // 使用 Model::delete_by_id 删除（与 admin.rs 删除用户模式一致）
    if let Err(e) = GbRegisterAudit::delete_by_id(&mut db, id).await {
        return R::error(500, format!("删除失败: {}", e));
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "register_audit_delete",
        &device_id, &format!("删除注册审核记录: {}", device_id)
    ).await;

    R::ok("审核记录已删除".to_string())
}

/// POST /api/v1/gb28181/register_audit/auto_approve — 按厂商白名单自动批准
///
/// 请求体：{ "manufacturers": ["Hikvision", "Dahua"] }
/// 将所有 pending 且厂商在白名单内的记录自动批准。
#[derive(Deserialize)]
pub struct AutoApproveReq {
    pub manufacturers: Vec<String>,
}

pub async fn auto_approve(
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
    ExtJson(req): ExtJson<AutoApproveReq>,
) -> R<String> {
    let pending = match GbRegisterAudit::filter_by_status("pending".to_string()).exec(&mut db).await {
        Ok(list) => list,
        Err(_) => vec![],
    };
    let mut approved = 0usize;
    let mut skipped = 0usize;

    for mut audit in pending {
        if !req.manufacturers.iter().any(|m| audit.manufacturer.contains(m)) {
            skipped += 1;
            continue;
        }
        audit.status = "approved".to_string();
        audit.auditor = "auto".to_string();
        audit.audit_remark = "自动批准（厂商白名单）".to_string();
        audit.audited_at = Some(jiff::Timestamp::now());
        // update().exec(&mut db).await 模式
        if audit.update().exec(&mut db).await.is_ok() {
            approved += 1;
        } else {
            skipped += 1;
        }
    }

    let operator = if login_id.is_empty() {
        "system".to_string()
    } else {
        format!("user_{}", login_id)
    };
    let _ = crate::api::admin::write_audit(
        &mut db, &operator, "register_auto_approve",
        "", &format!("自动批准 {} 条注册申请", approved)
    ).await;

    R::ok(format!("自动批准 {} 条，跳过 {} 条", approved, skipped))
}
