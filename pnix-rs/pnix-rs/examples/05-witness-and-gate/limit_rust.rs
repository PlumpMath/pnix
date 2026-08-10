//! plain Rust의 한계 — 실행에 기계가독 증거(witness)/권한(capability)이 없다.

fn main() {
    let x = 6 * 7;
    println!("결과: {}", x);
    // 이 결과가 "어떤 입력 해시에서 / 어떤 효과로 / 같은 환경에서" 나왔는지의
    // 기계가독 증거(witness)를 plain Rust는 자동으로 남기지 않는다.
    println!("plain Rust: 결과값만 있고, 무엇을 했는지의 witness/capability 기록이 없다");
}
