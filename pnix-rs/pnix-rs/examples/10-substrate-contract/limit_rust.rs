//! plain Rust의 한계 — '인터프리터 구현 == 컴파일 결과'를 강제하는 게 없다.
//!
//! translation validation(모든 프로그램에서 interp stdout == rustc stdout)은
//! 표준으로 없다. pnix-rs의 substrate-check는 px 엔진 소스를 rs-meta 인터프리터
//! 와 rustc 와 네이티브 세 방향으로 돌려 그 합치를 게이트한다.

fn main() {
    println!("plain Rust: 두 실행 경로(인터프리터 vs rustc)의 합치를 스스로 증명하지 않는다");
    println!("  (substrate-check는 rs-meta interp == rustc == native 3-way를 게이트)");
}
