//! plain Rust의 한계 — 여러 실행 경로(직접/정규화/CAS/AST왕복/replay)가
//! 코퍼스 규모로 같은 값을 내는지 게이트하지 않는다.

fn main() {
    println!("plain Rust: 5-경로 스테이지 사다리 값 일치 게이트가 없다");
}
