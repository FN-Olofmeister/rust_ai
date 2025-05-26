//master/src/ai.rs

use anyhow::Result;
use common::email::Email;
use common::classifier::classify_via_openai;

/// 이메일 객체를 AI에 보내 분류 결과 반환
pub async fn classify_with_ai(email: &Email) -> Result<(String, f32)> {
    // common::classifier 내부 로직 사용
    classify_via_openai(&email.subject, &email.body).await
}