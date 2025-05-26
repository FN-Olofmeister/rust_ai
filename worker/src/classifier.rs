//worker/src/classifier.rs

use common::classifier::classify_via_openai;
use anyhow::Result;

/// AI 단일 필터 분류기 (worker 전용)
pub async fn classify_two_tier(subject: &str, body: &str) -> Result<(String, f32)> {
    classify_via_openai(subject, body)
        .await
        .map_err(|e| { eprintln!("[ERROR] OpenAI 분류 실패(worker): {}", e); e.into() })
}
