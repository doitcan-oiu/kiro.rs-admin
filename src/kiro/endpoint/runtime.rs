//! Kiro Runtime 端点
//!
//! 对应 Kiro IDE 客户端较新的 `runtime.kiro.dev` 推理链路：
//! - API: `https://runtime.{api_region}.kiro.dev/generateAssistantResponse`
//! - MCP: `https://runtime.{api_region}.kiro.dev/mcp`
//!
//! 请求头、请求体加工（注入 profileArn）与 [`super::ide::IdeEndpoint`]
//! **完全一致**，唯一差别是域名从 `q.{region}.amazonaws.com` 换成
//! `runtime.{region}.kiro.dev`。
//!
//! 关键价值：实测 `runtime.kiro.dev` 与 `q.amazonaws.com` 是**两个独立的限流桶**——
//! 一个 429 时另一个仍可 200。

use reqwest::RequestBuilder;
use uuid::Uuid;

use super::ide::inject_profile_arn;
use super::{KiroEndpoint, RequestContext};
use crate::kiro::kiro_version;

/// Kiro Runtime 端点名称
pub const RUNTIME_ENDPOINT_NAME: &str = "runtime";

/// Kiro Runtime 端点
pub struct RuntimeEndpoint;

impl RuntimeEndpoint {
    pub fn new() -> Self {
        Self
    }

    fn api_region<'a>(&self, ctx: &'a RequestContext<'_>) -> &'a str {
        ctx.credentials.effective_api_region(ctx.config)
    }

    fn host(&self, ctx: &RequestContext<'_>) -> String {
        format!("runtime.{}.kiro.dev", self.api_region(ctx))
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

impl Default for RuntimeEndpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl KiroEndpoint for RuntimeEndpoint {
    fn name(&self) -> &'static str {
        RUNTIME_ENDPOINT_NAME
    }

    fn api_url(&self, ctx: &RequestContext<'_>) -> String {
        format!(
            "https://runtime.{}.kiro.dev/generateAssistantResponse",
            self.api_region(ctx)
        )
    }

    fn mcp_url(&self, ctx: &RequestContext<'_>) -> String {
        format!("https://runtime.{}.kiro.dev/mcp", self.api_region(ctx))
    }

    fn decorate_api(&self, req: RequestBuilder, ctx: &RequestContext<'_>) -> RequestBuilder {
        let mut req = req
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
    fn test_runtime_urls_use_kiro_dev_domain() {
        let endpoint = RuntimeEndpoint::new();
        let mut config = Config::default();
        config.api_region = Some("us-east-1".to_string());
        let creds = KiroCredentials::default();
        let rctx = ctx(&creds, &config, "machine");

        assert_eq!(
            endpoint.api_url(&rctx),
            "https://runtime.us-east-1.kiro.dev/generateAssistantResponse"
        );
        assert_eq!(endpoint.mcp_url(&rctx), "https://runtime.us-east-1.kiro.dev/mcp");
        assert_eq!(endpoint.host(&rctx), "runtime.us-east-1.kiro.dev");
    }
}
