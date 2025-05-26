// common/src/email.rs

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};

lazy_static::lazy_static! {
    static ref EMAIL_STORE: Arc<Mutex<HashMap<String, Email>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub received_at: DateTime<Utc>,
    pub category: Option<String>,
    pub ai_processed: bool,
}

pub async fn process_incoming_email(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<String> {
    let email_id = Uuid::new_v4().to_string();
    let email = Email {
        id: email_id.clone(),
        from: from.to_string(),
        to: to.to_string(),
        subject: subject.to_string(),
        body: body.to_string(),
        received_at: Utc::now(),
        category: None,
        ai_processed: false,
    };
    {
        let mut store = EMAIL_STORE
            .lock()
            .map_err(|_| anyhow!("이메일 저장소 잠금 실패"))?;
        store.insert(email_id.clone(), email);
    }
    Ok(email_id)
}

pub fn get_email(email_id: &str) -> Result<Email> {
    let store = EMAIL_STORE
        .lock()
        .map_err(|_| anyhow!("이메일 저장소 잠금 실패"))?;
    store
        .get(email_id)
        .cloned()
        .ok_or_else(|| anyhow!("이메일을 찾을 수 없음: {}", email_id))
}
