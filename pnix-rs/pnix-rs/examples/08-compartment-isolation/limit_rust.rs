//! plain Rust의 한계 — 런타임 권한-감쇠 격리 환경을 동적으로 만들 수단이 부족하다.

fn main() {
    // 모듈/가시성은 컴파일-타임 캡슐화다. 런타임에 "이 코드에는 이 권한만"으로
    // 격리한 실행 compartment를 동적으로 구성하는 표준 수단이 아니다.
    println!("plain Rust: 컴파일-타임 캡슐화는 있으나, 런타임 capability 격리 compartment는 없다");
}
