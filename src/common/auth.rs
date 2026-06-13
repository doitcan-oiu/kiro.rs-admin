//! 公共认证工具函数

use axum::{
    body::Body,
    http::{Request, header},
};

/// 从请求中提取 API Key
///
/// 支持两种认证方式：
/// - `x-api-key` header
/// - `Authorization: Bearer <token>` header
pub fn extract_api_key(request: &Request<Body>) -> Option<String> {
    // 优先检查 x-api-key
    if let Some(key) = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
    {
        return Some(key.to_string());
    }

    // 其次检查 Authorization: Bearer
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// constant_time_eq 已移除：统一走客户端 Key 校验后，时序安全的比较
// 由 ClientKeyManager 内部负责，无需对外暴露独立函数。
