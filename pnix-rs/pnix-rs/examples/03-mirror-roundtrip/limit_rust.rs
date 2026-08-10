//! plain Rust의 한계 — parse -> 재출력이 '의미 보존'인지 스스로 증명하지 않는다.
//!
//! Debug({:?})는 안정성 보증이 없고, '재출력을 다시 파싱해 같은 값인가'를
//! 판정하는 상태 어휘(lossless/held/rejected)나 정본 재출력이 표준으로 없다.

fn main() {
    let src = "let x = 21; in x + x";
    // plain Rust로 이 px를 파싱/재출력/재평가해 '왕복 무손실'을 증명하려면
    // 렉서·파서·프린터·평가기 + 상태 어휘를 전부 직접 만들어야 한다.
    println!("소스: {}", src);
    println!("plain Rust: 정본 재출력·왕복 무손실 판정을 위한 내장 미러가 없다");
    println!("  (Debug 출력은 안정성/의미보존을 보증하지 않는다)");
}
