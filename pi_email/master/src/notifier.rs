// master/src/notifier.rs
use chrono::Local;
use common::classifier::classify_via_openai;
use common::discord::send_discord_alert;
use common::gmail::{connect_to_gmail, fetch_unseen_emails, GmailConfig};
use dotenv::dotenv;
use fxhash::FxHasher;
use std::{env, hash::Hasher, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task};
use tracing::{debug, error, info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    // 1) 환경 변수 & 로거 초기화
    dotenv().ok();
    // 단일 빌더 사용: 로그 최대 레벨 DEBUG
    fmt().with_max_level(Level::DEBUG).init();

    // 2) 설정값 읽기
    let cfg = GmailConfig {
        email: env::var("GMAIL_EMAIL").expect("GMAIL_EMAIL 필요"),
        password: env::var("GMAIL_PASSWORD").expect("GMAIL_PASSWORD 필요"),
    };
    let webhook = env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL 필요");
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
        info!(
            "[Notifier] [{}] UNSEEN 메일 확인 중…",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        // 3) Gmail 연결 및 INBOX 선택
        let mut session = match connect_to_gmail(&cfg).await {
            Ok(sess) => sess,
            Err(e) => {
                error!("[Gmail] 연결 실패: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        if let Err(e) = session.select("INBOX") {
            error!("[Gmail] INBOX 선택 실패: {}", e);
            let _ = session.logout();
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }

        // 4) UNSEEN 이메일 조회
        let mails = match fetch_unseen_emails(&mut session) {
            Ok(v) => v,
            Err(e) => {
                error!("[Gmail] 메일 조회 실패: {}", e);
                let _ = session.logout();
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        info!("[Notifier] 발견된 메일 수: {}", mails.len());

        // 5) 샤딩 & 처리
        for em in mails {
            let mut hasher = FxHasher::default();
            hasher.write(em.uid.as_bytes());
            let shard = hasher.finish() % total;

            info!(
                "[DBG] UID={} -> hash={} -> shard={} (worker_id={})",
                em.uid,
                hasher.finish(),
                shard,
                worker_id
            );
            if shard != worker_id {
                debug!("[Shard:{}] 스킵 uid={}", worker_id, em.uid);
                continue;
            }

            // (A) 즉시 읽음 처리
            match session.uid_store(&em.uid, "+FLAGS (\\Seen)") {
                Ok(_) => {
                    if let Err(e) = session.expunge() {
                        error!("[Gmail] expunge 실패: {}", e);
                    }
                    info!("[Shard:{}] 읽음 처리 -> {}", worker_id, em.uid);
                }
                Err(e) => error!("[Gmail] UID {} 읽음 처리 실패: {}", em.uid, e),
            }

            // (B) 비동기로 분류
            let permit = sem.clone().acquire_owned().await.unwrap();
            let hook = webhook.clone();
            let subj = em.subject.clone();
            let sndr = em.from.clone();
            let body = em.body.clone();
            let wid = worker_id;
            let uid_clone = em.uid.clone();
            task::spawn(async move {
                info!("[Shard:{}] 분류 시작 -> {}", wid, uid_clone);
                if let Ok((cat, _)) = classify_via_openai(&subj, &body).await {
                    if let Err(e) = send_discord_alert(&hook, &subj, &sndr, &cat, Some(&wid.to_string())).await {
                        error!("[Discord] 전송 실패: {}", e);
                    }
                } else {
                    error!("[AI] 분류 실패: {}", uid_clone);
                }
                drop(permit);
            });
        }

        // 6) 세션 종료 & 대기
        let _ = session.logout();
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
