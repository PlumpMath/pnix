//! plain Rust의 한계 — Rust와 다른 표현(px) 사이 사영을 substrate로 증명할 수 없다.
//!
//! serde 등으로 값을 직렬화·역직렬화할 수는 있지만, "왕복이 의미를 보존하고
//! 서로 다른 두 실행기(인터프리터 vs 컴파일러)가 같은 결과를 낸다"까지는
//! 증명하지 않는다. pnix-rs의 rust-mirror는 그 3-way 합치를 게이트한다.

fn main() {
    // px 값 { a = 1; b = [2 3]; }를 손으로 Rust로 옮긴 것.
    let a = 1i64;
    let b = vec![2i64, 3i64];
    println!("Rust로 옮긴 값: a={}, b={:?}", a, b);
    println!("plain Rust: 이 이식이 'px 정본과 == interp == rustc'인지 증명하는 수단이 없다");
    println!("  (rust-mirror는 px값->Rust프로그램->3-way, 그리고 Rust->px->Rust 재구성을 게이트)");
}
