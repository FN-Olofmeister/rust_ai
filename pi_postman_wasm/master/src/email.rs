// master/src/email.rs
use anyhow::Result;
use common::email::{process_incoming_email, get_email};
use crate::ai::classify_with_ai;

/// process_incoming_email를 래핑합니다.
pub async fn receive_email(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<String> {
    process_incoming_email(from, to, subject, body).await
}

/// get_email + classify_with_ai를 묶어서 호출합니다.
pub async fn classify_email(email_id: &str) -> Result<(String, f32)> {
    let email = get_email(email_id)?;
    classify_with_ai(&email).await
}