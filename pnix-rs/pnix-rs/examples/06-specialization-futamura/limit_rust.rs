//! plain Rust의 한계 — 부분평가 / Futamura 사영(잔여 프로그램 생성)이 없다.
//!
//! const-eval은 컴파일-타임 상수만 접는다. "인터프리터를 특정 프로그램에
//! 특화해 해석 계층이 사라진 새 소스를 생성"하는 부분평가는 표준으로 없다.

fn interp(prog: &str, input: i64) -> i64 {
    // 미니 객체언어 인터프리터 (여기선 한 종류만).
    match prog { "double_plus_input" => input * 2 + 3, _ => 0 }
}

fn main() {
    println!("interp 결과: {}", interp("double_plus_input", 5));
    // plain Rust로는 이 interp를 "prog=double_plus_input"에 특화해
    // `fn(input) -> input*2+3` 같은 잔여 소스를 자동 생성할 수 없다.
    println!("plain Rust: 인터프리터를 프로그램에 특화한 '잔여 코드' 생성 수단이 없다");
}
