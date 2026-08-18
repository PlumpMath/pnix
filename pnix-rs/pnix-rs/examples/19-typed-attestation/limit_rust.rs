//! plain Rust의 한계 — 타입 있는 증명서(predicate-subject 분리, in-toto/SLSA
//! 스타일)가 없다.

fn main() {
    // 빌드 로그는 남지만, "이 주장(predicate)이 이 내용해시(subject)에
    // 대해 성립"이라는 검증 가능한 attestation 포맷이 없다.
    println!("plain Rust: 타입 있는 attestation(predicate-subject) 검증이 없다");
}
