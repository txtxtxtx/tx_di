use admin_domain::department::model::aggregate::Department;

// Re-export proto types directly (no hand-written DTOs)
pub use admin_proto::{CreateDeptRequest, UpdateDeptRequest, ListDeptsRequest, DeptResponse};

/// 将领域层的 Department 聚合根转换为 proto 的 DeptResponse
pub fn dept_to_response(dept: Department) -> DeptResponse {
    DeptResponse {
        id: dept.id,
        name: dept.name,
        parent_id: dept.parent_id,
        sort: dept.sort,
        leader_user_id: dept.leader_user_id,
        phone: dept.phone,
        email: dept.email,
        status: dept.status,
    }
}
