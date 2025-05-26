//! WASI CLI – 규칙 기반 분류기

use wasm_host::classify_native;          // ← 패키지(lib) 이름으로 변경
use futures_lite::future::block_on;
use serde_json::to_string_pretty;

fn main() {
    let mut args = std::env::args().skip(1);
    let subject = args.next().unwrap_or_default();
    let body    = args.next().unwrap_or_default();

    let res = block_on( classify_native(&subject, &body) );
    println!("{}", to_string_pretty(&res).unwrap());
}
