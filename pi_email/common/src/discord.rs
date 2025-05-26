//common/src/discord.rs

use reqwest::Client;
use serde::Serialize;
use tracing::{error, info};

#[derive(Serialize)]
pub struct DiscordPayload {
    pub content: String,
}

pub async fn send_discord_alert(
    webhook_url: &str,
    subject: &str,
    sender: &str,
    category: &str,
    worker_id: Option<&str>, // 새로 추가된 매개변수
) -> Result<(), reqwest::Error> {
    let is_spam = category.eq_ignore_ascii_case("SPAM");
    let prefix = if is_spam { "[스팸] " } else { "" };
   
    // worker_id 표시 부분
    let worker_info = match worker_id {
        Some(id) => format!("\n처리자: {}", id),
        None => String::new(),
    };
   
    let content = format!(
        "{prefix}📬 메일 알림\n\
         제목: {subject}\n\
         보낸이: {sender}\n\
         분류: {category}{worker_info}",
        prefix = prefix,
        subject = subject,
        sender = sender,
        category = category,
        worker_info = worker_info
    );

    let client = Client::new();
    let res = client
        .post(webhook_url)
        .json(&DiscordPayload { content })
        .send()
        .await?;

    let status = res.status();
    // 응답 본문 읽기 (실패 시 대체 문자열 사용)
    let body_text = res
        .text()
        .await
        .unwrap_or_else(|_| "<body 읽기 실패>".into());

    if status.is_success() {
        info!("[Discord] 전송 성공: {} (status={}) worker_id={:?}", subject, status, worker_id);
    } else {
        error!("[Discord] 전송 실패: status={} body={} worker_id={:?}", status, body_text, worker_id);
    }

    Ok(())
}