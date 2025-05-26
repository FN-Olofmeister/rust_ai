//master/src/bin/server.rs

use chrono::Local;
use common::gmail::{connect_to_gmail, fetch_unseen_emails, GmailConfig};
use common::classifier::classify_via_openai;
use common::discord::send_discord_alert;
use dotenv::dotenv;
use std::{env, sync::Arc};
use tokio::{sync::Semaphore, task};
use tracing::{error, info};
use tracing_subscriber::fmt::init as tracing_init;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_init();

    let cfg = GmailConfig {
        email: env::var("GMAIL_EMAIL").unwrap(),
        password: env::var("GMAIL_PASSWORD").unwrap(),
    };
    let webhook = env::var("DISCORD_WEBHOOK_URL").unwrap();
    let worker_id: u64 = env::var("WORKER_ID").unwrap().parse().unwrap();
    let total: u64 = env::var("TOTAL_WORKERS").unwrap().parse().unwrap();
    let concurrency: usize = env::var("CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);
    let sem = Arc::new(Semaphore::new(concurrency));

    info!("[Notifier] shard {}/{} 시작 — 동시처리={}", worker_id, total, concurrency);

    loop {
        match connect_to_gmail(&cfg).await {
            Ok(mut session) => {
                if let Ok(emails) = fetch_unseen_emails(&mut session) {
                    for em in emails {
                        // 샤딩: UID 해시 % TOTAL == WORKER_ID
                        let h = fxhash::hash32(em.uid.as_bytes()) as u64 % total;
                        if h != worker_id { continue; }

                        let permit = sem.clone().acquire_owned().await.unwrap();
                        let hook = webhook.clone();
                        let subj = em.subject.clone();
                        let sndr = em.from.clone();
                        let body = em.body.clone();

                        task::spawn(async move {
                            let now = Local::now();
                            info!("[{}] 처리 시작: {}", now.format("%Y-%m-%d %H:%M:%S"), subj);
                            match classify_via_openai(&subj, &body).await {
                                Ok((cat, _)) => {
                                    if let Err(e) = send_discord_alert(&hook, &subj, &sndr, &cat).await {
                                        error!("[Discord] 전송 실패: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("[AI] 분류 실패: {}", e);
                                }
                            }
                            drop(permit);
                        });
                    }
                }
                let _ = session.logout();
            }
            Err(e) => error!("[Gmail] 연결 실패: {}", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
