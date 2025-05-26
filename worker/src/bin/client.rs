//worker/src/bin/client.rs

use dotenv::dotenv;
use worker::api::ApiClient;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let base = std::env::var("MASTER_API_URL").unwrap_or("http://127.0.0.1:7860".into());
    let client = ApiClient::new(&base);
    println!("📬 이메일 자동 분류 루프를 시작합니다...");
    loop {
        // 샤딩 또는 큐 연동하여 email_id 가져오기
        let email_id = "bjh5438@naver.com"; 
        if let Ok((cat, conf)) = client.classify_email(email_id).await {
            println!("분류 완료: {} ({})", cat, conf);
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
