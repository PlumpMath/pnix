//! plain Rust의 한계 — binding-time analysis(무엇이 static/dynamic인가)가 없다.

fn main() {
    // "이 식의 어느 부분이 컴파일-타임에 접히고 어느 부분이 런타임인가"를
    // 예측하고, 그 예측을 실제 특화 거동과 대조하는 표준 분석기가 없다.
    println!("plain Rust: binding-time analysis(static/dynamic 예측+교차검증)가 없다");
}
