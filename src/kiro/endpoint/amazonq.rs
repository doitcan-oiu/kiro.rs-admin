//! Kiro AmazonQ 端点（Amazon Q Developer Streaming / SendMessage）
//!
//! 对应 demo（Kiro-Go）里的 "AmazonQ" 端点：与 [`super::ide::IdeEndpoint`] 走**同一个**
//! 主机与路径（`https://q.{api_region}.amazonaws.com/generateAssistantResponse`）、同样的
//! `AI_EDITOR` origin 与请求体加工（注入 profileArn），**唯一差别**是携带
//! `x-amz-target: AmazonQDeveloperStreamingService.SendMessage`，从而路由到
//! Amazon Q Developer 的 SendMessage 服务。
//!
//! 关键价值：它与 `ide`（无 x-amz-target）、`runtime`（runtime.kiro.dev）互为不同的
//! 上游服务/限流桶——某一个 429 / 5xx 时切到本端点仍可能 200，是多端点重试的又一路回退。

use reqwest::RequestBuilder;
use uuid::Uuid;

use super::ide::inject_profile_arn;
use super::{KiroEndpoint, RequestContext};
use crate::kiro::kiro_version;

/// Kiro AmazonQ 端点名称
pub const AMAZONQ_ENDPOINT_NAME: &str = "amazonq";

/// Amazon Q Developer Streaming 的 x-amz-target 值（SendMessage 操作）
const AMAZONQ_AMZ_TARGET: &str = "AmazonQDeveloperStreamingService.SendMessage";

/// Kiro AmazonQ 端点
pub struct AmazonQEndpoint;

impl AmazonQEndpoint {
    pub fn new() -> Self {
        Self
    }

    fn api_region<'a>(&self, ctx: &'a RequestContext<'_>) -> &'a str {
        ctx.credentials.effective_api_region(ctx.config)
    }

    fn host(&self, ctx: &RequestContext<'_>) -> String {
        format!("q.{}.amazonaws.com", self.api_region(ctx))
    }

    fn x_amz_user_agent(&self, ctx: &RequestContext<'_>) -> String {
        format!(
            "aws-sdk-js/1.0.34 KiroIDE-{}-{}",
            kiro_version::effective(&ctx.config.kiro_version),
            ctx.machine_id
        )
    }

    fn user_agent(&self, ctx: &RequestContext<'_>) -> String {
        format!(
            "aws-sdk-js/1.0.34 ua/2.1 os/{} lang/js md/nodejs#{} api/codewhispererstreaming#1.0.34 m/E KiroIDE-{}-{}",
            ctx.config.system_version,
            ctx.config.node_version,
            kiro_version::effective(&ctx.config.kiro_version),
            ctx.machine_id
        )
    }
}

impl Default for AmazonQEndpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl KiroEndpoint for AmazonQEndpoint {
    fn name(&self) -> &'static str {
        AMAZONQ_ENDPOINT_NAME
    }

    fn api_url(&self, ctx: &RequestContext<'_>) -> String {
        format!(
            "https://q.{}.amazonaws.com/generateAssistantResponse",
            self.api_region(ctx)
        )
    }

    fn mcp_url(&self, ctx: &RequestContext<'_>) -> String {
        format!("https://q.{}.amazonaws.com/mcp", self.api_region(ctx))
    }

    fn decorate_api(&self, req: RequestBuilder, ctx: &RequestContext<'_>) -> RequestBuilder {
        // 与 IdeEndpoint 完全一致，额外携带 x-amz-target 路由到 AmazonQ SendMessage 服务
        let mut req = req
            .header("x-amz-target", AMAZONQ_AMZ_TARGET)
            .header("x-amzn-codewhisperer-optout", "true")
            .header("x-amzn-kiro-agent-mode", "vibe")
            .header("x-amz-user-agent", self.x_amz_user_agent(ctx))
            .header("user-agent", self.user_agent(ctx))
            .header("host", self.host(ctx))
            .header("amz-sdk-invocation-id", Uuid::new_v4().to_string())
            .header("amz-sdk-request", "attempt=1; max=3")
            .header("Authorization", format!("Bearer {}", ctx.token));

        if let Some(token_type) = ctx.credentials.token_type_header() {
            req = req.header("tokentype", token_type);
        }
        req
    }

    fn decorate_mcp(&self, req: RequestBuilder, ctx: &RequestContext<'_>) -> RequestBuilder {
        let mut req = req
            .header("x-amz-user-agent", self.x_amz_user_agent(ctx))
            .header("user-agent", self.user_agent(ctx))
            .header("host", self.host(ctx))
            .header("amz-sdk-invocation-id", Uuid::new_v4().to_string())
            .header("amz-sdk-request", "attempt=1; max=3")
            .header("Authorization", format!("Bearer {}", ctx.token));

        if let Some(arn) = ctx.credentials.effective_profile_arn() {
            req = req.header("x-amzn-kiro-profile-arn", arn);
        }
        if let Some(token_type) = ctx.credentials.token_type_header() {
            req = req.header("tokentype", token_type);
        }
        req
    }

    fn transform_api_body(&self, body: &str, ctx: &RequestContext<'_>) -> String {
        inject_profile_arn(body, ctx.credentials.streaming_profile_arn().as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kiro::model::credentials::KiroCredentials;
    use crate::model::config::Config;

    fn ctx<'a>(
        creds: &'a KiroCredentials,
        config: &'a Config,
        machine_id: &'a str,
    ) -> RequestContext<'a> {
        RequestContext {
            credentials: creds,
            token: "tok",
            machine_id,
            config,
        }
    }

    #[test]
    fn test_amazonq_uses_q_host_generate_assistant_response() {
        let endpoint = AmazonQEndpoint::new();
        let mut config = Config::default();
        config.api_region = Some("us-east-1".to_string());
        let creds = KiroCredentials::default();
        let rctx = ctx(&creds, &config, "machine");

        assert_eq!(endpoint.name(), "amazonq");
        assert_eq!(
            endpoint.api_url(&rctx),
            "https://q.us-east-1.amazonaws.com/generateAssistantResponse"
        );
        assert_eq!(endpoint.host(&rctx), "q.us-east-1.amazonaws.com");
    }
}
