// A Rust program pnix-rs HOLDS (does not support): user macro_rules!.
// rustc accepts it, but rs-meta's front-end holds it (lex boundary: `$`).
// The peer-engine verdict reports status=held, surface=held-macro-rules — so a
// .px control plane routes it to a DIFFERENT engine instead of falsely
// accepting or rejecting it. This is the peer-engine value: capability-aware
// routing, not one engine pretending to do everything.
macro_rules! square {
    ($x:expr) => { $x * $x };
}
fn main() {
    println!("{}", square!(7));
}
