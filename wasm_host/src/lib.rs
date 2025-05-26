//! wasm_host – reusable email classifier
//!
//! ─────────────────────────────────────────────────────────
//! • 네이티브( x86_64 · aarch64 등 ) → GPT-4(OpenAI) API 호출
//! • WASM   ( wasm32 )              → 경량 규칙 기반 로직
//! ─────────────────────────────────────────────────────────

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::to_value;

/// JSON 구조체 (JS 쪽으로 그대로 직렬화)
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryResult {
    pub category:   String,
    pub confidence: f32,
}

//
// ───────────── 분기: 아키텍처별 구현 ─────────────
//

#[cfg(not(target_arch = "wasm32"))]          // ── 네이티브 ──
mod imp {
    use super::*;
    use common::classifier::classify_via_openai;

    pub async fn classify(subject: &str, body: &str) -> CategoryResult {
        match classify_via_openai(subject, body).await {
            Ok((cat, conf)) => CategoryResult { category: cat, confidence: conf },
            Err(e) => {
                eprintln!("OpenAI 분류 실패: {e}");
                CategoryResult { category: "ERROR".into(), confidence: 0.0 }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]               // ── WASM ──
mod imp {
    use super::*;

    pub async fn classify(subject: &str, body: &str) -> CategoryResult {
        let txt = format!("{} {}", subject.to_lowercase(), body.to_lowercase());

        if txt.contains("urgent") || txt.contains("asap") {
            CategoryResult { category: "긴급".into(), confidence: 0.9 }
        } else if txt.contains("discount") || txt.contains("promo") {
            CategoryResult { category: "홍보".into(), confidence: 0.8 }
        } else {
            CategoryResult { category: "일반".into(), confidence: 0.5 }
        }
    }
}

//
// ───────────── JS로 노출되는 API ─────────────
//

#[wasm_bindgen]
pub async fn classify(subject: String, body: String) -> JsValue {
    // imp::classify() 는 타깃 아키텍처에 맞게 자동 선택됨
    let res = imp::classify(&subject, &body).await;
    to_value(&res).expect("JSON 직렬화 실패")
}

/// CLI·테스트용 순수 Rust API
pub async fn classify_native(subject: &str, body: &str) -> CategoryResult {
    imp::classify(subject, body).await
}