use anyhow::{Result, anyhow};
use std::io::{self, Write, stdout};
use tokio::sync::mpsc;
use super::api::ApiClient;

// UI 상태 열거형
enum UiState {
    MainMenu,
    ConnectAi,
    QueryAi,
    Exit,
}

// UI 시작 함수
pub async fn start_ui() -> Result<()> {
    // API 클라이언트 생성
    let api_client = ApiClient::new("http://127.0.0.1:7860");
    
    // 서버 연결 확인
    match api_client.health_check().await {
        Ok(message) => println!("서버 연결 성공: {}", message),
        Err(e) => {
            println!("서버 연결 실패: {}. 서버가 실행 중인지 확인하세요.", e);
            return Err(anyhow!("서버 연결 실패"));
        }
    }
    
    // UI 상태 및 세션 정보
    let mut state = UiState::MainMenu;
    let mut ai_session_id = String::new();
    
    // UI 루프
    loop {
        match state {
            UiState::MainMenu => {
                println!("\n======== 이메일 분류기 클라이언트 ========");
                println!("1. AI 서비스 연결");
                if !ai_session_id.is_empty() {
                    println!("2. AI에 쿼리 보내기");
                }
                println!("0. 종료");
                print!("선택: ");
                io::stdout().flush().unwrap();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                match input.trim() {
                    "1" => state = UiState::ConnectAi,
                    "2" if !ai_session_id.is_empty() => state = UiState::QueryAi,
                    "0" => state = UiState::Exit,
                    _ => println!("잘못된 선택입니다. 다시 시도하세요."),
                }
            },
            UiState::ConnectAi => {
                println!("\n======== AI 서비스 연결 ========");
                
                print!("API 키 입력: ");
                io::stdout().flush().unwrap();
                let mut api_key = String::new();
                io::stdin().read_line(&mut api_key)?;
                
                print!("모델 이름 입력 (기본값: gpt-4): ");
                io::stdout().flush().unwrap();
                let mut model = String::new();
                io::stdin().read_line(&mut model)?;
                let model = if model.trim().is_empty() {
                    "gpt-4".to_string()
                } else {
                    model.trim().to_string()
                };
                
                println!("AI 서비스에 연결 중...");
                match api_client.connect_ai(api_key.trim(), &model).await {
                    Ok(session_id) => {
                        ai_session_id = session_id;
                        println!("AI 서비스에 성공적으로 연결되었습니다.");
                        state = UiState::MainMenu;
                    },
                    Err(e) => {
                        println!("AI 서비스 연결 실패: {}", e);
                        state = UiState::MainMenu;
                    }
                }
            },
            UiState::QueryAi => {
                println!("\n======== AI 쿼리 ========");
                
                print!("이메일 ID (선택사항): ");
                io::stdout().flush().unwrap();
                let mut email_id = String::new();
                io::stdin().read_line(&mut email_id)?;
                let email_id = if email_id.trim().is_empty() {
                    None
                } else {
                    Some(email_id.trim())
                };
                
                println!("프롬프트 입력 (빈 줄로 종료):");
                let mut prompt = String::new();
                loop {
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    if line.trim().is_empty() {
                        break;
                    }
                    prompt.push_str(&line);
                }
                
                println!("AI에 쿼리 전송 중...");
                match api_client.query_ai(&ai_session_id, &prompt, email_id).await {
                    Ok(response) => {
                        println!("\nAI 응답:");
                        println!("{}", response);
                        state = UiState::MainMenu;
                    },
                    Err(e) => {
                        println!("AI 쿼리 실패: {}", e);
                        state = UiState::MainMenu;
                    }
                }
            },
            UiState::Exit => {
                println!("프로그램을 종료합니다.");
                break;
            }
        }
    }
    
    Ok(())
}