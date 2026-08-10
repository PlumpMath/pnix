//! plain Rust의 한계 — '이 계산을 승인해도 되는가'의 종합 verdict가 없다.

fn main() {
    let value = 42;
    // 값은 나왔지만, "순수한가 / 의미보존 왕복인가 / 증거 해시는 / 허용인가"를
    // 하나의 판정으로 묶는 표준 체크포인트가 plain Rust엔 없다.
    println!("값: {}", value);
    println!("plain Rust: gate+mirror+ir+witness를 묶은 단일 action verdict가 없다");
}
