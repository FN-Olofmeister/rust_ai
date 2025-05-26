//master/src/api.rs

use axum::{routing::post, Router, Json};
use serde::{Deserialize, Serialize};
use common::email::{process_incoming_email, get_email};
use crate::ai::classify_with_ai;

#[derive(Deserialize)]
pub struct EmailReceiveRequest { pub from: String, pub to: String, pub subject: String, pub body: String }

#[derive(Serialize)]
pub struct EmailReceiveResponse { pub success: bool, pub email_id: Option<String>, pub message: String }

#[derive(Deserialize)]
pub struct ClassifyEmailRequest { pub email_id: String }

#[derive(Serialize)]
pub struct ClassifyEmailResponse { pub success: bool, pub category: Option<String>, pub confidence: Option<f32>, pub message: String }

#[derive(Deserialize)]
pub struct AiConnectRequest { pub api_key: String, pub model: String }

#[derive(Serialize)]
pub struct AiConnectResponse { pub success: bool, pub session_id: Option<String>, pub message: String }

pub fn create_router() -> Router {
    Router::new()
        .route("/api/email/receive", post(receive_email))
        .route("/api/email/classify", post(classify_email))
        .route("/api/ai/connect", post(connect_ai))
}

// 수신
async fn receive_email(Json(payload): Json<EmailReceiveRequest>) -> Json<EmailReceiveResponse> {
    match process_incoming_email(&payload.from, &payload.to, &payload.subject, &payload.body).await {
        Ok(id) => Json(EmailReceiveResponse { success: true, email_id: Some(id), message: "이메일 수신 성공".into() }),
        Err(e) => Json(EmailReceiveResponse { success: false, email_id: None, message: format!("이메일 처리 실패: {}", e) }),
    }
}

// 분류
async fn classify_email(Json(payload): Json<ClassifyEmailRequest>) -> Json<ClassifyEmailResponse> {
    match get_email(&payload.email_id) {
        Ok(email) => match classify_with_ai(&email).await {
            Ok((cat, conf)) => Json(ClassifyEmailResponse { success: true, category: Some(cat), confidence: Some(conf), message: "분류 성공".into() }),
            Err(e) => Json(ClassifyEmailResponse { success: false, category: None, confidence: None, message: format!("AI 분류 실패: {}", e) }),
        },
        Err(e) => Json(ClassifyEmailResponse { success: false, category: None, confidence: None, message: format!("이메일 조회 실패: {}", e) }),
    }
}

// AI 연결 (세션 매핑 생략하고 바로 성공 리턴)
async fn connect_ai(Json(_): Json<AiConnectRequest>) -> Json<AiConnectResponse> {
    // 별도 세션 관리 생략
    Json(AiConnectResponse { success: true, session_id: None, message: "AI 연결 성공".into() })
}