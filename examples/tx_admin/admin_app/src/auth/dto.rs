// Re-export proto types for backward compatibility.
// All request/response types are defined in admin_proto.

pub use admin_proto::{
    GetUserInfoRequest, LoginRequest, LoginResponse, LogoutRequest, UserInfoResponse,
};
