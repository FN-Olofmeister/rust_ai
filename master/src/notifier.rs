// master/src/notifier.rs

use chrono::Local;
use common::gmail::{connect_to_gmail, fetch_unseen_emails, GmailConfig};
use common::classifier::classify_via_openai;
use common::discord::send_discord_alert;
use dotenv::dotenv;
use fxhash::FxHasher;
use std::{env, hash::Hasher, sync::Arc, time::Duration};  // Duration 추가
use tokio::{sync::Semaphore, task};
use tracing::{error, info, debug};                         // debug 매크로 import
use tracing_subscriber;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    let cfg = GmailConfig {
        email: env::var("GMAIL_EMAIL").expect("GMAIL_EMAIL 필요"),
        password: env::var("GMAIL_PASSWORD").expect("GMAIL_PASSWORD 필요"),
    };
    let webhook = env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL 필요");

    // WORKER_ID와 TOTAL_WORKERS를 명시적으로 파싱
    let worker_id: u64 = env::var("WORKER_ID")
        .expect("WORKER_ID 필요")
        .parse()
        .expect("WORKER_ID는 정수여야 합니다");
    let total: u64 = env::var("TOTAL_WORKERS")
        .expect("TOTAL_WORKERS 필요")
        .parse()
        .expect("TOTAL_WORKERS는 정수여야 합니다");
    let concurrency: usize = env::var("CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);
    let sem = Arc::new(Semaphore::new(concurrency));

    info!("[Notifier] shard {}/{} 시작 — 동시처리={}", worker_id, total, concurrency);

    loop {
        info!("[Notifier] [{}] UNSEEN 메일 확인 중…", Local::now().format("%Y-%m-%d %H:%M:%S"));

        // Gmail 연결 및 메일 fetch
        let mails = match connect_to_gmail(&cfg).await.and_then(|mut s| fetch_unseen_emails(&mut s)) {
            Ok(v) => v,
            Err(e) => {
                error!("[Gmail] 연결/조회 실패: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        info!("[Notifier] 발견된 메일 수: {}", mails.len());

        for em in mails {
            // 샤딩 로직
            let mut hasher = FxHasher::default();
            hasher.write(em.uid.as_bytes());
            let h = (hasher.finish() % total) as u64;

            info!("[Shard] uid={} → hash%{}={} (내 id={})", em.uid, total, h, worker_id);
            if h != worker_id {
                debug!("[Shard] 스킵됨 uid={}", em.uid);
                continue;
            }

            let permit = sem.clone().acquire_owned().await.unwrap();
            let hook = webhook.clone();
            let subj = em.subject.clone();
            let sndr = em.from.clone();
            let body = em.body.clone();

            task::spawn(async move {
                info!("[{}] 분류 시작: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), subj);
                match classify_via_openai(&subj, &body).await {
                    Ok((cat, score)) => {
                        info!("[AI] 분류 완료: {} ({})", cat, score);
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

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
