// Real Rust, inside the rs-meta slice-1 subset.
//   interpreter: bootstrap run        -f samples/factorial.rs
//   rustc tier:  bootstrap native-run -f samples/factorial.rs
//   plain rustc: rustc samples/factorial.rs -o /tmp/f && /tmp/f
fn fact(n: i64) -> i64 {
    if n < 2 {
        1
    } else {
        n * fact(n - 1)
    }
}

fn main() {
    println!("{}", fact(10));
}
