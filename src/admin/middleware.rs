//! Admin API 中间件

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};

use super::client_keys::SharedClientKeyManager;
use super::groups::SharedGroupManager;
use super::service::AdminService;
use super::types::AdminErrorResponse;
use super::usage_stats::SharedAggregator;
use super::trace_db::SharedTraceStore;
use crate::common::auth;

/// Admin API 共享状态
#[derive(Clone)]
pub struct AdminState {
    /// Admin 服务
    pub service: Arc<AdminService>,
    /// 客户端 Key 管理器（与 anthropic 路由共享）
    pub client_keys: SharedClientKeyManager,
    /// 用量聚合器（与 anthropic 路由共享）
    pub usage_aggregator: SharedAggregator,
    /// 请求链路追踪存储（与 anthropic 路由共享）
    pub trace_store: SharedTraceStore,
    /// 账号分组注册表（持久化到 groups.json）
    pub groups: SharedGroupManager,
}

impl AdminState {
    pub fn new(
        service: AdminService,
        client_keys: SharedClientKeyManager,
        usage_aggregator: SharedAggregator,
        trace_store: SharedTraceStore,
        groups: SharedGroupManager,
    ) -> Self {
        Self {
            service: Arc::new(service),
            client_keys,
            usage_aggregator,
            trace_store,
            groups,
        }
    }
}

/// Admin API 认证中间件 — 统一走客户端 Key 校验
pub async fn admin_auth_middleware(
    State(state): State<AdminState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let presented = match auth::extract_api_key(&request) {
        Some(k) => k,
        None => {
            let error = AdminErrorResponse::authentication_error();
            return (StatusCode::UNAUTHORIZED, Json(error)).into_response();
        }
    };

    if state.client_keys.verify_and_touch(&presented).is_some() {
        return next.run(request).await;
    }

    let error = AdminErrorResponse::authentication_error();
    (StatusCode::UNAUTHORIZED, Json(error)).into_response()
}
