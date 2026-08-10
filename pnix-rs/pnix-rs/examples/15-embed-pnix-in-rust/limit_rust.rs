//! plain Rust의 한계 — 호스트 Rust 소스에 "다른 언어(pnix)"를 임베드해도, 그건
//! read-time에 폼으로 승격되지 않고 그냥 죽은 문자열(&str)일 뿐이다.
//!
//! Rust에는 사용자정의 reader macro가 없고(Lisp의 `#px`류), `macro_rules!`도
//! 인자를 토큰/문자열로만 받는다 → 임베드된 pnix는:
//!   - 컴파일 시점에 문법/타입이 검사되지 않는다(오타가 그대로 통과),
//!   - 실행 시점에 평가할 수단이 없다(std에 pnix 인터프리터가 없다),
//!   - Rust로의 사영이 "의미를 보존"하는지 증명할 방법이 없다.
//! → 한 언어(Rust)만으로는 "임베드된 pnix에 의미를 주고 검증"이 불가능.

fn main() {
    // 호스트 Rust 소스 안에 pnix 설정 스니펫을 임베드(= polyglot 시도).
    const PNIX_CFG: &str = "let base = 6; in base * 7";

    // Rust가 할 수 있는 것: 문자열로 출력하는 것뿐.
    println!("임베드된 pnix(문자열): {:?}", PNIX_CFG);

    // 할 수 없는 것들(전부 컴파일은 되지만 의미가 없음):
    //  - PNIX_CFG를 평가해 42를 얻기: std에 pnix eval이 없음.
    //  - 아래처럼 오타가 있어도 Rust는 통과시킨다(문자열이니까):
    const PNIX_TYPO: &str = "let base = 6; in base * ";  // pnix로는 문법 오류
    println!("오타 pnix도 Rust는 통과: {:?}", PNIX_TYPO);

    println!("plain Rust: 임베드된 pnix는 죽은 텍스트 — read-time 승격도, 평가도, 의미보존 사영 증명도 없다.");
    println!("  (pnix-rs 방식: 같은 스니펫을 평가(->42)하고 Rust로 사영해 interp==rustc==native로 게이트한다)");
}
