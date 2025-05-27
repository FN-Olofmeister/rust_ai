// common/src/classifier.rs
use anyhow::{anyhow, Result};
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    Credentials,
};
use regex::Regex;
use serde::Deserialize;
use std::{env, time::Duration};
use tokio::time::timeout;
use tracing::{error, info};

#[derive(Deserialize)]
struct Classification {
    category: String,
    confidence: f32,
}

/// 이메일 제목/본문을 AI로 분류 (정규식 + 타임아웃 개선)
pub async fn classify_via_openai(subject: &str, body: &str) -> Result<(String, f32)> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow!("환경변수 OPENAI_API_KEY가 설정되어야 합니다"))?;
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1".to_string());
    let api_url = env::var("OPENAI_API_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let to_sec: u64 = env::var("OPENAI_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let creds = Credentials::new(api_key, api_url);
    let system = ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some(
            "당신은 이메일 스팸 분류 전문가입니다. \
             결과는 정확히 JSON 하나만, 예시처럼 응답하세요:\n\
             {\"category\":\"SPAM\",\"confidence\":0.87}"
                .to_string(),
        ),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    };
    let user = ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(format!("제목: {}\n본문:\n{}", subject, body)),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    };

    info!("[AI] model={} timeout={}s subject='{}' body_len={}", model, to_sec, subject, body.len());
    let send_fut = ChatCompletion::builder(&model, vec![system, user])
        .credentials(creds)
        .create();
    let response = timeout(Duration::from_secs(to_sec), send_fut)
        .await
        .map_err(|_| anyhow!("OpenAI 호출 타임아웃 ({}초)", to_sec))?
        .map_err(|e| anyhow!("AI 호출 실패: {}", e))?;

    let full = response
        .choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    // ```json ... ``` 사이 콘텐츠 추출
    let re = Regex::new(r#"```json\s*([\s\S]*?)\s*```"#).unwrap();
    let json_block = re
        .captures(&full)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .unwrap_or(&full);

    match serde_json::from_str::<Classification>(json_block) {
        Ok(c) => Ok((c.category, c.confidence)),
        Err(e) => {
            error!("JSON 파싱 실패: {} | 추출된 JSON: {} | 전체 응답: {}", e, json_block, full);
            Err(anyhow!("JSON 파싱 실패: {}", e))
        }
    }
}
