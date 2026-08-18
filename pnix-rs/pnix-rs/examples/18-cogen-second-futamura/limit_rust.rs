//! plain Rust의 한계 — 인터프리터를 접어 컴파일러 자체를 생성하는
//! 2차 Futamura 사영이 없다.

fn main() {
    // rustc는 Rust의 컴파일러일 뿐, 스스로를 특화해 다른 언어용
    // 컴파일러를 만들어내는 표준 메커니즘이 없다.
    println!("plain Rust: 2차 Futamura 사영(컴파일러 생성기)이 없다");
}
