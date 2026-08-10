// The LIMIT of plain rustc.
//
// This program compiles and runs under `rustc` — you get a binary and an
// output. But that is ALL you get. Plain rustc cannot tell you:
//   - whether an INDEPENDENT semantic oracle (a meta-circular interpreter)
//     agrees with rustc on this program's output (translation validation),
//   - a format-invariant content address (ir_hash) for the program,
//   - a reproducible native-artifact receipt tied to the rustc version/flags,
//   - a routable verdict a common control plane can consume.
//
// rustc is a black-box toolchain. pnix-rs treats rs-meta (a META-CIRCULAR Rust
// compiler/evaluator) as a peer engine and emits all of the above as a `.px`
// value. That is why pnix-rs uses rs-meta and not "just a Rust compiler."
fn main() {
    let answer = 6 * 7;
    println!("{}", answer);
}
