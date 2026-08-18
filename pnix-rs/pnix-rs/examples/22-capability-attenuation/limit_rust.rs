//! plain Rust의 한계 — 하위 컴포넌트에 넘길 effect(capability) 권한을
//! 명시적으로 깎고 비가역성을 보장하는 모델이 없다.

fn main() {
    // 소유권/borrow는 메모리 안전 모델이지, "이 effect 집합만 허용하고
    // 되돌려 재확대할 수 없다"는 capability 감쇠 모델이 아니다.
    println!("plain Rust: capability 감쇠(비가역적 권한 축소) 모델이 없다");
}
