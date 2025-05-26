//worker/src/api.rs

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ClassifyEmailRequest {
    pub email_id: String,
}

#[derive(Deserialize)]
pub struct ClassifyEmailResponse {
    pub category: String,
    pub confidence: f32,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn classify_email(&self, email_id: &str) -> Result<(String, f32)> {
        // 1) 호출 URL 로깅
        println!(
            "👉 Worker calling classify: {}/api/email/classify (email_id={})",
            self.base_url, email_id
        );
        // 2) 요청 빌드
        let req = ClassifyEmailRequest {
            email_id: email_id.to_string(),
        };

        // 3) POST 호출 (self.client을 borrow)
        let resp = (&self.client)
            .post(format!("{}/api/email/classify", self.base_url))
            .json(&req)
            .send()
            .await
            .map_err(|e| anyhow!("분류 요청 실패: {}", e))?;

        // 4) HTTP 상태 확인
        if !resp.status().is_success() {
            return Err(anyhow!("분류 요청 오류 코드: {}", resp.status()));
        }

        // 5) 본문 파싱
        let body = resp
            .json::<ClassifyEmailResponse>()
            .await
            .map_err(|e| anyhow!("분류 응답 파싱 실패: {}", e))?;

        Ok((body.category, body.confidence))
    }
}
