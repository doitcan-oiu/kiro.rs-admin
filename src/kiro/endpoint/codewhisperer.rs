//! Kiro CodeWhisperer 端点（Amazon CodeWhisperer Streaming / GenerateAssistantResponse）
//!
//! 对应 demo（Kiro-Go）里的 "CodeWhisperer" 端点：`AI_EDITOR` origin、注入 profileArn，
//! 与 [`super::ide::IdeEndpoint`] 的请求体加工完全一致，区别在于：
//! - 携带 `x-amz-target: AmazonCodeWhispererStreamingService.GenerateAssistantResponse`；
//! - **主机不同**：`us-east-1` 走独有的 `codewhisperer.us-east-1.amazonaws.com` 主机
//!   （一个与 `q.*` / `runtime.kiro.dev` 相互独立的限流桶）。
//!
//! 区域折叠规则（对齐 demo 的 `regionalizeURLForRegion`）：CodeWhisperer 的 REST 主机
//! **只在 us-east-1 存在**——非 us-east-1 区域没有 `codewhisperer.{region}` 主机，统一
//! 回退到区域化的 Amazon Q 主机 `q.{region}.amazonaws.com`。us-east-1 或空区域为原样。

use reqwest::RequestBuilder;
use uuid::Uuid;

use super::ide::inject_profile_arn;
use super::{KiroEndpoint, RequestContext};
use crate::kiro::kiro_version;

/// Kiro CodeWhisperer 端点名称
pub const CODEWHISPERER_ENDPOINT_NAME: &str = "codewhisperer";

/// Amazon CodeWhisperer Streaming 的 x-amz-target 值（GenerateAssistantResponse 操作）
const CODEWHISPERER_AMZ_TARGET: &str =
    "AmazonCodeWhispererStreamingService.GenerateAssistantResponse";

/// Kiro CodeWhisperer 端点
pub struct CodeWhispererEndpoint;

impl CodeWhispererEndpoint {
    pub fn new() -> Self {
        Self
    }

    fn api_region<'a>(&self, ctx: &'a RequestContext<'_>) -> &'a str {
        ctx.credentials.effective_api_region(ctx.config)
    }

    /// 计算实际主机：us-east-1 / 空区域用独有的 codewhisperer 主机，其余折叠到区域化 q 主机。
    fn host(&self, ctx: &RequestContext<'_>) -> String {
        let region = self.api_region(ctx).trim();
        if region.is_empty() || region == "us-east-1" {
            "codewhisperer.us-east-1.amazonaws.com".to_string()
        } else {
            format!("q.{}.amazonaws.com", region)
        }
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

impl Default for CodeWhispererEndpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl KiroEndpoint for CodeWhispererEndpoint {
    fn name(&self) -> &'static str {
        CODEWHISPERER_ENDPOINT_NAME
    }

    fn api_url(&self, ctx: &RequestContext<'_>) -> String {
        format!("https://{}/generateAssistantResponse", self.host(ctx))
    }

    fn mcp_url(&self, ctx: &RequestContext<'_>) -> String {
        format!("https://{}/mcp", self.host(ctx))
    }

    fn decorate_api(&self, req: RequestBuilder, ctx: &RequestContext<'_>) -> RequestBuilder {
        // 与 IdeEndpoint 完全一致，额外携带 x-amz-target 路由到 CodeWhisperer GenerateAssistantResponse
        let mut req = req
            .header("x-amz-target", CODEWHISPERER_AMZ_TARGET)
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
    fn test_codewhisperer_host_for_us_east_1() {
        let endpoint = CodeWhispererEndpoint::new();
        let mut config = Config::default();
        config.api_region = Some("us-east-1".to_string());
        let creds = KiroCredentials::default();
        let rctx = ctx(&creds, &config, "machine");

        assert_eq!(endpoint.name(), "codewhisperer");
        assert_eq!(endpoint.host(&rctx), "codewhisperer.us-east-1.amazonaws.com");
        assert_eq!(
            endpoint.api_url(&rctx),
            "https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse"
        );
    }

    #[test]
    fn test_codewhisperer_folds_to_regional_q_host_outside_us_east_1() {
        let endpoint = CodeWhispererEndpoint::new();
        let mut config = Config::default();
        config.api_region = Some("eu-central-1".to_string());
        let creds = KiroCredentials::default();
        let rctx = ctx(&creds, &config, "machine");

        // 非 us-east-1：没有 codewhisperer.{region} 主机，折叠到区域化 q 主机
        assert_eq!(endpoint.host(&rctx), "q.eu-central-1.amazonaws.com");
        assert_eq!(
            endpoint.api_url(&rctx),
            "https://q.eu-central-1.amazonaws.com/generateAssistantResponse"
        );
    }
}
