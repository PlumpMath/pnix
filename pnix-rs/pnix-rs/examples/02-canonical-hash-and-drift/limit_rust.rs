//! plain Rust의 한계 — 코드 '의미'에 대한 안정적 내용주소 해시가 없다.
//!
//! DefaultHasher는 실행마다 시드가 바뀌어 재현 불가하고, 소스 문자열 해시는
//! 포맷/공백/바인딩 순서에 민감하다. 같은 뜻의 두 프로그램이 다른 해시를 갖는다.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn h(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    // 같은 의미, 다른 형식 -> 다른 해시.
    let a = "let a=1; b=2; in a+b";
    let b = "let b = 2; a = 1; in a + b"; // 같은 뜻, 순서·공백만 다름
    println!("소스 해시 a: {}", h(a));
    println!("소스 해시 b: {}", h(b));
    println!("두 해시가 다르다: {}", h(a) != h(b));
    println!("plain Rust: '의미의 정본' 내용주소가 아니라 '문자열'의 (비재현) 해시일 뿐");
}
