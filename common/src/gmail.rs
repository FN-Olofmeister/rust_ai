// common/src/gmail.rs

use imap::Session;
use mailparse::{parse_mail, ParsedMail, MailHeaderMap};  // ← MailHeaderMap 추가
use native_tls::TlsConnector;
use scraper::Html;
use std::net::TcpStream;
use std::io;
use imap::error::Error as ImapError;

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
    let tls_stream = tls
    .connect("imap.gmail.com", tcp)
    .map_err(|e| ImapError::Io(io::Error::new(io::ErrorKind::Other, e)))?;
    let mut client = imap::Client::new(tls_stream);
    let session = client
        .login(&config.email, &config.password)
        .map_err(|e| e.0)?;
    Ok(session)
}

pub fn fetch_unseen_emails(
    session: &mut Session<native_tls::TlsStream<TcpStream>>,
) -> imap::error::Result<Vec<ParsedEmail>> {
    session.select("INBOX")?;
    let uids = session.search("UNSEEN")?;  // mut 제거
    let mut out = Vec::new();

    for uid in uids {
        let fetches = session.fetch(uid.to_string(), "RFC822")?;
        for fetch in fetches.iter() {
            if let Some(bytes) = fetch.body() {
                if let Ok(parsed) = parse_mail(bytes) {
                    // header 추출: get_first_value 는 MailHeaderMap 트레잇에서 제공됩니다
                    let subject = parsed
                        .headers
                        .get_first_value("Subject")
                        .unwrap_or_else(|| "(제목 없음)".into());
                    let from = parsed
                        .headers
                        .get_first_value("From")
                        .unwrap_or_else(|| "(보낸 사람 없음)".into());

                    // 본문 추출
                    fn extract_plain_body(part: &ParsedMail) -> Option<String> {
                        if part.subparts.is_empty()
                            && part.ctype.mimetype.eq_ignore_ascii_case("text/plain")
                        {
                            return part.get_body().ok();
                        }
                        for sub in &part.subparts {
                            if let Some(b) = extract_plain_body(sub) {
                                return Some(b);
                            }
                        }
                        if part.subparts.is_empty()
                            && part.ctype.mimetype.eq_ignore_ascii_case("text/html")
                        {
                            if let Ok(html) = part.get_body() {
                                let fragment = Html::parse_fragment(&html);
                                let text = fragment
                                    .root_element()
                                    .text()
                                    .collect::<Vec<_>>()
                                    .join("");
                                return Some(text);
                            }
                        }
                        None
                    }

                    let body = extract_plain_body(&parsed)
                        .or_else(|| parsed.get_body().ok())
                        .unwrap_or_else(|| "(본문 없음)".into());

                    // attachments 수집
                    let mut attachments = Vec::new();
                    collect_attachments(&parsed, &mut attachments);

                    let gmail_link = format!(
                        "https://mail.google.com/mail/u/0/#search/rfc822msgid:{}",
                        uid
                    );

                    out.push(ParsedEmail {
                        uid: uid.to_string(),
                        subject,
                        from,
                        body,
                        attachments,
                        gmail_link,
                    });
                }
                let _ = session.store(uid.to_string(), "+FLAGS (\\Seen)");
            }
        }
    }

    Ok(out)
}

// --- 여기에 파일 끝부분에 추가하세요 ---
fn collect_attachments(part: &ParsedMail, out: &mut Vec<String>) {
    // 1) Content-Disposition 검사
    if let Some(disp) = part.get_headers().get_first_value("Content-Disposition") {
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
    // 재귀
    for sub in &part.subparts {
        collect_attachments(sub, out);
    }
}
