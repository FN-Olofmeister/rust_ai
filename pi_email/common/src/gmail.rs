// common/src/gmail.rs

use imap::Session;
use mailparse::{parse_mail, MailHeaderMap, ParsedMail};
use native_tls::TlsConnector;
use scraper::Html;
use std::net::TcpStream;

#[derive(Clone)]
pub struct GmailConfig {
    pub email: String,
    pub password: String,
}

pub struct ParsedEmail {
    pub uid: String,
    pub subject: String,
    pub from: String,
    pub body: String,
    pub attachments: Vec<String>,
    pub gmail_link: String,
}

pub async fn connect_to_gmail(
    config: &GmailConfig,
) -> imap::error::Result<Session<native_tls::TlsStream<TcpStream>>> {
    let tls = TlsConnector::builder().build().unwrap();
    let tcp = TcpStream::connect(("imap.gmail.com", 993))?;
    let tls_stream = tls.connect("imap.gmail.com", tcp)?;
    let client = imap::Client::new(tls_stream);
    let session = client
        .login(&config.email, &config.password)
        .map_err(|e| e.0)?;
    Ok(session)
}

/// 본문(text/plain or text/html)만 추출하는 헬퍼
fn extract_plain_body(part: &ParsedMail) -> Option<String> {
    if part.subparts.is_empty() && part.ctype.mimetype.eq_ignore_ascii_case("text/plain") {
        return part.get_body().ok();
    }
    for sub in &part.subparts {
        if let Some(b) = extract_plain_body(sub) {
            return Some(b);
        }
    }
    if part.subparts.is_empty() && part.ctype.mimetype.eq_ignore_ascii_case("text/html") {
        if let Ok(html) = part.get_body() {
            let fragment = Html::parse_fragment(&html);
            return Some(fragment.root_element().text().collect::<Vec<_>>().join(""));
        }
    }
    None
}

pub fn fetch_unseen_emails(
    session: &mut Session<native_tls::TlsStream<TcpStream>>,
) -> imap::error::Result<Vec<ParsedEmail>> {
    session.select("INBOX")?;
    let uids = session.uid_search("UNSEEN")?;
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
   
    // 🔥 수정사항 1: UID를 개별적으로 처리하여 IMAP 명령어 파싱 오류 방지
    for &uid in &uids {
        match fetch_single_email(session, uid) {
            Ok(Some(email)) => out.push(email),
            Ok(None) => {
                eprintln!("Warning: Failed to parse email with UID {}", uid);
            }
            Err(e) => {
                eprintln!("Error fetching email UID {}: {}", uid, e);
                // 개별 오류는 로그만 남기고 계속 진행
                continue;
            }
        }
    }

    Ok(out)
}

// 🔥 수정사항 2: 개별 이메일 처리 함수 분리
pub fn fetch_single_email(
    session: &mut Session<native_tls::TlsStream<TcpStream>>,
    uid: u32,
) -> imap::error::Result<Option<ParsedEmail>> {
    // 🔥 수정사항 3: 더 안전한 FETCH 명령어 사용
    let fetch_result = session.uid_fetch(uid.to_string(), "BODY.PEEK[]");
   
    let fetches = match fetch_result {
        Ok(f) => f,
        Err(e) => {
            eprintln!("FETCH failed for UID {}: {}", uid, e);
            // 🔥 수정사항 4: FETCH 실패 시 대안 명령어 시도
            return try_alternative_fetch(session, uid);
        }
    };

    if let Some(fetch) = fetches.iter().next() {
        if let Some(bytes) = fetch.body() {
            match parse_single_email(uid, bytes) {
                Ok(email) => {
                    // // ✅ 읽음 플래그 설정 (오류가 나도 계속 진행)
                    // let _ = session.uid_store(uid.to_string(), "+FLAGS (\\Seen)")
                    //     .map_err(|e| eprintln!("Warning: Failed to mark as seen UID {}: {}", uid, e));
                    return Ok(Some(email));
                }
                Err(e) => {
                    eprintln!("Email parsing failed for UID {}: {}", uid, e);
                    return Ok(None);
                }
            }
        }
    }
   
    Ok(None)
}

// 🔥 수정사항 5: FETCH 실패 시 대안 방법
fn try_alternative_fetch(
    session: &mut Session<native_tls::TlsStream<TcpStream>>,
    uid: u32,
) -> imap::error::Result<Option<ParsedEmail>> {
    // RFC822 전체 대신 헤더와 본문을 따로 가져오기
    match session.uid_fetch(uid.to_string(), "BODY[HEADER] BODY[TEXT]") {
        Ok(fetches) => {
            if let Some(_fetch) = fetches.iter().next() {
                // 간단한 이메일 정보만 추출
                let email = ParsedEmail {
                    uid: uid.to_string(),
                    subject: "(제목 파싱 실패)".to_string(),
                    from: "(발신자 파싱 실패)".to_string(),
                    body: "(본문 파싱 실패)".to_string(),
                    attachments: Vec::new(),
                    gmail_link: format!("https://mail.google.com/mail/u/0/#search/rfc822msgid:{}", uid),
                };
               
                // 읽음 표시
                let _ = session.uid_store(uid.to_string(), "+FLAGS (\\Seen)");
                return Ok(Some(email));
            }
        }
        Err(e) => {
            eprintln!("Alternative fetch also failed for UID {}: {}", uid, e);
        }
    }
   
    Ok(None)
}

// 🔥 수정사항 6: 이메일 파싱을 별도 함수로 분리하여 에러 핸들링 개선
fn parse_single_email(uid: u32, bytes: &[u8]) -> Result<ParsedEmail, Box<dyn std::error::Error>> {
    let parsed = parse_mail(bytes)?;

    let subject = parsed
        .headers
        .get_first_value("Subject")
        .unwrap_or_else(|| "(제목 없음)".into());
   
    let from = parsed
        .headers
        .get_first_value("From")
        .unwrap_or_else(|| "(보낸 사람 없음)".into());

    let body = extract_plain_body(&parsed)
        .or_else(|| parsed.get_body().ok())
        .unwrap_or_else(|| "(본문 없음)".into());

    let mut attachments = Vec::new();
    collect_attachments(&parsed, &mut attachments);

    let gmail_link = format!(
        "https://mail.google.com/mail/u/0/#search/rfc822msgid:{}",
        uid
    );

    Ok(ParsedEmail {
        uid: uid.to_string(),
        subject,
        from,
        body,
        attachments,
        gmail_link,
    })
}

// 🔥 수정사항 7: 첨부파일 수집 함수 에러 핸들링 추가
fn collect_attachments(part: &ParsedMail, out: &mut Vec<String>) {
    // 1) Content-Disposition 검사
    let headers = part.get_headers();
    if let Some(disp) = headers.get_first_value("Content-Disposition") {
        if disp.to_lowercase().starts_with("attachment") {
            if let Some(fname) = part.get_content_disposition().params.get("filename") {
                out.push(fname.clone());
            }
        }
    }
   
    // 2) Content-Type name 파라미터
    if let Some(name) = part.ctype.params.get("name") {
        out.push(name.clone());
    }
   
    // 3) 재귀 처리
    for sub in &part.subparts {
        collect_attachments(sub, out);
    }
}
