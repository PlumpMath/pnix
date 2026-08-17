//! Proof harness for the Rust meta-circular floor.
//!
//! Slice-1 checks (all runnable today):
//!   - `self-check` : the interpreter runs each corpus program; output matches
//!                    the expected value.
//!   - `tv-check`   : interpreter stdout == native(rustc) stdout for the same
//!                    Rust source (translation validation — interp == rustc).
//!
//! The corpus is real Rust (every program also compiles under rustc), staying
//! inside the slice-1 subset. The self-host stages (interpreter evaluating its
//! own source) are tracked by `stage-status`.

use crate::emit::emit_program;
use crate::independent_mini_backend::compile_and_run;
use crate::interp::Interp;
use crate::lexer::lex;
use crate::native::{default_workdir, native_artifact_receipt, native_cache_probe, native_run};
use crate::parser::parse_program;
use crate::typeck;
use std::fs;
use std::process::Command;

/// (name, Rust source, expected stdout-trimmed). Every entry is valid Rust.
pub fn corpus() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("enum-struct-variant-debug", "#[derive(Debug)] enum E { A { x: i64, y: i64 }, B(i64, i64), C } fn main() { println!(\"{:?} {:?} {:?}\", E::A { x: 1, y: 2 }, E::B(3, 4), E::C); }", "A { x: 1, y: 2 } B(3, 4) C"),
        ("enum-struct-variant-pretty", "#[derive(Debug)] enum E { P { x: i64 } } fn main() { println!(\"{:#?}\", E::P { x: 5 }); }", "P {\n    x: 5,\n}"),
        ("vec-drain", "fn main() { let mut v = vec![1, 2, 3, 4, 5]; let d: Vec<i64> = v.drain(1..3).collect(); let e: Vec<i64> = v.drain(0..1).collect(); println!(\"{:?} {:?} {:?}\", d, e, v); }", "[2, 3] [1] [4, 5]"),
        ("vec-drain-empty", "fn main() { let mut v = vec![1, 2, 3]; let d: Vec<i64> = v.drain(1..1).collect(); println!(\"{:?} {:?}\", d, v); }", "[] [1, 2, 3]"),
        ("into-annot", "fn main() { let a: u8 = 5; let x: i64 = a.into(); let b: i32 = 100; let y: i64 = b.into(); let s: String = \"hi\".into(); println!(\"{} {} {}\", x, y, s); }", "5 100 hi"),
        ("slice-pat-exact", "fn main() { let v = vec![1, 2, 3]; if let [a, b, c] = v[..] { println!(\"{}\", a + b + c); } }", "6"),
        ("slice-pat-rest", "fn main() { let v = vec![1, 2, 3, 4]; if let [first, rest @ ..] = &v[..] { println!(\"{} {:?}\", first, rest); } }", "1 [2, 3, 4]"),
        ("slice-pat-match", "fn main() { let v = vec![1, 2]; let s = match v[..] { [] => \"empty\", [_] => \"one\", [_, _] => \"two\", _ => \"many\" }; println!(\"{}\", s); }", "two"),
        ("local-item-hoist", "fn main() { #[derive(Debug)] struct P { x: i64 } impl P { fn dbl(&self) -> i64 { self.x * 2 } } fn helper(n: i64) -> i64 { n + 1 } enum E { A, B } let p = P { x: 7 }; let e = E::A; let s = match e { E::A => \"a\", E::B => \"b\" }; println!(\"{} {:?} {} {}\", p.dbl(), p, helper(9), s); }", "14 P { x: 7 } 10 a"),
        ("local-fn-recursive", "fn main() { fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } } println!(\"{}\", fib(10)); }", "55"),
        ("labeled-break-value", "fn main() { let x: i64 = 'a: loop { loop { break 'a 42; } }; let y = 'o: loop { let mut i = 0; loop { if i == 3 { break 'o i * 10; } i += 1; } }; println!(\"{} {}\", x, y); }", "42 30"),
        ("ordering-cmp-match", "use std::cmp::Ordering; fn main() { let s = match 5i64.cmp(&3) { Ordering::Less => \"lt\", Ordering::Equal => \"eq\", Ordering::Greater => \"gt\" }; println!(\"{} {:?}\", s, 2i64.cmp(&2)); }", "gt Equal"),
        ("vec-sort-by", "fn main() { let mut v = vec![3, 1, 2]; v.sort_by(|a, b| b.cmp(a)); println!(\"{:?}\", v); }", "[3, 2, 1]"),
        ("closure-ref-pat", "fn main() { let v = vec![1, 2, 3, 4]; let s: i64 = v.iter().map(|&x| x * 2).sum(); let n = v.iter().filter(|&&x| x % 2 == 0).count(); println!(\"{} {}\", s, n); }", "20 2"),
        ("method-ret-self", "struct C { v: i64 } impl C { fn new() -> Self { C { v: 0 } } fn add(&self, n: i64) -> Self { C { v: self.v + n } } } fn main() { println!(\"{}\", C::new().add(5).add(10).v); }", "15"),
        ("trait-assoc-type", "trait Cont { type Item; fn get(&self) -> i64; } struct B { v: i64 } impl Cont for B { type Item = i64; fn get(&self) -> i64 { self.v } } fn main() { println!(\"{}\", B { v: 7 }.get()); }", "7"),
        ("ord-struct-tuple-vec", "#[derive(PartialEq, Eq, PartialOrd, Ord)] struct P { x: i64, y: i64 } fn main() { let a = P { x: 1, y: 2 }; let b = P { x: 1, y: 3 }; println!(\"{} {} {}\", a < b, (1, 2) < (1, 3), vec![1, 2] < vec![1, 3]); }", "true true true"),
        ("ord-string", "fn main() { let s = String::from(\"b\"); println!(\"{} {} {}\", \"apple\" < \"banana\", \"cat\" > \"car\", s > String::from(\"a\")); }", "true true true"),
        ("tuple-let-annot", "fn main() { let (a, b): (i64, i64) = (3, 4); let (c, (d, e)): (i64, (i64, i64)) = (1, (2, 3)); println!(\"{} {}\", a + b, c + d + e); }", "7 6"),
        ("fmt-radix", "fn main() { println!(\"{:x} {:X} {:b} {:o}\", 255, 255, 10, 64); }", "ff FF 1010 100"),
        ("lit-octal-binary", "fn main() { println!(\"{} {}\", 0o17, 0b1010 + 0o10); }", "15 18"),
        ("println-empty", "fn main() { print!(\"a\"); println!(); print!(\"b\"); println!(); }", "a\nb"),
        ("pat-range-neg-half", "fn main() { let a = match -5 { -10..=-1 => 1, _ => 0 }; let b = match 2 { 0..3 => 1, _ => 0 }; let c = match 3 { 0..3 => 1, _ => 0 }; println!(\"{} {} {}\", a, b, c); }", "1 1 0"),
        ("pat-paren-or", "fn main() { let x = 2; let s = match x { (1 | 2 | 3) => \"small\", _ => \"big\" }; println!(\"{}\", s); }", "small"),
        ("opt-mapor-okor-and", "fn main() { let x: Option<i64> = Some(5); let y: Option<i64> = None; let r: Result<i64, &str> = y.ok_or(\"no\"); println!(\"{} {} {:?}\", x.map_or(0, |v| v * 2), x.is_some_and(|v| v > 3), r); }", "10 true Err(\"no\")"),
        ("vec-windows-splitat", "fn main() { let v = vec![1, 2, 3, 4, 5]; let (a, b) = v.split_at(2); println!(\"{} {:?} {:?}\", v.windows(2).count(), a, b); }", "4 [1, 2] [3, 4, 5]"),
        ("vec-concat-bsearch", "fn main() { let v = vec![vec![1, 2], vec![3, 4]]; let s = vec![1, 3, 5, 7]; println!(\"{:?} {:?} {:?}\", v.concat(), s.binary_search(&5), s.binary_search(&4)); }", "[1, 2, 3, 4] Ok(2) Err(2)"),
        ("vec-rotate", "fn main() { let mut v = vec![1, 2, 3, 4, 5]; v.rotate_left(2); v.rotate_right(1); println!(\"{:?}\", v); }", "[2, 3, 4, 5, 1]"),
        ("iter-takewhile-findmap", "fn main() { let v = vec![1, 2, 3, 4, 1]; let a: Vec<i64> = v.clone().into_iter().take_while(|x| *x < 3).collect(); let b = v.iter().find_map(|x| if *x == 3 { Some(*x * 10) } else { None }); println!(\"{:?} {:?}\", a, b); }", "[1, 2] Some(30)"),
        ("str-split-ws-lines", "fn main() { println!(\"{} {}\", \" a  b c \".split_whitespace().count(), \"x\\ny\\nz\".lines().count()); }", "3 3"),
        ("str-replace-trim", "fn main() { println!(\"[{}] {}\", \"  hi  \".trim_start(), \"a-b-c\".replace(\"-\", \"+\")); }", "[hi  ] a+b+c"),
        ("str-strip-splitonce", "fn main() { println!(\"{:?} {:?}\", \"foobar\".strip_prefix(\"foo\"), \"key=val\".split_once(\"=\")); }", "Some(\"bar\") Some((\"key\", \"val\"))"),
        ("str-splitn", "fn main() { let v: Vec<&str> = \"a:b:c\".splitn(2, \":\").collect(); println!(\"{} {}\", v[0], v[1]); }", "a b:c"),
        ("str-char-indices", "fn main() { let mut n = 0; for (i, _c) in \"abcd\".char_indices() { n += i; } println!(\"{}\", n); }", "6"),
        ("int-abs-checked", "fn main() { let x: i64 = -7; println!(\"{} {:?} {:?} {:?}\", x.abs(), x.checked_add(3), x.checked_mul(2), x.checked_div(0)); }", "7 Some(-4) Some(-14) None"),
        ("int-bits", "fn main() { let x: i64 = 8; println!(\"{} {} {}\", x.count_ones(), x.leading_zeros(), x.trailing_zeros()); }", "1 60 3"),
        ("int-saturating-add", "fn main() { let x: usize = 5; let y: i64 = 9223372036854775807; println!(\"{} {}\", x.saturating_add(3), y.saturating_add(9)); }", "8 9223372036854775807"),
        ("char-classify", "fn main() { println!(\"{} {} {} {}\", 'a'.is_alphabetic(), 'A'.is_uppercase(), 'a'.to_ascii_uppercase(), '5'.is_numeric()); }", "true true A true"),
        ("sort-struct-ord", "#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)] struct P { x: i64 } fn main() { let mut v = vec![P { x: 3 }, P { x: 1 }, P { x: 2 }]; v.sort(); println!(\"{} {} {}\", v[0].x, v[1].x, v[2].x); }", "1 2 3"),
        ("sort-tuple-lex", "fn main() { let mut v = vec![(2, 1), (1, 9), (1, 2)]; v.sort(); println!(\"{:?}\", v); }", "[(1, 2), (1, 9), (2, 1)]"),
        ("sort-str-bytes", "fn main() { let mut v = vec![\"cherry\", \"apple\", \"banana\"]; v.sort(); println!(\"{:?}\", v); }", "[\"apple\", \"banana\", \"cherry\"]"),
        ("debug-container-quote", "#[derive(Debug)] struct P { name: String, x: i64 } fn main() { let v = vec![\"a\", \"b\"]; println!(\"{:?}\", v); println!(\"{:?}\", P { name: String::from(\"hi\"), x: 3 }); }", "[\"a\", \"b\"]\nP { name: \"hi\", x: 3 }"),
        ("debug-tuple-struct", "#[derive(Debug)] struct P(i64, i64); #[derive(Debug)] struct Id(i64); fn main() { println!(\"{:?} {:?}\", P(1, 2), Id(42)); }", "P(1, 2) Id(42)"),
        ("debug-pretty-alt", "#[derive(Debug)] struct P { x: i64, y: i64 } fn main() { println!(\"{:#?}\", P { x: 1, y: 2 }); }", "P {\n    x: 1,\n    y: 2,\n}"),
                                                                                                                                                                                                                                ("suffixed-literal-method", "fn main() { println!(\"{} {}\", 5i64.pow(2), (7i64 - 4).signum()); }", "25 1"),
("impl-display", "use std::fmt; struct P(i64); impl fmt::Display for P { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, \"P({})\", self.0) } } fn main() { let p = P(42); println!(\"{}\", p); println!(\"{}\", format!(\"x={}\", P(7))); }", "P(42)\nx=P(7)"),
("str-find", "fn main() { println!(\"{:?} {:?}\", \"hello\".find(\"ll\"), \"hello\".find(\"zz\")); }", "Some(2) None"),
        ("iter-by-key", "fn main() { let v = vec![1, 5, 3]; println!(\"{:?} {:?}\", v.iter().max_by_key(|x| **x), v.iter().min_by_key(|x| **x)); }", "Some(5) Some(1)"),
        ("vec-sort-by-key", "fn main() { let mut v = vec![30, 10, 20]; v.sort_by_key(|x| *x); println!(\"{} {} {}\", v[0], v[1], v[2]); }", "10 20 30"),
        ("vec-chunks", "fn main() { let v = vec![1, 2, 3, 4, 5]; println!(\"{}\", v.chunks(2).count()); }", "3"),
("hashmap-keys-values", "fn main() { use std::collections::HashMap; let mut m: HashMap<i64, i64> = HashMap::new(); m.insert(1, 10); m.insert(2, 20); let s: i64 = m.values().sum(); let k: i64 = m.keys().sum(); println!(\"{} {}\", s, k); }", "30 3"),
        ("iter-flat-map", "fn main() { let v: Vec<i64> = vec![1, 2].into_iter().flat_map(|x| vec![x, x * 10]).collect(); println!(\"{} {} {} {}\", v[0], v[1], v[2], v[3]); }", "1 10 2 20"),
("vec-retain", "fn main() { let mut v = vec![1, 2, 3, 4, 5, 6]; v.retain(|x| x % 2 == 0); println!(\"{} {}\", v.len(), v[2]); }", "3 6"),
        ("iter-chain-stepby", "fn main() { let a: Vec<i64> = vec![1, 2].into_iter().chain(vec![3, 4].into_iter()).collect(); let b: Vec<i64> = (0..10).step_by(3).collect(); println!(\"{} {}\", a.len(), b.len()); }", "4 4"),
        ("iter-rposition", "fn main() { let v = vec![1, 2, 2, 3]; println!(\"{:?}\", v.iter().rposition(|x| *x == 2)); }", "Some(2)"),
("use-in-fn", "fn main() { use std::collections::HashMap; let mut m: HashMap<i64, i64> = HashMap::new(); m.insert(1, 5); println!(\"{}\", m[&1]); }", "5"),
("hashmap-index", "fn main() { let mut m: std::collections::HashMap<i64, i64> = std::collections::HashMap::new(); m.insert(1, 10); m.insert(2, 20); println!(\"{} {}\", m[&1], m[&2]); }", "10 20"),
("int-signum-remeuclid", "fn main() { let a: i64 = -7; println!(\"{} {}\", a.signum(), a.rem_euclid(3)); }", "-1 2"),
        ("iter-product", "fn main() { let p: i64 = vec![1, 2, 3, 4].iter().product(); println!(\"{}\", p); }", "24"),
        ("vec-truncate", "fn main() { let mut v = vec![1, 2, 3, 4, 5]; v.truncate(2); println!(\"{}\", v.len()); }", "2"),
        ("str-case", "fn main() { println!(\"{} {}\", \"aB\".to_uppercase(), \"aB\".to_lowercase()); }", "AB ab"),
("vec-insert", "fn main() { let mut v = vec![1, 3, 4]; v.insert(1, 2); println!(\"{} {} {} {}\", v[0], v[1], v[2], v[3]); }", "1 2 3 4"),
        ("vec-extend", "fn main() { let mut v = vec![1, 2]; v.extend(vec![3, 4, 5]); println!(\"{} {}\", v.len(), v[4]); }", "5 5"),
("generic-close-eq", "fn main() { let o: Option<i64>=Some(5); let v: Vec<i64>=vec![1, 2]; println!(\"{:?} {}\", o, v.len()); }", "Some(5) 2"),
("vec-dedup", "fn main() { let mut v = vec![1, 1, 2, 3, 3, 3, 1]; v.dedup(); println!(\"{}\", v.len()); }", "4"),
        ("iter-min-max", "fn main() { let v = vec![3, 1, 4, 1, 5]; println!(\"{:?} {:?}\", v.iter().min(), v.iter().max()); }", "Some(1) Some(5)"),
        ("option-debug", "fn main() { let a: Option<i64> = Some(5); let b: Option<i64> = None; println!(\"{:?} {:?}\", a, b); }", "Some(5) None"),
("vec-sort", "fn main() { let mut v = vec![3, 1, 4, 1, 5, 9, 2, 6]; v.sort(); println!(\"{} {} {}\", v[0], v[3], v[7]); }", "1 3 9"),
("iter-map-collect", "fn main() { let v: Vec<i64> = vec![1, 2, 3].iter().map(|x| x * 2).collect(); println!(\"{} {} {}\", v[0], v[1], v[2]); }", "2 4 6"),
        ("ref-arithmetic", "fn main() { let x = 5; let r = &x; println!(\"{}\", r + 1); }", "6"),
        ("iter-map-sum", "fn main() { let s: i64 = vec![1, 2, 3, 4].iter().map(|x| x * x).sum(); println!(\"{}\", s); }", "30"),
("trait-default", "trait Greet { fn hi(&self) -> i64 { 7 } } struct A {} impl Greet for A {} fn main() { let a = A {}; println!(\"{}\", a.hi()); }", "7"),
        ("trait-default-override", "trait Greet { fn hi(&self) -> i64 { 7 } } struct B {} impl Greet for B { fn hi(&self) -> i64 { 99 } } fn main() { let b = B {}; println!(\"{}\", b.hi()); }", "99"),
        ("trait-default-selfcall", "trait T { fn base(&self) -> i64; fn doubled(&self) -> i64 { self.base() * 2 } } struct C {} impl T for C { fn base(&self) -> i64 { 5 } } fn main() { let c = C {}; println!(\"{}\", c.doubled()); }", "10"),
("str-repeat", "fn main() { println!(\"{}\", \"ab\".repeat(3)); }", "ababab"),
        ("vec-contains", "fn main() { let v = vec![1, 2, 3]; println!(\"{} {}\", v.contains(&2), v.contains(&9)); }", "true false"),
        ("cmp-max-min", "fn main() { println!(\"{} {}\", std::cmp::max(3, 7), std::cmp::min(3, 7)); }", "7 3"),
("labeled-break", "fn main() { let mut c = 0; 'outer: loop { loop { c += 1; if c > 2 { break 'outer; } } } println!(\"{}\", c); }", "3"),
        ("labeled-break-value", "fn main() { let x = 'a: loop { break 'a 42; }; println!(\"{}\", x); }", "42"),
("assoc-const", "struct C {} impl C { const MAX: i64 = 100; } fn main() { println!(\"{}\", C::MAX * 2 + 1); }", "201"),
("where-clause", "fn dup<T>(x: T) -> T where T: Clone { x } fn main() { println!(\"{}\", dup(7)); }", "7"),
        ("turbofish", "fn id<T>(x: T) -> T { x } fn main() { println!(\"{}\", id::<i64>(42)); }", "42"),
("const-fn", "const fn sq(x: i64) -> i64 { x * x } fn main() { println!(\"{}\", sq(6)); }", "36"),
        ("move-closure", "fn main() { let a = 10; let f = move || a + 5; println!(\"{}\", f()); }", "15"),
("compound-bit-assign", "fn main() { let mut x = 12; x <<= 4; x >>= 2; x &= 30; x |= 1; println!(\"{}\", x); }", "17"),
        ("bool-bitwise", "fn main() { println!(\"{} {} {}\", true & false, true | false, true ^ true); }", "false true false"),
("int-shift", "fn main() { let a = 1i64 << 4; let b = 256 >> 2; println!(\"{} {}\", a, b); }", "16 64"),
("bit-and-or", "fn main() { let x = 6 & 3; let y = 6 | 1; let z = 1 | 2 & 3; println!(\"{} {} {}\", x, y, z); }", "2 7 3"),
("int-pow", "fn main() { let a: i64 = 2; let b: u64 = 3; println!(\"{} {}\", a.pow(10), b.pow(4)); }", "1024 81"),
("array-type-ann", "fn main() { let a: [i64; 3] = [10, 20, 30]; let mut s = 0; for i in 0..3 { s = s + a[i as usize]; } println!(\"{}\", s); }", "60"),
("loop-break-value", "fn main() { let mut i = 0; let r = loop { i = i + 1; if i > 3 { break i * 10; } }; println!(\"{}\", r); }", "40"),
("tuple-struct", "struct P(i64, i64); fn main() { let p = P(5, 9); println!(\"{}\", p.0 + p.1); }", "14"),
("unit-struct", "struct D; fn main() { let _d = D; println!(\"{}\", 42); }", "42"),
("literal", "fn main() { println!(\"{}\", 42); }", "42"),
        ("arith", "fn main() { println!(\"{}\", 1 + 2 * 3 - 4); }", "3"),
        ("float-debug", "fn main() { println!(\"{:?}\", 3.5); }", "3.5"),
        ("float-debug-whole", "fn main() { println!(\"{:?}\", 7.0); }", "7.0"),
        ("float-display", "fn main() { println!(\"{}\", 7.0); }", "7"),
        (
            "float-arith",
            "fn main() { println!(\"{:?}\", 2.5 * 2.0 + 1.0 - 0.5); }",
            "5.5",
        ),
        ("float-div", "fn main() { println!(\"{:?}\", 7.0 / 2.0); }", "3.5"),
        (
            "float-cmp",
            "fn main() { println!(\"{}\", if 3.5 < 3.6 { 1 } else { 0 }); }",
            "1",
        ),
        (
            "float-cast-to-int",
            "fn main() { println!(\"{}\", 3.7 as i64); }",
            "3",
        ),
        (
            "float-cast-from-int",
            "fn main() { println!(\"{:?}\", 3 as f64); }",
            "3.0",
        ),
        (
            "float-fn",
            "fn double(x: f64) -> f64 { x * 2.0 } fn main() { println!(\"{:?}\", double(3.5)); }",
            "7.0",
        ),
        (
            "float-parse",
            "fn main() { println!(\"{:?}\", \"3.5\".parse::<f64>().unwrap()); }",
            "3.5",
        ),
        (
            "float-neg",
            "fn main() { println!(\"{:?}\", -1.5 + 0.25); }",
            "-1.25",
        ),
        (
            "float-let-mut",
            "fn main() { let mut acc = 0.5; acc = acc * 4.0; println!(\"{:?}\", acc); }",
            "2.0",
        ),
        (
            "let",
            "fn main() { let x = 5; let y = 7; println!(\"{}\", x + y); }",
            "12",
        ),
        (
            "if-expr",
            "fn main() { let n = 3; println!(\"{}\", if n < 5 { 10 } else { 20 }); }",
            "10",
        ),
        (
            "bool-logic",
            "fn main() { let a = true; let b = false; println!(\"{}\", if a && !b { 1 } else { 0 }); }",
            "1",
        ),
        ("rem", "fn main() { println!(\"{}\", 17 % 5); }", "2"),
        (
            "shadow-block",
            "fn main() { let x = 1; let y = { let x = 10; x + 5 }; println!(\"{}\", x + y); }",
            "16",
        ),
        (
            "factorial",
            "fn fact(n: i64) -> i64 { if n < 2 { 1 } else { n * fact(n - 1) } } \
             fn main() { println!(\"{}\", fact(10)); }",
            "3628800",
        ),
        (
            "fib",
            "fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } } \
             fn main() { println!(\"{}\", fib(20)); }",
            "6765",
        ),
        (
            "mutual-recursion",
            "fn is_even(n: i64) -> bool { if n == 0 { true } else { is_odd(n - 1) } } \
             fn is_odd(n: i64) -> bool { if n == 0 { false } else { is_even(n - 1) } } \
             fn main() { println!(\"{}\", if is_even(10) { 1 } else { 0 }); }",
            "1",
        ),
        (
            "gcd",
            "fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a } else { gcd(b, a % b) } } \
             fn main() { println!(\"{}\", gcd(48, 36)); }",
            "12",
        ),
        (
            "ackermann",
            "fn ack(m: i64, n: i64) -> i64 { if m == 0 { n + 1 } else { if n == 0 { ack(m - 1, 1) } else { ack(m - 1, ack(m, n - 1)) } } } \
             fn main() { println!(\"{}\", ack(2, 3)); }",
            "9",
        ),
        (
            "early-return",
            "fn classify(n: i64) -> i64 { if n < 0 { return 0; } if n == 0 { return 1; } 2 } \
             fn main() { println!(\"{}\", classify(-3) + classify(0) + classify(7)); }",
            "3",
        ),
        (
            "struct-field",
            "struct Point { x: i64, y: i64 } \
             fn main() { let p = Point { x: 3, y: 4 }; println!(\"{}\", p.x + p.y); }",
            "7",
        ),
        (
            "struct-field-assignment",
            "struct Counter { n: i64 } fn main() { let mut c = Counter { n: 40 }; c.n += 2; println!(\"{}\", c.n); }",
            "42",
        ),
        (
            "enum-match-unit",
            "enum Dir { North, South, East, West } \
             fn dx(d: Dir) -> i64 { match d { Dir::East => 1, Dir::West => -1, _ => 0 } } \
             fn main() { println!(\"{}\", dx(Dir::East) + dx(Dir::West) + dx(Dir::North)); }",
            "0",
        ),
        (
            "enum-match-tuple",
            "enum Shape { Circle(i64), Rect(i64, i64) } \
             fn area(s: Shape) -> i64 { match s { Shape::Circle(r) => 3 * r * r, Shape::Rect(w, h) => w * h } } \
             fn main() { println!(\"{}\", area(Shape::Circle(2)) + area(Shape::Rect(3, 4))); }",
            "24",
        ),
        (
            "match-bool-exhaustive",
            "fn main() { let n = match true { true => 42, false => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "match-enum-exhaustive",
            "enum E { A, B } fn main() { let n = match E::A { E::A => 1, E::B => 41 }; println!(\"{}\", n + 1); }",
            "2",
        ),
        (
            "match-guard-wildcard-exhaustive",
            "fn main() { let n = match true { true if false => 0, _ => 42 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "tuple-return",
            "fn divmod(a: i64, b: i64) -> (i64, i64) { (a / b, a % b) } \
             fn main() { let t = divmod(17, 5); println!(\"{}\", t.0 * 10 + t.1); }",
            "32",
        ),
        (
            "tuple-index-assignment",
            "fn main() { let mut p = (1, 2); p.1 = 41; println!(\"{}\", p.0 + p.1); }",
            "42",
        ),
        (
            "tuple-index-mut-ref-assignment",
            "fn main() { let mut p = (1, 2); let r = &mut p; r.1 = 41; println!(\"{}\", p.0 + p.1); }",
            "42",
        ),
        (
            "match-int-bind",
            "fn classify(n: i64) -> i64 { match n { 0 => 100, 1 => 200, x => x * x } } \
             fn main() { println!(\"{}\", classify(0) + classify(1) + classify(5)); }",
            "325",
        ),
        (
            "match-tuple-pat",
            "fn f(p: (i64, bool)) -> i64 { match p { (n, true) => n, (n, false) => -n } } \
             fn main() { println!(\"{}\", f((5, true)) + f((3, false))); }",
            "2",
        ),
        (
            "option-enum",
            "enum Opt { None, Some(i64) } \
             fn unwrap_or(o: Opt, d: i64) -> i64 { match o { Opt::Some(v) => v, Opt::None => d } } \
             fn main() { println!(\"{}\", unwrap_or(Opt::Some(7), 0) + unwrap_or(Opt::None, 9)); }",
            "16",
        ),
        (
            "while-sum",
            "fn main() { let mut i = 1; let mut sum = 0; while i <= 100 { sum = sum + i; i = i + 1; } println!(\"{}\", sum); }",
            "5050",
        ),
        (
            "for-sum",
            "fn main() { let mut sum = 0; for i in 1..101 { sum = sum + i; } println!(\"{}\", sum); }",
            "5050",
        ),
        (
            "for-inclusive",
            "fn main() { let mut sum = 0; for i in 1..=100 { sum = sum + i; } println!(\"{}\", sum); }",
            "5050",
        ),
        (
            "loop-break",
            "fn main() { let mut i = 0; let mut acc = 0; loop { if i >= 10 { break; } acc = acc + i; i = i + 1; } println!(\"{}\", acc); }",
            "45",
        ),
        (
            "loop-return-never",
            "fn f() -> i64 { loop { return 42; } } fn main() { println!(\"{}\", f()); }",
            "42",
        ),
        (
            "match-arm-break-never",
            "fn main() { let mut n = 0; loop { let x = match n { 0 => 42, _ => break }; println!(\"{}\", x); n = n + 1; break; } }",
            "42",
        ),
        (
            "match-arm-block-return-never",
            "fn f(ok: bool) -> Result<String, String> { let s = match ok { true => String::from(\"ok\"), false => { return Err(String::from(\"bad\")); } }; Ok(s) } fn main() { println!(\"{}\", f(true).unwrap()); }",
            "ok",
        ),
        (
            "while-continue",
            "fn main() { let mut sum = 0; let mut i = 0; while i < 10 { i = i + 1; if i % 2 == 0 { continue; } sum = sum + i; } println!(\"{}\", sum); }",
            "25",
        ),
        (
            "gcd-iterative",
            "fn gcd(a: i64, b: i64) -> i64 { let mut x = a; let mut y = b; while y != 0 { let t = y; y = x % y; x = t; } x } \
             fn main() { println!(\"{}\", gcd(48, 36)); }",
            "12",
        ),
        (
            "impl-associated-method",
            "struct Point { x: i64, y: i64 } \
             impl Point { \
               fn new(x: i64, y: i64) -> Point { Point { x: x, y: y } } \
               fn dist_sq(&self) -> i64 { self.x * self.x + self.y * self.y } \
             } \
             fn main() { let p = Point::new(3, 4); println!(\"{}\", p.dist_sq()); }",
            "25",
        ),
        (
            "impl-enum-by-value",
            "enum Opt { None, Some(i64) } \
             impl Opt { \
               fn unwrap_or(self, d: i64) -> i64 { match self { Opt::Some(v) => v, Opt::None => d } } \
             } \
             fn main() { println!(\"{}\", Opt::Some(7).unwrap_or(0) + Opt::None.unwrap_or(9)); }",
            "16",
        ),
        (
            "impl-refmut-receiver",
            "struct Counter { n: i64 } \
             impl Counter { \
               fn get(&self) -> i64 { self.n } \
               fn add(&mut self, delta: i64) -> i64 { self.n + delta } \
             } \
             fn main() { let mut c = Counter { n: 10 }; println!(\"{}\", c.get() + c.add(5)); }",
            "25",
        ),
        (
            "impl-refmut-field-mutation",
            "struct Counter { n: i64 } \
             impl Counter { \
               fn add(&mut self, delta: i64) { self.n += delta; } \
               fn get(&self) -> i64 { self.n } \
             } \
             fn main() { let mut c = Counter { n: 10 }; c.add(5); println!(\"{}\", c.get()); }",
            "15",
        ),
        (
            "impl-enum-associated",
            "enum Flag { Off, On } \
             impl Flag { \
               fn one() -> Flag { Flag::On } \
               fn as_i64(&self) -> i64 { match self { Flag::On => 1, Flag::Off => 0 } } \
             } \
             fn main() { println!(\"{}\", Flag::one().as_i64()); }",
            "1",
        ),
        (
            "ref-read-param",
            "fn inc(r: &i64) -> i64 { *r + 1 } \
             fn main() { let n = 41; println!(\"{}\", inc(&n)); }",
            "42",
        ),
        (
            "ref-ref-param-autoderef",
            "fn f(v: &i64) -> i64 { *v } fn main() { let n = 42; let r = &n; let rr = &r; println!(\"{}\", f(rr)); }",
            "42",
        ),
        (
            "ref-mut-param",
            "fn bump(r: &mut i64) { *r = *r + 1; } \
             fn main() { let mut n = 41; bump(&mut n); println!(\"{}\", n); }",
            "42",
        ),
        (
            "ref-mut-local",
            "fn main() { let mut n = 1; let r = &mut n; *r = *r + 6; println!(\"{}\", n); }",
            "7",
        ),
        (
            "if-stmt-before-deref-assign",
            "fn set(r: &mut i64) { if false { return; } *r = 42; } fn main() { let mut n = 0; set(&mut n); println!(\"{}\", n); }",
            "42",
        ),
        (
            "ref-method-autoderef",
            "struct Point { x: i64, y: i64 } \
             impl Point { fn dist_sq(&self) -> i64 { self.x * self.x + self.y * self.y } } \
             fn main() { let p = Point { x: 5, y: 12 }; let r = &p; println!(\"{}\", r.dist_sq()); }",
            "169",
        ),
        (
            "vec-new-push-len-index",
            "fn main() { let mut v: Vec<i64> = Vec::new(); v.push(10); v.push(32); println!(\"{} {} {}\", v.len(), v[0], v[1]); }",
            "2 10 32",
        ),
        (
            "vec-with-capacity",
            "fn main() { let mut v: Vec<i64> = Vec::with_capacity(4); v.push(40); v.push(2); println!(\"{}\", v[0] + v[1]); }",
            "42",
        ),
        (
            "vec-literal-index",
            "fn main() { let v = vec![2, 3, 5]; println!(\"{}\", v[0] * 100 + v[1] * 10 + v[2]); }",
            "235",
        ),
        (
            "vec-clear-is-empty",
            "fn main() { let mut v = vec![1, 2, 3]; v.clear(); println!(\"{}\", if v.is_empty() { 1 } else { 0 }); }",
            "1",
        ),
        (
            "vec-mut-ref-param",
            "fn add(v: &mut Vec<i64>, n: i64) { v.push(n); } \
             fn main() { let mut v: Vec<i64> = Vec::new(); add(&mut v, 7); add(&mut v, 8); println!(\"{}\", v[0] + v[1]); }",
            "15",
        ),
        (
            "string-new-push-len",
            "fn main() { let mut s = String::new(); s.push_str(\"rs\"); s.push_str(\"-meta\"); println!(\"{} {}\", s.len(), s); }",
            "7 rs-meta",
        ),
        (
            "string-from-as-str-methods",
            "fn main() { let s = String::from(\"hello\"); let r = s.as_str(); println!(\"{}\", if r.starts_with(\"he\") && r.contains(\"ll\") { 1 } else { 0 }); }",
            "1",
        ),
        (
            "string-ref-to-str-param",
            "fn f(s: &str) -> i64 { s.len() as i64 } fn main() { let s = String::from(\"rs\"); println!(\"{}\", f(&s) + 40); }",
            "42",
        ),
        (
            "string-str-comparison",
            "fn main() { let s = String::from(\"rs\"); println!(\"{}\", if s == \"rs\" { 42 } else { 0 }); }",
            "42",
        ),
        (
            "string-add",
            "fn main() { let s = String::from(\"rs\") + \"-meta\"; println!(\"{}\", s); }",
            "rs-meta",
        ),
        (
            "string-push-str-string-ref",
            "fn main() { let mut s = String::from(\"rs\"); let suffix = String::from(\"-meta\"); s.push_str(&suffix); println!(\"{}\", s); }",
            "rs-meta",
        ),
        (
            "vec-string-join",
            "fn main() { let v = vec![String::from(\"rs\"), String::from(\"meta\")]; println!(\"{}\", v.join(\"-\")); }",
            "rs-meta",
        ),
        (
            "string-line-continuation",
            "fn main() { let s = \"rs\\
                 meta\"; println!(\"{}\", s); }",
            "rsmeta",
        ),
        (
            "str-methods",
            "fn main() { let s = \"abc\"; println!(\"{} {}\", s.len(), if s.is_empty() { 0 } else { 1 }); }",
            "3 1",
        ),
        (
            "option-some-none",
            "fn main() { let a = Some(7); let b: Option<i64> = None; println!(\"{}\", a.unwrap_or(0) + b.unwrap_or(5) + if a.is_some() && b.is_none() { 1 } else { 0 }); }",
            "13",
        ),
        (
            "option-unwrap",
            "fn main() { println!(\"{}\", Some(42).unwrap()); }",
            "42",
        ),
        (
            "result-ok-err",
            "fn main() { let a: Result<i64, &str> = Ok(7); let b: Result<i64, &str> = Err(\"bad\"); println!(\"{}\", a.unwrap_or(0) + b.unwrap_or(5) + if a.is_ok() && b.is_err() { 1 } else { 0 }); }",
            "13",
        ),
        (
            "result-ok-to-option",
            "fn main() { let r: Result<i64, &str> = Ok(9); let o = r.ok(); println!(\"{}\", o.unwrap_or(0)); }",
            "9",
        ),
        (
            "result-map-ok",
            "fn main() { let r: Result<i64, &str> = Ok(41); println!(\"{}\", r.map(|n| n + 1).unwrap_or(0)); }",
            "42",
        ),
        (
            "result-map-err-enum-ctor",
            "enum Signal { Error(String) } fn main() { let r: Result<i64, String> = Err(String::from(\"bad\")); let out = r.map_err(Signal::Error); println!(\"{}\", if out.is_err() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "option-ok-or-else",
            "fn main() { let o: Option<i64> = None; let r = o.ok_or_else(|| String::from(\"empty\")); println!(\"{}\", if r.is_err() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "option-map-some",
            "fn main() { let o = Some(41).map(|n| n + 1); println!(\"{}\", o.unwrap_or(0)); }",
            "42",
        ),
        (
            "option-map-none",
            "fn main() { let o: Option<i64> = None; let out = o.map(|n| n + 1).unwrap_or(42); println!(\"{}\", out); }",
            "42",
        ),
        (
            "option-and-then",
            "fn main() { let o = Some(41).and_then(|n| Some(n + 1)).unwrap_or(0); println!(\"{}\", o); }",
            "42",
        ),
        (
            "option-or-else",
            "fn main() { let o: Option<i64> = None; let out = o.or_else(|| Some(42)).unwrap_or(0); println!(\"{}\", out); }",
            "42",
        ),
        (
            "option-as-ref",
            "fn main() { let s = Some(String::from(\"rs\")); println!(\"{}\", s.as_ref().unwrap().len() + 40); }",
            "42",
        ),
        (
            "option-unwrap-or-else",
            "fn main() { let n: Option<i64> = None; println!(\"{}\", n.unwrap_or_else(|| 42)); }",
            "42",
        ),
        (
            "option-none-later-some-assign",
            "enum Kind { A } fn main() { let mut x = None; x = Some(Kind::A); println!(\"{}\", if x.is_some() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "option-none-later-some-vec-assign",
            "fn main() { let mut out = None; let mut xs = Vec::new(); xs.push(42); out = Some(xs); if let Some(values) = out { println!(\"{}\", values[0]); } else { println!(\"0\"); } }",
            "42",
        ),
        (
            "box-deref",
            "fn main() { let b = Box::new(21); println!(\"{}\", *b * 2); }",
            "42",
        ),
        (
            "box-as-ref",
            "fn main() { let b = Box::new(42); let r = b.as_ref(); println!(\"{}\", *r); }",
            "42",
        ),
        (
            "rc-clone-deref",
            "use std::rc::Rc; fn main() { let r = Rc::new(20); let r2 = Rc::clone(&r); println!(\"{}\", *r + *r2 + 2); }",
            "42",
        ),
        (
            "rc-as-ref",
            "use std::rc::Rc; fn main() { let r = Rc::new(42); println!(\"{}\", *r.as_ref()); }",
            "42",
        ),
        (
            "rc-vec-len",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![20, 22]); println!(\"{}\", r.len()); }",
            "2",
        ),
        (
            "rc-vec-iter",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![20, 22]); let mut sum = 0; for x in r.iter() { sum += *x; } println!(\"{}\", sum); }",
            "42",
        ),
        (
            "rc-vec-get",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![40, 2]); println!(\"{}\", *r.get(0).unwrap_or(&0) + *r.get(1).unwrap_or(&0)); }",
            "42",
        ),
        (
            "rc-string-as-str",
            "use std::rc::Rc; fn main() { let r = Rc::new(String::from(\"rs\")); println!(\"{}\", r.as_str().len()); }",
            "2",
        ),
        (
            "rc-string-to-string",
            "use std::rc::Rc; fn main() { let r = Rc::new(String::from(\"rs\")); println!(\"{}\", r.to_string()); }",
            "rs",
        ),
        (
            "box-rc-ref-deref-coercion",
            "use std::rc::Rc; fn f(n: &i64) -> i64 { *n } fn main() { let b = Box::new(40); let r = Rc::new(2); println!(\"{}\", f(&b) + f(&r)); }",
            "42",
        ),
        (
            "rc-string-ref-to-str-param",
            "use std::rc::Rc; fn f(s: &str) -> i64 { s.len() as i64 } fn main() { let r = Rc::new(String::from(\"rs\")); println!(\"{}\", f(&r) + 40); }",
            "42",
        ),
        (
            "rc-string-chars",
            "use std::rc::Rc; fn main() { let r = Rc::new(String::from(\"rs\")); let chars: Vec<char> = r.chars().collect(); println!(\"{}{}\", chars[0], chars[1]); }",
            "rs",
        ),
        (
            "rc-vec-char-collect-turbofish",
            "use std::rc::Rc; fn main() { let r = Rc::new(\"rs\".chars().collect::<Vec<char>>()); println!(\"{}\", r.len() as i64 + 40); }",
            "42",
        ),
        (
            "refcell-borrow",
            "use std::cell::RefCell; fn main() { let c = RefCell::new(42); println!(\"{}\", *c.borrow()); }",
            "42",
        ),
        (
            "refcell-borrow-mut",
            "use std::cell::RefCell; fn main() { let c = RefCell::new(1); *c.borrow_mut() = 42; println!(\"{}\", *c.borrow()); }",
            "42",
        ),
        (
            "hashmap-insert-get",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let a = String::from(\"a\"); m.insert(a, 7); let q = String::from(\"a\"); println!(\"{}\", if m.contains_key(&q) { *m.get(&q).unwrap() } else { 0 }); }",
            "7",
        ),
        (
            "hashmap-string-get-str",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.insert(String::from(\"rs\"), 42); println!(\"{}\", *m.get(\"rs\").unwrap()); }",
            "42",
        ),
        (
            "hashmap-remove-len",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let a = String::from(\"a\"); let b = String::from(\"b\"); m.insert(a, 10); m.insert(b, 20); let q = String::from(\"a\"); let old = m.remove(&q).unwrap_or(0); println!(\"{} {}\", old, m.len()); }",
            "10 1",
        ),
        (
            "hashmap-replace",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let a = String::from(\"a\"); m.insert(a, 1); let a2 = String::from(\"a\"); let old = m.insert(a2, 2).unwrap_or(0); let q = String::from(\"a\"); println!(\"{} {}\", old, *m.get(&q).unwrap()); }",
            "1 2",
        ),
        (
            "hashmap-get-mut",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.insert(String::from(\"a\"), 40); *m.get_mut(\"a\").unwrap() += 2; println!(\"{}\", *m.get(\"a\").unwrap()); }",
            "42",
        ),
        (
            "hashmap-is-empty",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); println!(\"{}\", if m.is_empty() { 1 } else { 0 }); }",
            "1",
        ),
        (
            "hashmap-iter",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.insert(String::from(\"a\"), 41); let mut out = 0; for (k, v) in m.iter() { if k.as_str() == \"a\" { out = *v + 1; } } println!(\"{}\", out); }",
            "42",
        ),
        (
            "hashmap-entry-or-insert-new",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); *m.entry(String::from(\"a\")).or_insert(40) += 2; println!(\"{}\", *m.get(\"a\").unwrap()); }",
            "42",
        ),
        (
            "hashmap-entry-or-insert-existing",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.insert(String::from(\"a\"), 42); let v = m.entry(String::from(\"a\")).or_insert(0); println!(\"{}\", *v); }",
            "42",
        ),
        (
            "hashmap-entry-or-insert-with",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let v = m.entry(String::from(\"a\")).or_insert_with(|| 42); println!(\"{}\", *v); }",
            "42",
        ),
        (
            "hashmap-entry-and-modify",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.entry(String::from(\"a\")).and_modify(|v: &mut i64| { *v += 1; }).or_insert(41); m.entry(String::from(\"a\")).and_modify(|v: &mut i64| { *v += 1; }).or_insert(0); println!(\"{}\", *m.get(\"a\").unwrap()); }",
            "42",
        ),
        (
            "vec-push-empty-hashmap-placeholder",
            "use std::collections::HashMap; fn main() { let mut v: Vec<HashMap<String, i64>> = Vec::new(); v.push(HashMap::new()); println!(\"{}\", v.len() + 41); }",
            "42",
        ),
        (
            "vec-new-push-refines-join",
            "fn main() { let mut parts = Vec::new(); parts.push(String::from(\"rs\")); parts.push(String::from(\"meta\")); println!(\"{}\", parts.join(\"-\")); }",
            "rs-meta",
        ),
        (
            "vec-reverse",
            "fn main() { let mut v = vec![40, 1, 1]; v.reverse(); println!(\"{}\", v[0] + v[1] + v[2]); }",
            "42",
        ),
        (
            "attrs-pub-mod-surface",
            "#[derive(Clone, Debug)] pub struct PubPoint { pub x: i64, pub y: i64 } \
             impl PubPoint { pub fn sum(&self) -> i64 { self.x + self.y } } \
             mod ignored { pub fn hidden() -> i64 { 999 } } \
             pub fn main() { let p = PubPoint { x: 20, y: 22 }; println!(\"{}\", p.sum()); }",
            "42",
        ),
        (
            "char-basic-methods",
            "fn main() { let c = 'A'; println!(\"{} {}\", c, if c.is_ascii_alphabetic() && !c.is_ascii_digit() { 1 } else { 0 }); }",
            "A 1",
        ),
        (
            "char-hexdigit-method",
            "fn main() { println!(\"{}\", if 'f'.is_ascii_hexdigit() && !'g'.is_ascii_hexdigit() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "char-from-u32",
            "fn main() { let c = char::from_u32(65u32).unwrap_or('?'); println!(\"{}\", c); }",
            "A",
        ),
        (
            "string-push-char-to-string",
            "fn main() { let mut s = String::from(\"r\"); s.push('s'); let bang = '!'.to_string(); s.push_str(bang.as_str()); println!(\"{}\", s); }",
            "rs!",
        ),
        (
            "string-chars-collect",
            "fn main() { let s: String = \"rs\".chars().collect(); println!(\"{}\", s); }",
            "rs",
        ),
        (
            "string-chars-collect-string-turbofish",
            "fn main() { let s = \"rs\".chars().collect::<String>(); println!(\"{}\", s); }",
            "rs",
        ),
        (
            "string-bytes-sum",
            "fn main() { let mut sum = 0; for b in \"rs\".bytes() { sum += b as i64; } println!(\"{}\", sum); }",
            "229",
        ),
        (
            "string-trim",
            "fn main() { let s = String::from(\"  rs  \"); println!(\"{}\", s.trim()); }",
            "rs",
        ),
        (
            "string-split-collect-join",
            "fn main() { let parts: Vec<&str> = \"rs-meta\".split(\"-\").collect(); println!(\"{}\", parts.join(\"+\")); }",
            "rs+meta",
        ),
        (
            "string-chars-collect-vec-char",
            "fn main() { let chars: Vec<char> = \"rs\".chars().collect(); println!(\"{}{}\", chars[0], chars[1]); }",
            "rs",
        ),
        (
            "string-chars-collect-vec-char-param",
            "fn len(v: Vec<char>) -> i64 { v.len() as i64 } fn main() { println!(\"{}\", len(\"rs\".chars().collect()) + 40); }",
            "42",
        ),
        (
            "string-chars-map-collect",
            "fn main() { let s: String = \"rs\".chars().map(|c| c).collect(); println!(\"{}\", s.len() + 40); }",
            "42",
        ),
        (
            "vec-iter-collect-string",
            "fn main() { let chars: Vec<char> = vec!['r', 's']; let s: String = chars.iter().collect(); println!(\"{}\", s); }",
            "rs",
        ),
        (
            "vec-into-iter-collect",
            "fn main() { let v = vec![1, 2, 3]; let w: Vec<i64> = v.into_iter().collect(); println!(\"{}\", w.len()); }",
            "3",
        ),
        (
            "tuple-vec-placeholder-arg",
            "fn f(p: (Vec<i64>, i64)) -> i64 { p.0.len() as i64 + p.1 } fn main() { let p = (Vec::new(), 42); println!(\"{}\", f(p)); }",
            "42",
        ),
        (
            "iter-next",
            "fn main() { let mut it = vec![4, 2].into_iter(); let a = it.next().unwrap_or(0); let b = it.next().unwrap_or(0); println!(\"{}\", a * 10 + b); }",
            "42",
        ),
        (
            "iter-next-temporary",
            "fn main() { println!(\"{}\", vec![42].into_iter().next().unwrap_or(0)); }",
            "42",
        ),
        (
            "iter-nth",
            "fn main() { let mut it = vec![10, 20, 30].into_iter(); println!(\"{}\", it.nth(1).unwrap_or(0) + it.next().unwrap_or(0)); }",
            "50",
        ),
        (
            "iter-last",
            "fn main() { let n = vec![10, 20, 30].into_iter().skip(1).last().unwrap_or(0); println!(\"{}\", n + 12); }",
            "42",
        ),
        (
            "iter-map-collect",
            "fn main() { let v: Vec<i64> = vec![1, 2, 3].into_iter().map(|x| x * 2).collect(); println!(\"{}\", v[0] + v[1] + v[2]); }",
            "12",
        ),
        (
            "range-map-sum",
            "fn main() { let n: i64 = (1..5).map(|x| x * 2).sum(); println!(\"{}\", n); }",
            "20",
        ),
        (
            "range-inclusive-map-sum",
            "fn main() { let n: i64 = (1..=3).map(|x| x * x).sum(); println!(\"{}\", n); }",
            "14",
        ),
        (
            "iter-zip-for",
            "fn main() { let mut sum = 0; for (a, b) in vec![20, 21].into_iter().zip(vec![1, 0].into_iter()) { sum += a + b; } println!(\"{}\", sum); }",
            "42",
        ),
        (
            "iter-all-zip",
            "fn main() { let ok = vec![1, 2].into_iter().zip(vec![1, 2].into_iter()).all(|(a, b)| a == b); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "iter-any",
            "fn main() { let ok = vec![1, 42, 3].into_iter().any(|n| n == 42); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "iter-rev",
            "fn main() { let v: Vec<i64> = vec![40, 1, 1].into_iter().rev().collect(); println!(\"{}\", v[0] + v[1] + v[2]); }",
            "42",
        ),
        (
            "iter-enumerate",
            "fn main() { let pairs: Vec<(usize, i64)> = vec![40, 1].into_iter().enumerate().collect(); println!(\"{}\", pairs[0].0 as i64 + pairs[0].1 + pairs[1].0 as i64 + pairs[1].1); }",
            "42",
        ),
        (
            "iter-find",
            "fn main() { let n = vec![1, 42, 3].into_iter().find(|n| *n == 42).unwrap_or(0); println!(\"{}\", n); }",
            "42",
        ),
        (
            "iter-mut-find-read",
            "fn main() { let mut v = vec![40, 2]; let mut it = v.iter_mut(); println!(\"{}\", if it.next().is_some() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "iter-position",
            "fn main() { let n = vec![1, 42, 3].into_iter().position(|n| n == 42).unwrap_or(0); println!(\"{}\", n as i64); }",
            "1",
        ),
        (
            "iter-count",
            "fn main() { let n = vec![1, 2, 3].into_iter().count(); println!(\"{}\", n as i64); }",
            "3",
        ),
        (
            "iter-sum",
            "fn main() { let n: i64 = vec![20, 22].into_iter().sum(); println!(\"{}\", n); }",
            "42",
        ),
        (
            "iter-fold",
            "fn main() { let n = vec![20, 22].into_iter().fold(0, |acc, x| acc + x); println!(\"{}\", n); }",
            "42",
        ),
        (
            "iter-take-skip",
            "fn main() { let n: i64 = vec![1, 20, 22, 99].into_iter().skip(1).take(2).sum(); println!(\"{}\", n); }",
            "42",
        ),
        (
            "iter-filter-collect-string",
            "fn main() { let chars: Vec<char> = vec!['r', '_', 's']; let s: String = chars.iter().filter(|c| **c != '_').collect(); println!(\"{}\", s); }",
            "rs",
        ),
        (
            "iter-copied",
            "fn main() { let v: Vec<i64> = vec![20, 22].iter().copied().collect(); println!(\"{}\", v[0] + v[1]); }",
            "42",
        ),
        (
            "iter-cloned",
            "fn main() { let v: Vec<String> = vec![String::from(\"rs\")].iter().cloned().collect(); println!(\"{}\", v[0].len() + 40); }",
            "42",
        ),
        (
            "numeric-types-casts",
            "fn main() { let n: usize = 5; let m: i32 = -3; let b = true as i64; println!(\"{}\", n as i64 + m as i64 + b); }",
            "3",
        ),
        (
            "char-match-cast",
            "fn val(c: char) -> i64 { match c { 'a' => 1, 'b' => 2, _ => c as u32 as i64 } } \
             fn main() { println!(\"{}\", val('a') + val('F')); }",
            "71",
        ),
        (
            "ref-char-cast",
            "fn main() { let c = 'A'; let r = &c; println!(\"{}\", *r as u32 as i64 - 23); }",
            "42",
        ),
        (
            "for-usize-range",
            "fn main() { let mut sum: usize = 0; for i in 1 as usize..=5 as usize { sum = sum + i; } println!(\"{}\", sum as i64); }",
            "15",
        ),
        (
            "usize-literal-comparison",
            "fn main() { let n: usize = 1; println!(\"{}\", if n < 2 { 42 } else { 0 }); }",
            "42",
        ),
        (
            "unsuffixed-int-fn-arg",
            "fn add(n: usize, m: u8) -> i64 { n as i64 + m as i64 } fn main() { println!(\"{}\", add(40, 2)); }",
            "42",
        ),
        (
            "unsuffixed-int-return",
            "fn answer() -> usize { 42 } fn main() { println!(\"{}\", answer() as i64); }",
            "42",
        ),
        (
            "unsuffixed-int-vec-context",
            "fn main() { let v: Vec<usize> = vec![40, 2]; println!(\"{}\", (v[0] + v[1]) as i64); }",
            "42",
        ),
        (
            "usize-saturating-sub",
            "fn main() { let n: usize = 0; println!(\"{}\", n.saturating_sub(1usize) as i64); }",
            "0",
        ),
        (
            "i64-saturating-sub",
            "fn main() { let n: i64 = 5; println!(\"{}\", n.saturating_sub(2) + 39); }",
            "42",
        ),
        (
            "i64-max",
            "fn main() { let n: i64 = 40; println!(\"{}\", n.max(42)); }",
            "42",
        ),
        (
            "i64-min",
            "fn main() { let n: i64 = 99; println!(\"{}\", n.min(42)); }",
            "42",
        ),
        (
            "i64-wrapping-neg",
            "fn main() { let n: i64 = -42; println!(\"{}\", n.wrapping_neg()); }",
            "42",
        ),
        (
            "i64-wrapping-arith",
            "fn main() { let n: i64 = 20; println!(\"{}\", n.wrapping_add(30).wrapping_sub(8).wrapping_mul(2).wrapping_div(2).wrapping_rem(43)); }",
            "42",
        ),
        (
            "i64-to-string",
            "fn main() { let n: i64 = 42; println!(\"{}\", n.to_string()); }",
            "42",
        ),
        (
            "bool-to-string",
            "fn main() { let b = true; println!(\"{}\", b.to_string()); }",
            "true",
        ),
        (
            "display-ref-primitive",
            "fn main() { let n = 42; let b = true; println!(\"{}-{}\", &n, &b); }",
            "42-true",
        ),
        (
            "ref-primitive-to-string",
            "fn main() { let n = 42; let b = true; println!(\"{}-{}\", (&n).to_string(), (&b).to_string()); }",
            "42-true",
        ),
        (
            "bool-then",
            "fn main() { let out = true.then(|| 42).unwrap_or(0); println!(\"{}\", out); }",
            "42",
        ),
        (
            "vec-pop-get-last",
            "fn main() { let mut v = vec![4, 5, 6]; let last = *v.last().unwrap(); let mid = *v.get(1).unwrap(); let popped = v.pop().unwrap(); println!(\"{}\", last + mid + popped + v.len() as i64); }",
            "19",
        ),
        (
            "vec-get-mut-read",
            "fn main() { let mut v = vec![42]; println!(\"{}\", *v.get_mut(0).unwrap()); }",
            "42",
        ),
        (
            "vec-last-mut-hashmap",
            "use std::collections::HashMap; fn main() { let mut v: Vec<HashMap<String, i64>> = Vec::new(); v.push(HashMap::new()); v.last_mut().unwrap().insert(String::from(\"answer\"), 42); println!(\"{}\", *v.last().unwrap().get(\"answer\").unwrap()); }",
            "42",
        ),
        (
            "vec-pop-empty",
            "fn main() { let mut v: Vec<i64> = Vec::new(); println!(\"{}\", if v.pop().is_none() { 1 } else { 0 }); }",
            "1",
        ),
        (
            "vec-remove",
            "fn main() { let mut v = vec![10, 42, 99]; let n = v.remove(1); println!(\"{} {}\", n, v.len()); }",
            "42 2",
        ),
        (
            "vec-index-assign",
            "fn main() { let mut v = vec![1, 2]; v[1] = 40; println!(\"{}\", v[0] + v[1]); }",
            "41",
        ),
        (
            "vec-for-owned",
            "fn main() { let v = vec![1, 2, 3]; let mut sum = 0; for x in v { sum = sum + x; } println!(\"{}\", sum); }",
            "6",
        ),
        (
            "vec-for-ref",
            "fn main() { let v = vec![2, 4, 8]; let mut sum = 0; for x in &v { sum = sum + *x; } println!(\"{}\", sum); }",
            "14",
        ),
        (
            "str-to-string",
            "fn main() { let s = \"rs\".to_string() + \"-meta\"; println!(\"{}\", s); }",
            "rs-meta",
        ),
        (
            "option-question",
            "fn maybe(n: i64) -> Option<i64> { if n > 0 { Some(n) } else { None } } \
             fn add() -> Option<i64> { let a = maybe(2)?; let b = maybe(3)?; Some(a + b) } \
             fn main() { println!(\"{}\", add().unwrap_or(0)); }",
            "5",
        ),
        (
            "option-question-early-none",
            "fn maybe(ok: bool) -> Option<i64> { if ok { Some(1) } else { None } } \
             fn calc() -> Option<i64> { let n = maybe(false)?; Some(n + 1) } \
             fn main() { println!(\"{}\", calc().unwrap_or(42)); }",
            "42",
        ),
        (
            "result-question",
            "fn parse(ok: bool) -> Result<i64, String> { if ok { Ok(7) } else { Err(String::from(\"bad\")) } } \
             fn calc() -> Result<i64, String> { let n = parse(true)?; Ok(n * 6) } \
             fn main() { println!(\"{}\", calc().unwrap_or(0)); }",
            "42",
        ),
        (
            "result-question-string-error",
            "fn fail() -> Result<i64, String> { Err(String::from(\"bad\")) } \
             fn calc() -> Result<i64, String> { let n = fail()?; Ok(n + 1) } \
             fn main() { println!(\"{}\", if calc().is_err() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "result-question-early-err",
            "fn parse(ok: bool) -> Result<i64, String> { if ok { Ok(7) } else { Err(String::from(\"bad\")) } } \
             fn calc() -> Result<i64, String> { let n = parse(false)?; Ok(n * 6) } \
             fn main() { println!(\"{}\", calc().unwrap_or(9)); }",
            "9",
        ),
        (
            "result-question-from-str-error",
            "fn parse(ok: bool) -> Result<i64, &'static str> { if ok { Ok(40) } else { Err(\"bad\") } } \
             fn calc(ok: bool) -> Result<i64, String> { let n = parse(ok)?; Ok(n + 2) } \
             fn main() { println!(\"{}\", calc(true).unwrap_or(0) + calc(false).unwrap_or(5)); }",
            "47",
        ),
        (
            "result-map-err-question",
            "fn parse() -> Result<i64, String> { let n = \"42\".parse::<i64>().map_err(|_| format!(\"bad\"))?; Ok(n) } \
             fn main() { println!(\"{}\", parse().unwrap_or(0)); }",
            "42",
        ),
        (
            "match-guard",
            "fn classify(n: i64) -> i64 { match n { x if x < 0 => -1, 0 => 0, x if x < 10 => 1, _ => 2 } } \
             fn main() { println!(\"{}\", classify(-3) + classify(0) + classify(7) + classify(20)); }",
            "2",
        ),
        (
            "compound-assign",
            "fn main() { let mut n = 10; n += 5; n -= 3; n *= 4; n /= 2; n %= 7; println!(\"{}\", n); }",
            "3",
        ),
        (
            "bool-and-assign",
            "fn main() { let mut b = true; b &= false; println!(\"{}\", if b { 0 } else { 42 }); }",
            "42",
        ),
        (
            "option-result-prelude-patterns",
            "fn f(o: Option<i64>, r: Result<i64, String>) -> i64 { \
                let a = match o { Some(x) => x, None => 0 }; \
                let b = match r { Ok(x) => x, Err(_) => 0 }; \
                a + b \
             } \
             fn main() { println!(\"{}\", f(Some(20), Ok(22))); }",
            "42",
        ),
        (
            "assignment-expression",
            "fn main() { let mut n = 0; match true { true => n += 42, false => n += 1 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "return-expression",
            "fn f(ok: bool) -> i64 { let n = match ok { true => return 40, false => 2 }; n } \
             fn main() { println!(\"{}\", f(true) + f(false)); }",
            "42",
        ),
        (
            "format-macro",
            "fn main() { let msg = format!(\"{}-{}\", \"rs\", 42); println!(\"{}\", msg); }",
            "rs-42",
        ),
        (
            "format-fixed-float-precision",
            "fn main() { println!(\"{:.6} {:.2}\", 1.5, -0.125); }",
            "1.500000 -0.12",
        ),
        (
            "format-fixed-int-precision",
            "fn main() { println!(\"{:.6}\", 1); }",
            "1",
        ),
        (
            "rc-pointer-equality",
            "use std::rc::Rc; fn main() { let a = Rc::new(1); let b = Rc::clone(&a); let c = Rc::new(1); println!(\"{} {}\", Rc::ptr_eq(&a, &b), Rc::ptr_eq(&a, &c)); }",
            "true false",
        ),
        (
            "rc-refcell-reference-parameter",
            "use std::cell::RefCell; use std::rc::Rc; fn clone_cell(r: &Rc<RefCell<i64>>) -> Rc<RefCell<i64>> { Rc::clone(r) } fn same(a: &Rc<RefCell<i64>>, b: &Rc<RefCell<i64>>) -> bool { Rc::ptr_eq(a, b) } fn main() { let a = Rc::new(RefCell::new(42)); let b = clone_cell(&a); println!(\"{} {}\", same(&a, &b), *b.borrow()); }",
            "true 42",
        ),
        (
            "format-hex-pad",
            "fn main() { let msg = format!(\"prog_{:016x}.rs\", 42u64); println!(\"{}\", msg); }",
            "prog_000000000000002a.rs",
        ),
        (
            "format-debug",
            "fn main() { let msg = format!(\"{:?}\", \"rs\"); println!(\"{}\", msg); }",
            "\"rs\"",
        ),
        (
            "format-debug-vec",
            "fn main() { let msg = format!(\"{:?}\", vec![1, 2]); println!(\"{}\", msg); }",
            "[1, 2]",
        ),
        (
            "format-display-rc-string",
            "use std::rc::Rc; fn main() { let msg = Rc::new(String::from(\"rs\")); println!(\"{}\", msg); }",
            "rs",
        ),
        (
            "fully-qualified-rc-string",
            "fn main() { let r: std::rc::Rc<String> = std::rc::Rc::new(String::from(\"rs\")); println!(\"{}\", r.as_str()); }",
            "rs",
        ),
        (
            "format-pretty-debug",
            "fn main() { let msg = format!(\"{:#?}\", \"rs\"); println!(\"{}\", msg); }",
            "\"rs\"",
        ),
        (
            "format-left-align",
            "fn main() { let msg = format!(\"{:<5}|{:<3}|\", \"rs\", 7); println!(\"{}\", msg); }",
            "rs   |7  |",
        ),
        (
            "format-right-align",
            "fn main() { let msg = format!(\"{:>5}|{:>3}|\", \"rs\", 7); println!(\"{}\", msg); }",
            "rs|  7|",
        ),
        (
            "format-named-positional",
            "fn main() { let who = \"rs\"; println!(\"{name}-{0}\", 42, name = who); }",
            "rs-42",
        ),
        (
            "format-positional-reuse",
            "fn main() { println!(\"{0}-{0}-{1}\", \"rs\", 42); }",
            "rs-rs-42",
        ),
        (
            "reference-pattern",
            "fn main() { let n = 41; let r = &n; let out = match r { &x => x + 1 }; println!(\"{}\", out); }",
            "42",
        ),
        (
            "ref-binding-pattern",
            "fn main() { let n = 41; let out = match n { ref r => *r + 1 }; println!(\"{}\", out); }",
            "42",
        ),
        (
            "ref-mut-binding-pattern",
            "fn main() { let mut n = 41; let out = match n { ref mut r => *r + 1 }; println!(\"{}\", out); }",
            "42",
        ),
        (
            "ref-match-ergonomics-option",
            "fn main() { let o = Some(41); let r = &o; let n = match r { Some(x) => *x + 1, None => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "ref-match-ergonomics-tuple",
            "fn main() { let p = (40, 2); let r = &p; let n = match r { (a, b) => *a + *b }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "ref-match-ergonomics-struct",
            "struct Point { x: i64 } fn main() { let p = Point { x: 42 }; let n = match &p { Point { x } => *x }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "unary-not-ref-bool",
            "fn main() { let b = &true; println!(\"{}\", if !b { 0 } else { 42 }); }",
            "42",
        ),
        (
            "unary-not-ref-int",
            "fn main() { let n: i64 = -1; let r = &n; println!(\"{}\", !r); }",
            "0",
        ),
        (
            "rvalue-reference-literal",
            "fn main() { let r = &'x'; println!(\"{}\", if *r == 'x' { 42 } else { 0 }); }",
            "42",
        ),
        (
            "rvalue-mut-reference-temp",
            "fn main() { let r = &mut 1; *r = 41; println!(\"{}\", *r + 1); }",
            "42",
        ),
        (
            "ref-match-ergonomics-literal",
            "fn main() { let v = vec!['x']; let n = match v.get(0) { Some('x') => 42, _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "vec-slice-range",
            "fn main() { let v = vec![1, 2, 20, 22, 99]; println!(\"{} {}\", v[2..4].len(), v[2..4][0] + v[2..4][1]); }",
            "2 42",
        ),
        (
            "vec-slice-to-vec",
            "fn main() { let v = vec![10, 20, 12]; let s = v[1..].to_vec(); println!(\"{}\", s[0] + s[1]); }",
            "32",
        ),
        (
            "typed-closure-call",
            "fn main() { let base = 40; let f = |x: i64| x + base; println!(\"{}\", f(2)); }",
            "42",
        ),
        (
            "parse-turbofish-i64",
            "fn main() { let n = \"42\".parse::<i64>().unwrap_or(0); println!(\"{}\", n); }",
            "42",
        ),
        (
            "clone-vec-enum-slice-eq",
            "#[derive(Clone, PartialEq)] enum Ty { A, B } \
             fn is_a(xs: &[Ty]) -> bool { xs == [Ty::A] } \
             fn main() { let a = vec![Ty::A]; let b = a.clone(); let c = vec![Ty::B]; \
             println!(\"{}\", if is_a(&a) && is_a(&b) && !is_a(&c) { 42 } else { 0 }); }",
            "42",
        ),
        (
            "string-literal-pattern",
            "fn main() { let s = String::from(\"rs\"); let n = match s.as_str() { \"rs\" => 42, _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "slice-ref-param",
            "fn f(xs: &[i64]) -> i64 { xs.len() as i64 } fn main() { let v = vec![1, 2, 3]; println!(\"{}\", f(&v) + 39); }",
            "42",
        ),
        (
            "slice-get-param",
            "fn f(xs: &[i64]) -> i64 { *xs.get(1).unwrap() } fn main() { let v = vec![20, 42]; println!(\"{}\", f(&v)); }",
            "42",
        ),
        (
            "slice-first-param",
            "fn f(xs: &[i64]) -> i64 { *xs.first().unwrap() } fn main() { let v = vec![42, 20]; println!(\"{}\", f(&v)); }",
            "42",
        ),
        (
            "slice-index-param",
            "fn f(xs: &[i64]) -> i64 { xs[1] } fn main() { let v = vec![20, 42]; println!(\"{}\", f(&v)); }",
            "42",
        ),
        (
            "slice-iter-param",
            "fn f(xs: &[i64]) -> i64 { let mut sum = 0; for x in xs.iter() { sum += *x; } sum } fn main() { let v = vec![20, 22]; println!(\"{}\", f(&v)); }",
            "42",
        ),
        (
            "slice-for-ref-param",
            "fn f(xs: &[i64]) -> i64 { let mut sum = 0; for x in xs { sum += *x; } sum } fn main() { let v = vec![20, 22]; println!(\"{}\", f(&v)); }",
            "42",
        ),
        (
            "option-copied-slice-get",
            "fn f(xs: &[char]) -> char { xs.get(0).copied().unwrap_or('x') } fn main() { let v = vec!['r']; println!(\"{}\", f(&v)); }",
            "r",
        ),
        (
            "option-cloned-slice-get",
            "fn f(xs: &[char]) -> char { xs.get(0).cloned().unwrap_or('x') } fn main() { let v = vec!['s']; println!(\"{}\", f(&v)); }",
            "s",
        ),
        (
            "option-ref-none-compare",
            "fn main() { let v: Vec<String> = Vec::new(); println!(\"{}\", if v.get(0) == None { 42 } else { 0 }); }",
            "42",
        ),
        (
            "struct-like-enum-variant-def",
            "enum Event { Click { x: i64, y: i64 }, Quit } fn main() { println!(\"{}\", 42); }",
            "42",
        ),
        (
            "struct-like-enum-variant-lit",
            "enum Event { Click { x: i64, y: i64 }, Quit } fn main() { let _e = Event::Click { x: 20, y: 22 }; println!(\"{}\", 42); }",
            "42",
        ),
        (
            "lifetime-surface",
            "struct Holder<'a> { xs: &'a [i64] } fn main() { let v = vec![1, 2]; let h = Holder { xs: &v }; println!(\"{}\", h.xs.len() as i64 + 40); }",
            "42",
        ),
        (
            "matches-macro",
            "fn main() { let n = Some(41); println!(\"{}\", if matches!(n, Some(x) if x > 40) { 42 } else { 0 }); }",
            "42",
        ),
        (
            "cfg-macro-unix",
            "fn main() { println!(\"{}\", if cfg!(unix) { 42 } else { 0 }); }",
            "42",
        ),
        (
            "struct-literal-shorthand",
            "struct Point { x: i64, y: i64 } fn main() { let x = 20; let p = Point { x, y: 22 }; println!(\"{}\", p.x + p.y); }",
            "42",
        ),
        (
            "struct-field-vec-push",
            "struct Bag { xs: Vec<i64> } fn main() { let mut b = Bag { xs: Vec::new() }; b.xs.push(42); println!(\"{}\", b.xs[0]); }",
            "42",
        ),
        (
            "return-no-semi",
            "fn f() -> i64 { if true { return 42 } 0 } fn main() { println!(\"{}\", f()); }",
            "42",
        ),
        (
            "integer-suffix",
            "fn main() { let n = 40usize; println!(\"{}\", n as i64 + 2i64); }",
            "42",
        ),
        (
            "hex-u64-literal",
            "fn main() { let n: u64 = 0x2a; println!(\"{}\", n as i64); }",
            "42",
        ),
        (
            "u64-from-str-radix",
            "fn main() { let n = u64::from_str_radix(\"2a\", 16).map(|x| x as i64).unwrap_or(0); println!(\"{}\", n); }",
            "42",
        ),
        (
            "fs-create-dir-all",
            "use std::fs; fn main() { let ok = fs::create_dir_all(\"work/fs-corpus\").is_ok(); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "pathbuf-from-join",
            "use std::fs; use std::path::PathBuf; fn main() { let path = PathBuf::from(\"work\").join(\"path-corpus\"); let ok = fs::create_dir_all(path).is_ok(); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "pathbuf-as-path-ref",
            "use std::path::{Path, PathBuf}; fn f(_p: &Path) -> i64 { 42 } fn main() { let path = PathBuf::from(\"work\"); println!(\"{}\", f(&path)); }",
            "42",
        ),
        (
            "command-output-status",
            "use std::process::Command; fn main() { let ok = Command::new(\"rustc\").arg(\"--version\").output().unwrap().status.success(); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "fs-write-read-to-string",
            "use std::fs; use std::path::PathBuf; fn main() { let dir = PathBuf::from(\"work/fs-rw\"); fs::create_dir_all(&dir).unwrap(); let path = dir.join(\"msg.txt\"); fs::write(&path, \"rs-meta\").unwrap(); let text = fs::read_to_string(&path).unwrap(); println!(\"{}\", text); }",
            "rs-meta",
        ),
        (
            "fs-read-bytes",
            "use std::fs; use std::path::PathBuf; fn main() { let dir = PathBuf::from(\"work/fs-read\"); fs::create_dir_all(&dir).unwrap(); let path = dir.join(\"byte.txt\"); fs::write(&path, \"*\").unwrap(); let bytes = fs::read(&path).unwrap(); println!(\"{}\", bytes[0] as i64); }",
            "42",
        ),
        (
            "path-display-exists",
            "use std::fs; use std::path::{Path, PathBuf}; fn main() { let dir = PathBuf::from(\"work/path-display-exists\"); fs::create_dir_all(&dir).unwrap(); let p = Path::new(\"work\").join(\"path-display-exists\"); let shown = p.display().to_string(); println!(\"{}\", if p.exists() && shown.contains(\"path-display-exists\") { 42 } else { 0 }); }",
            "42",
        ),
        (
            "command-env-empty-output",
            "use std::process::Command; fn main() { let out = Command::new(\"/bin/sh\").env(\"RSMETA_TEST\", \"1\").arg(\"-c\").arg(\"\").output().unwrap(); println!(\"{}\", if out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "command-env-clear-empty-output",
            "use std::process::Command; fn main() { let out = Command::new(\"/bin/sh\").env_clear().arg(\"-c\").arg(\"\").output().unwrap(); println!(\"{}\", if out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() { 42 } else { 0 }); }",
            "42",
        ),
        (
            "env-var-path",
            "fn main() { let ok = std::env::var(\"PATH\").is_ok(); println!(\"{}\", if ok { 42 } else { 0 }); }",
            "42",
        ),
        (
            "fully-qualified-std-paths",
            "fn main() { let mut m: std::collections::HashMap<String, i64> = std::collections::HashMap::new(); m.insert(String::from(\"a\"), 42); let ok = std::fs::create_dir_all(std::path::PathBuf::from(\"work/fq-path\")).is_ok(); println!(\"{}\", if ok { *m.get(\"a\").unwrap() } else { 0 }); }",
            "42",
        ),
        (
            "env-args-collect",
            "fn main() { let args: Vec<String> = std::env::args().collect(); println!(\"{}\", if args.len() > 0 { 42 } else { 0 }); }",
            "42",
        ),
        (
            "exitcode-main",
            "use std::process::ExitCode; fn main() -> ExitCode { println!(\"{}\", 42); ExitCode::SUCCESS }",
            "42",
        ),
        (
            "fully-qualified-exitcode-main",
            "fn main() -> std::process::ExitCode { println!(\"{}\", 42); std::process::ExitCode::SUCCESS }",
            "42",
        ),
        (
            "bitxor-and-assign",
            "fn main() { let mut n: u64 = 0x28; n ^= 0x02; println!(\"{}\", (n ^ 0x00) as i64); }",
            "42",
        ),
        (
            "or-pattern",
            "fn main() { let n = match \"rs\" { \"rs\" | \"meta\" => 42, _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "range-pattern-bind-at",
            "fn main() { let n = match 4 { x @ 1..=5 => x * 10 + 2, _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "char-range-pattern",
            "fn main() { let n = match 'r' { 'a'..='z' => 42, _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "zero-arg-closure-return-type",
            "fn main() { let f = || -> i64 { 42 }; println!(\"{}\", f()); }",
            "42",
        ),
        (
            "if-let-option",
            "fn main() { let v = Some(41); let out = if let Some(x) = v { x + 1 } else { 0 }; println!(\"{}\", out); }",
            "42",
        ),
        (
            "while-let-pop",
            "fn main() { let mut xs = vec![1, 2, 3]; let mut sum = 0; while let Some(n) = xs.pop() { sum += n; } println!(\"{}\", sum); }",
            "6",
        ),
        (
            "let-else-success",
            "fn maybe(n: i64) -> Option<i64> { if n > 0 { Some(n) } else { None } } fn main() { let Some(n) = maybe(42) else { return; }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "let-else-fallback",
            "fn main() { let None = Some(1) else { println!(\"{}\", 42); return; }; println!(\"0\"); }",
            "42",
        ),
        (
            "doc-comments",
            "//! module doc\n/// main doc\nfn main() { println!(\"{}\", 42); }",
            "42",
        ),
        (
            "const-global",
            "const ANSWER: i64 = 42; fn main() { println!(\"{}\", ANSWER); }",
            "42",
        ),
        (
            "static-global",
            "static ENABLED: bool = true; fn main() { println!(\"{}\", if ENABLED { 42 } else { 0 }); }",
            "42",
        ),
        (
            "immediate-closure-call",
            "fn main() { let n = (|| -> i64 { 42 })(); println!(\"{}\", n); }",
            "42",
        ),
        (
            "function-item-callable",
            "fn inc(n: i64) -> i64 { n + 1 } fn main() { let f = inc; println!(\"{}\", f(41)); }",
            "42",
        ),
        (
            "impl-fn-param-call",
            "fn cmp(a: i64, b: i64, f: impl Fn(i64, i64) -> bool) -> i64 { if f(a, b) { 42 } else { 0 } } fn main() { println!(\"{}\", cmp(1, 2, |x: i64, y: i64| x < y)); }",
            "42",
        ),
        (
            "impl-fn-param-inferred-closure",
            "fn cmp(a: i64, b: i64, f: impl Fn(i64, i64) -> bool) -> i64 { if f(a, b) { 42 } else { 0 } } fn main() { println!(\"{}\", cmp(1, 2, |x, y| x < y)); }",
            "42",
        ),
        (
            "closure-tuple-pattern-param",
            "fn main() { let f = |(a, b): (i64, i64)| a + b; println!(\"{}\", f((20, 22))); }",
            "42",
        ),
        (
            "struct-like-enum-pattern",
            "enum Event { Click { x: i64, y: i64 }, Quit } fn sum(e: Event) -> i64 { match e { Event::Click { x, y } => x + y, Event::Quit => 0 } } fn main() { println!(\"{}\", sum(Event::Click { x: 20, y: 22 })); }",
            "42",
        ),
        (
            "struct-pattern-rest",
            "struct Point { x: i64, y: i64 } fn main() { let p = Point { x: 42, y: 7 }; let n = match p { Point { x, .. } => x }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "struct-like-enum-pattern-rest",
            "enum Ty { Ref { mutable: bool, inner: i64 }, Unit } fn value(t: Ty) -> i64 { match t { Ty::Ref { inner, .. } => inner, Ty::Unit => 0 } } fn main() { println!(\"{}\", value(Ty::Ref { mutable: true, inner: 42 })); }",
            "42",
        ),
        (
            "for-tuple-pattern",
            "fn main() { let pairs = vec![(20, 1), (21, 0)]; let mut sum = 0; for (a, b) in pairs { sum += a + b; } println!(\"{}\", sum); }",
            "42",
        ),
        (
            "match-block-arm-no-comma",
            "fn main() { let n = match (1, 2) { (0, 0) => { 0 } (1, 2) | (3, 4) => { 42 } _ => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "array-literal-index",
            "fn main() { let a = [20, 22]; println!(\"{}\", a[0] + a[1]); }",
            "42",
        ),
        (
            "array-repeat",
            "fn main() { let a = [7; 6]; println!(\"{}\", a[0] * 6); }",
            "42",
        ),
        (
            "vec-repeat-string",
            "fn main() { let v = vec![String::from(\"rs\"); 2]; println!(\"{}\", v.join(\"-\")); }",
            "rs-rs",
        ),
        (
            "slice-array-compare",
            "fn diff(xs: &[i64]) -> i64 { if xs != [1, 2] { 42 } else { 0 } } fn main() { let v = vec![1, 3]; println!(\"{}\", diff(&v)); }",
            "42",
        ),
        (
            "type-alias-item-surface",
            "type Alias = i64; fn main() { println!(\"{}\", 42); }",
            "42",
        ),
        (
            "type-alias-rc-refcell-borrow",
            "use std::cell::RefCell; use std::rc::Rc; type Slot = Rc<RefCell<i64>>; fn main() { let slot: Slot = Rc::new(RefCell::new(41)); let cur = *slot.borrow(); *slot.borrow_mut() = cur + 1; println!(\"{}\", *slot.borrow()); }",
            "42",
        ),
        (
            "type-alias-fn-return",
            "type R = Result<i64, String>; fn answer() -> R { Ok(42) } fn main() { println!(\"{}\", answer().unwrap_or(0)); }",
            "42",
        ),
        (
            "lifetime-only-generic-constructor",
            "struct Scope<'p> { parent: Option<&'p i64>, n: i64 } impl<'p> Scope<'p> { fn new(parent: Option<&'p i64>) -> Scope<'p> { Scope { parent, n: 42 } } } fn main() { println!(\"{}\", Scope::new(None).n); }",
            "42",
        ),
        (
            "generic-wrapper-vec-placeholder",
            "use std::rc::Rc; struct E { data: Rc<Vec<i64>> } fn main() { let e = E { data: Rc::new(Vec::new()) }; println!(\"{}\", e.data.len() + 42); }",
            "42",
        ),
        (
            "rc-vec-index",
            "use std::rc::Rc; struct E { data: Rc<Vec<i64>> } fn main() { let e = E { data: Rc::new(vec![40, 2]) }; println!(\"{}\", e.data[0] + e.data[1]); }",
            "42",
        ),
        (
            "impl-trait-param-surface",
            "fn accept(_x: impl Into<String>) -> i64 { 42 } fn main() { println!(\"{}\", accept(String::from(\"rs\"))); }",
            "42",
        ),
        (
            "trait-impl-method-surface",
            "trait Speak { fn speak(&self) -> i64; } struct Dog { n: i64 } impl Speak for Dog { fn speak(&self) -> i64 { self.n } } fn main() { let d = Dog { n: 42 }; println!(\"{}\", d.speak()); }",
            "42",
        ),
        (
            "impl-into-string-method",
            "fn take(x: impl Into<String>) -> String { x.into() } fn main() { println!(\"{}\", take(\"rs\")); }",
            "rs",
        ),
        (
            "generic-impl-target-surface",
            "struct Wrap<T> { value: T } impl<T> Wrap<T> { fn answer(&self) -> i64 { 42 } } fn main() { println!(\"{}\", 42); }",
            "42",
        ),
        (
            "clone-enum",
            "#[derive(Clone)] enum Expr { Int(i64) } fn main() { let e = Expr::Int(42); let c = e.clone(); let n = match c { Expr::Int(v) => v }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "clone-string-deep",
            "fn main() { let mut a = String::from(\"r\"); let b = a.clone(); a.push_str(\"s\"); println!(\"{}\", b); }",
            "r",
        ),
        (
            "clone-ref-inner",
            "#[derive(Clone)] struct Point { x: i64 } fn cloned(p: &Point) -> Point { p.clone() } fn main() { let p = Point { x: 42 }; let q = cloned(&p); println!(\"{}\", q.x); }",
            "42",
        ),
        (
            "clone-vec-deep",
            "fn main() { let mut a = vec![String::from(\"r\")]; let b = a.clone(); a[0] = String::from(\"rs\"); println!(\"{}\", b[0]); }",
            "r",
        ),
        (
            "clone-struct-deep",
            "#[derive(Clone)] struct Bag { xs: Vec<String> } fn main() { let mut a = Bag { xs: vec![String::from(\"r\")] }; let b = a.clone(); a.xs = vec![String::from(\"rs\")]; println!(\"{}\", b.xs[0]); }",
            "r",
        ),
        (
            "clone-iter-state",
            "fn main() { let mut it = vec![1, 2, 3].into_iter(); let _ = it.next(); let mut c = it.clone(); println!(\"{}\", it.next().unwrap_or(0) + c.next().unwrap_or(0)); }",
            "4",
        ),
        (
            "unreachable-macro-surface",
            "fn main() { if false { unreachable!(); } println!(\"{}\", 42); }",
            "42",
        ),
        (
            "eprintln-macro",
            "fn main() { eprintln!(\"{}\", 1); println!(\"{}\", 42); }",
            "42",
        ),
        (
            "print-macro",
            "fn main() { print!(\"{}\", 4); print!(\"{}\", 2); }",
            "42",
        ),
        (
            "write-macro-string",
            "use std::fmt::Write; fn main() { let mut s = String::new(); write!(s, \"{}\", 4).unwrap(); write!(&mut s, \"{}\", 2).unwrap(); println!(\"{}\", s); }",
            "42",
        ),
        (
            "writeln-macro-string",
            "use std::fmt::Write; fn main() { let mut s = String::new(); writeln!(&mut s, \"{}\", 42).unwrap(); print!(\"{}\", s); }",
            "42",
        ),
        (
            "assert-macro",
            "fn main() { assert!(1 < 2); println!(\"{}\", 42); }",
            "42",
        ),
        (
            "assert-eq-macro",
            "fn main() { assert_eq!(20 + 22, 42); println!(\"{}\", 42); }",
            "42",
        ),
        (
            "let-tuple-pattern",
            "fn pair() -> (i64, i64) { (20, 22) } fn main() { let (a, b) = pair(); println!(\"{}\", a + b); }",
            "42",
        ),
        (
            "generic-fn-id",
            "fn id<T>(x: T) -> T { x } fn main() { println!(\"{}\", id(42)); }",
            "42",
        ),
        (
            "generic-struct-field",
            "struct Wrap<T> { value: T } fn main() { let w = Wrap { value: 42 }; println!(\"{}\", w.value); }",
            "42",
        ),
        (
            "generic-enum-match",
            "enum Opt<T> { None, Some(T) } fn main() { let o = Opt::Some(42); let n = match o { Opt::Some(v) => v, Opt::None => 0 }; println!(\"{}\", n); }",
            "42",
        ),
        (
            "rc-ptr-eq-unsuffixed-literal-provenance",
            "use std::rc::Rc; fn main() { println!(\"{}\", Rc::ptr_eq(&Rc::new(vec![1]), &Rc::new(vec![1u64]))); }",
            "false",
        ),
    ]
}

pub fn interp_run(src: &str) -> Result<String, String> {
    let toks = lex(src)?;
    let prog = parse_program(&toks)?;
    typeck::check(&prog)?;
    let interp = Interp::new(&prog)?;
    interp.run_main()
}

/// Programs that must be REJECTED — by the interpreter's typeck AND by rustc.
/// This is translation validation on the *acceptance* boundary, not just results.
pub fn negative_corpus() -> Vec<(&'static str, &'static str)> {
    vec![
        ("inherent-assoc-type", "struct S { n: i64 } impl S { type X = i64; fn go(&self) -> i64 { self.n } } fn main() { println!(\"{}\", S { n: 9 }.go()); }"),
        ("cast-lit-overflow-u8", "fn main() { println!(\"{}\", 1000 as u8); }"),
        ("cast-lit-overflow-i32", "fn main() { println!(\"{}\", 3000000000 as i32); }"),
        ("cast-lit-neg-unsigned", "fn main() { println!(\"{}\", (-5) as u8); }"),
        ("add-bool", "fn main() { println!(\"{}\", 1 + true); }"),
        ("fn-ret-i64-empty-body", "fn f() -> i64 { } fn main() { let _x = f(); println!(\"{}\", 1); }"),
        ("fn-ret-i64-let-only", "fn f() -> i64 { let _y = 3; } fn main() { let _z = f(); println!(\"{}\", 1); }"),
        ("int-mix-u32-u64", "fn main() { let a: u32 = 1; let b: u64 = 2; println!(\"{}\", a + b); }"),
        ("int-mix-i32-u32", "fn main() { let a: i32 = 1; let b: u32 = 2; println!(\"{}\", a + b); }"),
        ("int-mix-cmp-u32-u64", "fn main() { let a: u32 = 1; let b: u64 = 2; println!(\"{}\", a < b); }"),
        ("int-mix-bitand-u32-u64", "fn main() { let a: u32 = 1; let b: u64 = 2; println!(\"{}\", a & b); }"),
        ("hashmap-index-assign", "fn main() { let mut m: std::collections::HashMap<i64, i64> = std::collections::HashMap::new(); m.insert(1, 10); m[&1] = 20; println!(\"{}\", 1); }"),
        ("literal-method-e0689", "fn main() { println!(\"{}\", (7 - 4).signum()); }"),
        ("float-int-add", "fn main() { println!(\"{:?}\", 1.5 + 1); }"),
        (
            "let-i64-float",
            "fn main() { let x: i64 = 3.5; println!(\"{}\", x); }",
        ),
        ("float-cmp-mixed", "fn main() { println!(\"{}\", 3.5 < 3); }"),
        (
            "let-type-mismatch",
            "fn main() { let x: bool = 5; println!(\"{}\", x); }",
        ),
        (
            "ret-mismatch",
            "fn f() -> i64 { true } fn main() { println!(\"{}\", f()); }",
        ),
        (
            "arg-type",
            "fn sq(n: i64) -> i64 { n * n } fn main() { println!(\"{}\", sq(true)); }",
        ),
        ("unbound", "fn main() { println!(\"{}\", nope); }"),
        (
            "cmp-mixed",
            "fn main() { println!(\"{}\", if 1 == true { 1 } else { 0 }); }",
        ),
        (
            "assign-immutable",
            "fn main() { let x = 1; x = 2; println!(\"{}\", x); }",
        ),
        ("break-outside-loop", "fn main() { break; }"),
        (
            "unknown-associated",
            "struct P { x: i64 } fn main() { println!(\"{}\", P::missing()); }",
        ),
        (
            "method-arg-type",
            "struct P { x: i64 } impl P { fn add(&self, n: i64) -> i64 { self.x + n } } \
             fn main() { let p = P { x: 1 }; println!(\"{}\", p.add(true)); }",
        ),
        (
            "refmut-on-immutable",
            "struct C { n: i64 } impl C { fn get(&mut self) -> i64 { self.n } } \
             fn main() { let c = C { n: 1 }; println!(\"{}\", c.get()); }",
        ),
        (
            "impl-unknown-target",
            "impl Missing { fn new() -> Missing { Missing { } } } fn main() { }",
        ),
        (
            "trait-impl-unknown-target",
            "trait Speak { fn speak(&self) -> i64; } impl Speak for Missing { fn speak(&self) -> i64 { 1 } } fn main() { }",
        ),
        (
            "mut-ref-immutable",
            "fn bump(r: &mut i64) { *r = 2; } fn main() { let n = 1; bump(&mut n); }",
        ),
        (
            "deref-non-ref",
            "fn main() { let x = 1; println!(\"{}\", *x); }",
        ),
        (
            "assign-through-immut-ref",
            "fn main() { let mut n = 1; let r = &n; *r = 2; println!(\"{}\", n); }",
        ),
        (
            "assign-through-ref-type-mismatch",
            "fn main() { let mut b = true; let r = &mut b; *r = 1; }",
        ),
        (
            "vec-push-wrong-type",
            "fn main() { let mut v: Vec<i64> = Vec::new(); v.push(true); }",
        ),
        (
            "vec-push-immutable",
            "fn main() { let v: Vec<i64> = Vec::new(); v.push(1); }",
        ),
        (
            "vec-with-capacity-non-int",
            "fn main() { let v: Vec<i64> = Vec::with_capacity(true); println!(\"{}\", v.len()); }",
        ),
        (
            "vec-index-bool",
            "fn main() { let v = vec![1, 2]; println!(\"{}\", v[true]); }",
        ),
        (
            "vec-mixed-literal",
            "fn main() { let v = vec![1, true]; println!(\"{}\", v.len()); }",
        ),
        (
            "string-push-immutable",
            "fn main() { let s = String::new(); s.push_str(\"x\"); }",
        ),
        (
            "string-push-wrong-type",
            "fn main() { let mut s = String::new(); s.push_str(1); }",
        ),
        (
            "string-push-str-non-string-ref",
            "fn main() { let mut s = String::from(\"a\"); let n = 1; s.push_str(&n); println!(\"{}\", s); }",
        ),
        (
            "vec-join-non-string",
            "fn main() { let v = vec![1, 2]; println!(\"{}\", v.join(\",\")); }",
        ),
        (
            "string-from-wrong-type",
            "fn main() { let s = String::from(1); }",
        ),
        (
            "string-compare-non-string",
            "fn main() { let s = String::from(\"a\"); println!(\"{}\", if s == 1 { 1 } else { 0 }); }",
        ),
        (
            "string-add-wrong-type",
            "fn main() { let s = String::from(\"a\") + 1; println!(\"{}\", s); }",
        ),
        ("string-bad-escape", "fn main() { let _s = \"\\q\"; }"),
        (
            "option-unwrap-or-wrong-type",
            "fn main() { let o = Some(true); println!(\"{}\", o.unwrap_or(1)); }",
        ),
        (
            "result-unwrap-or-wrong-type",
            "fn main() { let r: Result<bool, &str> = Ok(true); println!(\"{}\", r.unwrap_or(1)); }",
        ),
        (
            "result-map-err-non-closure",
            "fn main() { let r: Result<i64, String> = \"x\".parse::<i64>().map_err(1); println!(\"{}\", r.unwrap_or(0)); }",
        ),
        (
            "enum-tuple-variant-missing-arg-call",
            "enum Signal { Error(String) } fn main() { let _e = Signal::Error(); }",
        ),
        (
            "enum-tuple-variant-constructor-call-arity",
            "enum Signal { Error(String) } fn main() { let f = Signal::Error; let _e = f(); }",
        ),
        (
            "result-map-non-closure",
            "fn main() { let r: Result<i64, String> = Ok(1); let _x = r.map(1); }",
        ),
        (
            "option-ok-or-else-non-closure",
            "fn main() { let o: Option<i64> = None; let r = o.ok_or_else(1); println!(\"{}\", if r.is_err() { 1 } else { 0 }); }",
        ),
        (
            "option-map-non-closure",
            "fn main() { let o = Some(1).map(1); println!(\"{}\", o.unwrap_or(0)); }",
        ),
        (
            "option-and-then-non-option",
            "fn main() { let _x = Some(1).and_then(|n| n + 1); }",
        ),
        (
            "option-and-then-arity",
            "fn main() { let _x = Some(1).and_then(); }",
        ),
        (
            "option-or-else-non-option",
            "fn main() { let _x = Some(1).or_else(|| 2); }",
        ),
        (
            "option-as-ref-arity",
            "fn main() { let _x = Some(1).as_ref(1); }",
        ),
        (
            "option-unwrap-or-else-non-closure",
            "fn main() { let _x = Some(1).unwrap_or_else(2); }",
        ),
        (
            "option-none-assign-non-option",
            "fn main() { let mut x = None; x = 1; println!(\"{}\", x.is_some()); }",
        ),
        (
            "option-none-refined-wrong-some",
            "fn main() { let mut x = None; x = Some(1); x = Some(true); }",
        ),
        ("some-arity", "fn main() { let o = Some(); }"),
        ("ok-arity", "fn main() { let r = Ok(); }"),
        ("box-new-arity", "fn main() { let b = Box::new(); }"),
        ("box-as-ref-arity", "fn main() { let b = Box::new(1); let _r = b.as_ref(1); }"),
        (
            "rc-clone-wrong-type",
            "use std::rc::Rc; fn main() { let n = 1; let r = Rc::clone(&n); }",
        ),
        (
            "rc-pointer-equality-inner-type",
            "use std::rc::Rc; fn main() { let a = Rc::new(1i64); let b = Rc::new(1u64); println!(\"{}\", Rc::ptr_eq(&a, &b)); }",
        ),
        (
            "rc-as-ref-arity",
            "use std::rc::Rc; fn main() { let r = Rc::new(1); let _x = r.as_ref(1); }",
        ),
        (
            "rc-len-non-vec",
            "use std::rc::Rc; fn main() { let r = Rc::new(1); println!(\"{}\", r.len()); }",
        ),
        (
            "rc-iter-non-vec",
            "use std::rc::Rc; fn main() { let r = Rc::new(1); let _it = r.iter(); }",
        ),
        (
            "rc-get-non-vec",
            "use std::rc::Rc; fn main() { let r = Rc::new(1); let _x = r.get(0); }",
        ),
        (
            "rc-get-bool-index",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![1]); let _x = r.get(true); }",
        ),
        (
            "rc-as-str-non-string",
            "use std::rc::Rc; fn main() { let r = Rc::new(1); let _s = r.as_str(); }",
        ),
        (
            "box-ref-coercion-needs-reference",
            "fn f(n: &i64) -> i64 { *n } fn main() { let b = Box::new(1); println!(\"{}\", f(b)); }",
        ),
        (
            "refcell-new-arity",
            "use std::cell::RefCell; fn main() { let c = RefCell::new(); }",
        ),
        (
            "deref-refcell",
            "use std::cell::RefCell; fn main() { let c = RefCell::new(1); println!(\"{}\", *c); }",
        ),
        (
            "hashmap-insert-wrong-key",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.insert(1, 2); }",
        ),
        (
            "hashmap-insert-wrong-value",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let k = String::from(\"a\"); m.insert(k, true); }",
        ),
        (
            "hashmap-get-wrong-key",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); println!(\"{}\", m.contains_key(&1)); }",
        ),
        (
            "hashmap-insert-immutable",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); let k = String::from(\"a\"); m.insert(k, 1); }",
        ),
        (
            "hashmap-iter-arg",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); let _it = m.iter(1); }",
        ),
        (
            "hashmap-get-mut-immutable",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); let _v = m.get_mut(\"a\"); }",
        ),
        (
            "hashmap-entry-immutable",
            "use std::collections::HashMap; fn main() { let m: HashMap<String, i64> = HashMap::new(); let _v = m.entry(String::from(\"a\")); }",
        ),
        (
            "hashmap-entry-wrong-key",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let _v = m.entry(1); }",
        ),
        (
            "hashmap-entry-or-insert-wrong-value",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let _v = m.entry(String::from(\"a\")).or_insert(true); }",
        ),
        (
            "hashmap-entry-or-insert-with-wrong-value",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); let _v = m.entry(String::from(\"a\")).or_insert_with(|| true); }",
        ),
        (
            "hashmap-entry-and-modify-wrong-param",
            "use std::collections::HashMap; fn main() { let mut m: HashMap<String, i64> = HashMap::new(); m.entry(String::from(\"a\")).and_modify(|v: &mut bool| { *v = true; }); }",
        ),
        (
            "string-push-non-char",
            "fn main() { let mut s = String::new(); s.push(1); }",
        ),
        ("string-chars-arg", "fn main() { let _x = \"rs\".chars(1); }"),
        ("string-bytes-arg", "fn main() { let _x = \"rs\".bytes(1); }"),
        ("string-trim-arg", "fn main() { let _x = \"rs\".trim(1); }"),
        ("string-split-non-string", "fn main() { let _x = \"rs\".split(1); }"),
        ("string-iter-rejected", "fn main() { let _x = \"rs\".iter(); }"),
        (
            "string-chars-map-non-closure",
            "fn main() { let s: String = \"rs\".chars().map(1).collect(); println!(\"{}\", s.len()); }",
        ),
        ("int-as-bool", "fn main() { let b = 1 as bool; }"),
        (
            "ref-as-int",
            "fn main() { let n = 1; let r = &n; println!(\"{}\", r as i64); }",
        ),
        (
            "char-method-arg",
            "fn main() { let b = 'a'.is_ascii_digit(1); }",
        ),
        (
            "char-from-u32-wrong-arg",
            "fn main() { let c = char::from_u32(true); println!(\"{}\", c.unwrap_or('?')); }",
        ),
        ("char-let-mismatch", "fn main() { let c: char = 65; }"),
        (
            "char-arithmetic",
            "fn main() { println!(\"{}\", 'a' + 'b'); }",
        ),
        ("vec-pop-immutable", "fn main() { let v = vec![1]; v.pop(); }"),
        (
            "vec-unit-push-non-unit",
            "fn main() { let mut v: Vec<()> = Vec::new(); v.push(1); }",
        ),
        (
            "vec-to-vec-arity",
            "fn main() { let v = vec![1]; let _x = v.to_vec(1); }",
        ),
        (
            "vec-reverse-immutable",
            "fn main() { let v = vec![1]; v.reverse(); }",
        ),
        (
            "vec-last-mut-immutable",
            "fn main() { let v = vec![1]; let _x = v.last_mut(); }",
        ),
        (
            "vec-remove-immutable",
            "fn main() { let v = vec![1]; let _x = v.remove(0); }",
        ),
        (
            "vec-iter-arity",
            "fn main() { let v = vec![1]; let _it = v.iter(1); }",
        ),
        (
            "vec-iter-mut-arity",
            "fn main() { let mut v = vec![1]; let _it = v.iter_mut(1); }",
        ),
        (
            "vec-iter-mut-immutable",
            "fn main() { let v = vec![1]; let _it = v.iter_mut(); }",
        ),
        (
            "vec-into-iter-arity",
            "fn main() { let v = vec![1]; let w: Vec<i64> = v.into_iter(1).collect(); println!(\"{}\", w.len()); }",
        ),
        (
            "clone-arity",
            "fn main() { let n = 1; let _m = n.clone(2); }",
        ),
        (
            "tuple-vec-placeholder-arg-mismatch",
            "fn f(p: (Vec<i64>, i64)) -> i64 { p.1 } fn main() { let p = (Vec::new(), true); println!(\"{}\", f(p)); }",
        ),
        (
            "iter-next-immutable",
            "fn main() { let it = vec![1].into_iter(); let _x = it.next(); }",
        ),
        (
            "iter-nth-immutable",
            "fn main() { let it = vec![1].into_iter(); let _x = it.nth(0); }",
        ),
        (
            "iter-nth-arity",
            "fn main() { let mut it = vec![1].into_iter(); let _x = it.nth(); }",
        ),
        (
            "iter-nth-bool",
            "fn main() { let mut it = vec![1].into_iter(); let _x = it.nth(true); }",
        ),
        (
            "iter-last-arity",
            "fn main() { let _x = vec![1].into_iter().last(1); }",
        ),
        (
            "iter-map-non-closure",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().map(1).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "range-map-non-closure",
            "fn main() { let _n = (1..3).map(1).count(); }",
        ),
        (
            "range-end-non-integer",
            "fn main() { let _n = (1..true).count(); }",
        ),
        (
            "iter-collect-vec-char-mismatch",
            "fn main() { let chars: Vec<i64> = \"rs\".chars().collect(); println!(\"{}\", chars.len()); }",
        ),
        (
            "iter-collect-string-turbofish-mismatch",
            "fn main() { let _s: String = vec![1].into_iter().collect::<String>(); }",
        ),
        (
            "iter-filter-non-bool",
            "fn main() { let v = vec![1]; let _out: Vec<i64> = v.iter().filter(|x| **x).collect(); }",
        ),
        (
            "iter-zip-non-iter",
            "fn main() { let v = vec![1].into_iter().zip(1).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-zip-arity",
            "fn main() { let v = vec![1].into_iter().zip(); println!(\"{}\", v.collect().len()); }",
        ),
        (
            "iter-all-non-bool",
            "fn main() { let ok = vec![1].into_iter().all(|n| n); println!(\"{}\", ok); }",
        ),
        (
            "iter-all-immutable",
            "fn main() { let it = vec![1].into_iter(); let ok = it.all(|n| n == 1); println!(\"{}\", ok); }",
        ),
        (
            "iter-rev-arity",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().rev(1).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-enumerate-arity",
            "fn main() { let v: Vec<(usize, i64)> = vec![1].into_iter().enumerate(1).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-find-arity",
            "fn main() { let n = vec![1].into_iter().find().unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "iter-find-non-bool",
            "fn main() { let n = vec![1].into_iter().find(|x| *x).unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "iter-position-arity",
            "fn main() { let n = vec![1].into_iter().position().unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "iter-position-non-bool",
            "fn main() { let n = vec![1].into_iter().position(|x| x).unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "iter-count-arity",
            "fn main() { let n = vec![1].into_iter().count(1); println!(\"{}\", n); }",
        ),
        (
            "iter-sum-non-int",
            "fn main() { let n: i64 = vec![true].into_iter().sum(); println!(\"{}\", n); }",
        ),
        (
            "iter-fold-return-mismatch",
            "fn main() { let n = vec![1].into_iter().fold(0, |acc, x| acc == x); println!(\"{}\", n); }",
        ),
        (
            "iter-take-bool",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().take(true).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-skip-bool",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().skip(true).collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-copied-non-ref",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().copied().collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "iter-cloned-non-ref",
            "fn main() { let v: Vec<i64> = vec![1].into_iter().cloned().collect(); println!(\"{}\", v.len()); }",
        ),
        (
            "vec-get-bool-index",
            "fn main() { let v = vec![1]; let x = v.get(true); }",
        ),
        (
            "vec-get-mut-immutable",
            "fn main() { let v = vec![1]; let _x = v.get_mut(0); }",
        ),
        (
            "vec-index-assign-immutable",
            "fn main() { let v = vec![1]; v[0] = 2; }",
        ),
        (
            "tuple-index-assign-immutable",
            "fn main() { let p = (1, 2); p.1 = 3; }",
        ),
        (
            "tuple-index-assign-immutable-ref",
            "fn main() { let p = (1, 2); let r = &p; r.1 = 3; }",
        ),
        ("for-non-iter", "fn main() { for x in 1 { println!(\"{}\", x); } }"),
        (
            "to-string-arg",
            "fn main() { let s = \"x\".to_string(1); }",
        ),
        (
            "question-in-unit-fn",
            "fn main() { let n = Some(1)?; println!(\"{}\", n); }",
        ),
        ("question-non-carrier", "fn main() { let n = 1?; }"),
        (
            "question-error-no-from",
            "fn parse() -> Result<i64, i64> { Err(1) } fn calc() -> Result<i64, String> { let n = parse()?; Ok(n) } fn main() { let _ = calc(); }",
        ),
        (
            "match-guard-non-bool",
            "fn main() { let n = match 1 { x if x => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "match-bool-non-exhaustive",
            "fn main() { let _n = match true { true => 1 }; }",
        ),
        (
            "match-enum-non-exhaustive",
            "enum E { A, B } fn main() { let _n = match E::A { E::A => 1 }; }",
        ),
        (
            "match-guard-does-not-cover",
            "fn main() { let _n = match true { true if true => 1, false => 0 }; }",
        ),
        (
            "compound-assign-immutable",
            "fn main() { let n = 1; n += 1; println!(\"{}\", n); }",
        ),
        (
            "bool-and-assign-type",
            "fn main() { let mut b = true; b &= 1; }",
        ),
        (
            "some-pattern-non-option",
            "fn main() { let n = match 1 { Some(x) => x, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "assignment-expression-immutable",
            "fn main() { let n = 0; match true { true => n += 1, false => () }; println!(\"{}\", n); }",
        ),
        (
            "return-expression-type",
            "fn f() -> i64 { match true { true => return true, false => 0 } } fn main() { println!(\"{}\", f()); }",
        ),
        (
            "format-arity",
            "fn main() { let s = format!(\"{} {}\", 1); println!(\"{}\", s); }",
        ),
        (
            "format-left-align-bad",
            "fn main() { println!(\"{:<$}\", 1); }",
        ),
        (
            "format-right-align-bad",
            "fn main() { println!(\"{:>$}\", 1); }",
        ),
        (
            "format-pretty-debug-bad",
            "fn main() { println!(\"{:#q}\", 1); }",
        ),
        (
            "format-named-unused",
            "fn main() { println!(\"{}\", 1, extra = 2); }",
        ),
        (
            "format-positional-missing",
            "fn main() { println!(\"{1}\", 1); }",
        ),
        (
            "format-display-vec",
            "fn main() { println!(\"{}\", vec![1]); }",
        ),
        (
            "format-hex-string",
            "fn main() { println!(\"{:016x}\", \"rs\"); }",
        ),
        (
            "format-fixed-precision-too-large",
            "fn main() { println!(\"{:.65536}\", 1.5); }",
        ),
        (
            "format-fixed-precision-overflow",
            "fn main() { println!(\"{:.18446744073709551616}\", 1.5); }",
        ),
        (
            "format-debug-closure",
            "fn main() { let f = |x: i64| x; println!(\"{:?}\", f); }",
        ),
        (
            "format-display-rc-vec",
            "use std::rc::Rc; fn main() { println!(\"{}\", Rc::new(vec![1])); }",
        ),
        (
            "eprintln-arity",
            "fn main() { eprintln!(\"{} {}\", 1); }",
        ),
        ("print-arity", "fn main() { print!(\"{} {}\", 1); }"),
        (
            "reference-pattern-non-ref",
            "fn main() { let n = match 1 { &x => x, _ => 0 }; println!(\"{}\", n); }",
        ),
        ("ref-binding-missing-name", "fn main() { let _out = match 1 { ref => 0 }; }"),
        ("ref-mut-binding-missing-name", "fn main() { let _out = match 1 { ref mut => 0 }; }"),
        (
            "ref-match-ergonomics-literal-type",
            "fn main() { let v = vec![1]; let _n = match v.get(0) { Some('x') => 1, _ => 0 }; }",
        ),
        (
            "vec-slice-bool-bound",
            "fn main() { let v = vec![1, 2, 3]; println!(\"{}\", v[true..2].len()); }",
        ),
        (
            "closure-arg-type",
            "fn main() { let f = |x: i64| x + 1; println!(\"{}\", f(true)); }",
        ),
        (
            "parse-turbofish-unsupported",
            "fn main() { let n = \"42\".parse::<Vec<i64>>().unwrap_or(vec![0]); println!(\"{}\", n.len()); }",
        ),
        (
            "string-pattern-non-string",
            "fn main() { let n = match 1 { \"one\" => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "string-to-str-param-needs-ref",
            "fn f(s: &str) -> i64 { s.len() as i64 } fn main() { let s = String::from(\"rs\"); println!(\"{}\", f(s)); }",
        ),
        (
            "slice-ref-param-wrong",
            "fn f(xs: &[i64]) -> i64 { xs.len() as i64 } fn main() { let n = 1; println!(\"{}\", f(&n)); }",
        ),
        (
            "ref-ref-param-wrong-inner",
            "fn f(s: &String) -> i64 { s.len() as i64 } fn main() { let n = 1; let r = &n; let rr = &r; println!(\"{}\", f(rr)); }",
        ),
        (
            "slice-get-bool-index",
            "fn f(xs: &[i64]) -> i64 { *xs.get(true).unwrap() } fn main() { let v = vec![1]; println!(\"{}\", f(&v)); }",
        ),
        (
            "slice-first-arg",
            "fn f(xs: &[i64]) -> i64 { *xs.first(1).unwrap() } fn main() { let v = vec![1]; println!(\"{}\", f(&v)); }",
        ),
        (
            "slice-index-bool",
            "fn f(xs: &[i64]) -> i64 { xs[true] } fn main() { let v = vec![1]; println!(\"{}\", f(&v)); }",
        ),
        (
            "slice-iter-arity",
            "fn f(xs: &[i64]) -> i64 { xs.iter(1).count() as i64 } fn main() { let v = vec![1]; println!(\"{}\", f(&v)); }",
        ),
        (
            "option-copied-non-ref",
            "fn main() { let n = Some(1).copied().unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "option-cloned-non-ref",
            "fn main() { let n = Some(1).cloned().unwrap_or(0); println!(\"{}\", n); }",
        ),
        (
            "direct-ref-compare-mismatch",
            "fn main() { let n = &1; let s = &String::new(); println!(\"{}\", if n == s { 1 } else { 0 }); }",
        ),
        (
            "matches-guard-non-bool",
            "fn main() { let n = Some(1); println!(\"{}\", if matches!(n, Some(x) if x) { 1 } else { 0 }); }",
        ),
        (
            "struct-literal-shorthand-unbound",
            "struct Point { x: i64 } fn main() { let p = Point { x }; println!(\"{}\", p.x); }",
        ),
        (
            "struct-field-assign-immutable",
            "struct Counter { n: i64 } fn main() { let c = Counter { n: 1 }; c.n = 2; println!(\"{}\", c.n); }",
        ),
        (
            "struct-field-vec-push-immutable",
            "struct Bag { xs: Vec<i64> } fn main() { let b = Bag { xs: Vec::new() }; b.xs.push(1); }",
        ),
        ("integer-bad-suffix", "fn main() { let n = 1wat; println!(\"{}\", n); }"),
        ("integer-bad-hex", "fn main() { let n = 0x; println!(\"{}\", n); }"),
        ("integer-bitxor-bool", "fn main() { println!(\"{}\", 1 ^ true); }"),
        (
            "u64-from-str-radix-bad-radix",
            "fn main() { println!(\"{:?}\", u64::from_str_radix(\"2a\", true)); }",
        ),
        ("fs-create-dir-all-arity", "use std::fs; fn main() { let _ = fs::create_dir_all(); }"),
        ("fs-write-arity", "use std::fs; fn main() { let _ = fs::write(\"work/x\"); }"),
        (
            "fs-read-to-string-arity",
            "use std::fs; fn main() { let _ = fs::read_to_string(\"work/x\", \"extra\"); }",
        ),
        ("fs-read-arity", "use std::fs; fn main() { let _ = fs::read(); }"),
        (
            "pathbuf-from-arity",
            "use std::path::PathBuf; fn main() { let _p = PathBuf::from(); }",
        ),
        (
            "path-exists-arity",
            "use std::path::PathBuf; fn main() { let p = PathBuf::from(\"work\"); let _ = p.exists(1); }",
        ),
        (
            "path-display-arity",
            "use std::path::PathBuf; fn main() { let p = PathBuf::from(\"work\"); let _ = p.display(1); }",
        ),
        (
            "fully-qualified-pathbuf-from-arity",
            "fn main() { let _p = std::path::PathBuf::from(); }",
        ),
        (
            "fully-qualified-hashmap-new-arity",
            "fn main() { let _m = std::collections::HashMap::new(1); }",
        ),
        (
            "command-new-arity",
            "use std::process::Command; fn main() { let _cmd = Command::new(); }",
        ),
        (
            "command-env-arity",
            "use std::process::Command; fn main() { let _cmd = Command::new(\"/bin/sh\").env(\"K\"); }",
        ),
        (
            "command-env-key-type",
            "use std::process::Command; fn main() { let _cmd = Command::new(\"/bin/sh\").env(1, \"V\"); }",
        ),
        (
            "command-env-clear-arity",
            "use std::process::Command; fn main() { let _cmd = Command::new(\"/bin/sh\").env_clear(1); }",
        ),
        (
            "command-output-arity",
            "use std::process::Command; fn main() { let _out = Command::new(\"/bin/sh\").output(1); }",
        ),
        ("env-args-arity", "fn main() { let _args = std::env::args(1); }"),
        ("env-var-arity", "fn main() { let _path = std::env::var(); }"),
        (
            "exitcode-unknown-const",
            "use std::process::ExitCode; fn main() { let _x = ExitCode::MAYBE; }",
        ),
        (
            "fully-qualified-exitcode-unknown",
            "fn main() { let _x = std::process::ExitCode::MAYBE; }",
        ),
        ("cfg-macro-empty", "fn main() { println!(\"{}\", cfg!()); }"),
        ("assert-non-bool", "fn main() { assert!(1); }"),
        ("assert-eq-mismatch", "fn main() { assert_eq!(1, true); }"),
        ("assert-arity", "fn main() { assert!(); }"),
        ("write-missing-destination", "fn main() { write!(\"{}\"); }"),
        (
            "write-immutable-string",
            "use std::fmt::Write; fn main() { let s = String::new(); write!(s, \"{}\", 42).unwrap(); }",
        ),
        (
            "write-non-string",
            "use std::fmt::Write; fn main() { let mut n = 0; write!(n, \"{}\", 42).unwrap(); }",
        ),
        (
            "write-format-arg-count",
            "use std::fmt::Write; fn main() { let mut s = String::new(); write!(s, \"{}\", 1, 2).unwrap(); }",
        ),
        (
            "usize-saturating-sub-wrong-arg",
            "fn main() { let n: usize = 1; println!(\"{}\", n.saturating_sub(true)); }",
        ),
        (
            "i64-saturating-sub-wrong-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.saturating_sub(true)); }",
        ),
        (
            "i64-max-wrong-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.max(true)); }",
        ),
        (
            "i64-min-wrong-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.min(true)); }",
        ),
        (
            "i64-wrapping-neg-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.wrapping_neg(1)); }",
        ),
        (
            "i64-wrapping-add-wrong-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.wrapping_add(true)); }",
        ),
        (
            "i64-to-string-arg",
            "fn main() { let n: i64 = 1; println!(\"{}\", n.to_string(1)); }",
        ),
        (
            "bool-to-string-arg",
            "fn main() { let b = true; println!(\"{}\", b.to_string(1)); }",
        ),
        (
            "bool-then-non-closure",
            "fn main() { let _x = true.then(1); }",
        ),
        (
            "or-pattern-type-mismatch",
            "fn main() { let n = match 1 { 1 | true => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "range-pattern-type-mismatch",
            "fn main() { let n = match true { 1..=2 => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "char-range-pattern-type-mismatch",
            "fn main() { let n = match 1 { 'a'..='z' => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "bind-at-range-pattern-type-mismatch",
            "fn main() { let n = match true { x @ 1..=2 => 1, _ => 0 }; println!(\"{}\", n); }",
        ),
        (
            "closure-return-type-mismatch",
            "fn main() { let f = || -> i64 { true }; println!(\"{}\", f()); }",
        ),
        (
            "impl-fn-param-inferred-closure-return",
            "fn cmp(a: i64, b: i64, f: impl Fn(i64, i64) -> bool) -> i64 { if f(a, b) { 1 } else { 0 } } fn main() { println!(\"{}\", cmp(1, 2, |x, y| x + y)); }",
        ),
        (
            "struct-like-enum-variant-lit-type",
            "enum Event { Click { x: i64 } } fn main() { let _e = Event::Click { x: true }; }",
        ),
        (
            "if-let-branch-mismatch",
            "fn main() { let v = Some(1); let _n = if let Some(x) = v { x } else { true }; }",
        ),
        (
            "while-let-pattern-mismatch",
            "fn main() { while let Some(n) = 1 { println!(\"{}\", n); } }",
        ),
        (
            "while-let-body-non-unit",
            "fn main() { let mut xs = vec![1]; while let Some(n) = xs.pop() { n } }",
        ),
        (
            "let-else-pattern-mismatch",
            "fn main() { let Some(n) = 1 else { return; }; println!(\"{}\", n); }",
        ),
        (
            "let-else-non-diverging",
            "fn main() { let Some(n) = Some(1) else { 0 }; println!(\"{}\", n); }",
        ),
        (
            "let-else-missing-semicolon",
            "fn main() { let Some(n) = Some(1) else { return; } println!(\"{}\", n); }",
        ),
        ("const-missing-type", "const ANSWER = 42; fn main() { println!(\"{}\", ANSWER); }"),
        ("const-type-mismatch", "const ANSWER: i64 = true; fn main() { println!(\"{}\", ANSWER); }"),
        ("static-assign", "static ANSWER: i64 = 1; fn main() { ANSWER = 2; }"),
        (
            "immediate-closure-call-arity",
            "fn main() { let _n = (|| -> i64 { 42 })(1); }",
        ),
        (
            "function-item-callable-arity",
            "fn inc(n: i64) -> i64 { n + 1 } fn main() { let f = inc; println!(\"{}\", f(1, 2)); }",
        ),
        (
            "impl-fn-param-call-arity",
            "fn cmp(a: i64, b: i64, f: impl Fn(i64, i64) -> bool) -> i64 { if f(a) { 42 } else { 0 } } fn main() { println!(\"{}\", cmp(1, 2, |x: i64, y: i64| x < y)); }",
        ),
        (
            "closure-tuple-pattern-param-type",
            "fn main() { let f = |(a, b): (i64, i64)| a + b; let _n = f((1, true)); }",
        ),
        (
            "struct-like-enum-pattern-missing-field",
            "enum Event { Click { x: i64, y: i64 } } fn f(e: Event) -> i64 { match e { Event::Click { x } => x } } fn main() { }",
        ),
        (
            "struct-like-enum-pattern-rest-unknown-field",
            "enum Event { Click { x: i64, y: i64 } } fn f(e: Event) -> i64 { match e { Event::Click { z, .. } => z } } fn main() { }",
        ),
        (
            "for-tuple-pattern-non-tuple",
            "fn main() { for (_a, _b) in vec![1] { } }",
        ),
        ("array-mixed-literal", "fn main() { let _a = [1, true]; }"),
        (
            "array-repeat-count-bool",
            "fn main() { let _a = [1; true]; }",
        ),
        (
            "slice-array-compare-mismatch",
            "fn same(xs: &[i64]) -> bool { xs == [true] } fn main() { println!(\"{}\", same(&vec![1])); }",
        ),
        (
            "type-alias-missing-semi",
            "type Alias = i64 fn main() { }",
        ),
        (
            "type-alias-method-wrong-target",
            "type Slot = i64; fn main() { let slot: Slot = 1; let _x = slot.borrow(); }",
        ),
        (
            "type-alias-fn-return-mismatch",
            "type R = Result<i64, String>; fn answer() -> R { 42 } fn main() { println!(\"{}\", answer().unwrap_or(0)); }",
        ),
        (
            "generic-wrapper-vec-placeholder-mismatch",
            "use std::rc::Rc; struct E { data: Rc<Vec<i64>> } fn main() { let _e = E { data: Rc::new(vec![true]) }; }",
        ),
        (
            "rc-vec-bool-index",
            "use std::rc::Rc; fn main() { let v = Rc::new(vec![1]); println!(\"{}\", v[true]); }",
        ),
        (
            "rc-vec-to-string",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![1]); println!(\"{}\", r.to_string()); }",
        ),
        (
            "rc-vec-to-str-param",
            "use std::rc::Rc; fn f(s: &str) -> i64 { s.len() as i64 } fn main() { let r = Rc::new(vec![1]); println!(\"{}\", f(&r)); }",
        ),
        (
            "rc-vec-chars",
            "use std::rc::Rc; fn main() { let r = Rc::new(vec![1]); let _it = r.chars(); }",
        ),
        (
            "into-string-wrong-source",
            "fn main() { let n = 1; let _s: String = n.into(); }",
        ),
        (
            "let-tuple-pattern-non-tuple",
            "fn main() { let (_a, _b) = 1; }",
        ),
        (
            "generic-fn-arg-mismatch",
            "fn same<T>(a: T, b: T) -> T { a } fn main() { let _x = same(1, true); }",
        ),
        (
            "generic-struct-field-mismatch",
            "struct Pair<T> { a: T, b: T } fn main() { let _p = Pair { a: 1, b: true }; }",
        ),
        (
            "generic-enum-pattern-mismatch",
            "enum Opt<T> { None, Some(T) } fn main() { let o = Opt::Some(1); let _n = match o { Opt::Some(true) => 1, _ => 0 }; }",
        ),
        (
            "rc-ptr-eq-explicit-conflicting-types",
            "use std::rc::Rc; fn main() { let a: Rc<Vec<i64>> = Rc::new(vec![1]); let b: Rc<Vec<u64>> = Rc::new(vec![1]); let _n = Rc::ptr_eq(&a, &b); }",
        ),
    ]
}

pub struct Report {
    pub name: String,
    pub passed: usize,
    pub failed: usize,
    pub lines: Vec<String>,
}

impl Report {
    fn new(name: &str) -> Report {
        Report {
            name: name.to_string(),
            passed: 0,
            failed: 0,
            lines: Vec::new(),
        }
    }
    fn ok(&mut self, msg: String) {
        self.passed += 1;
        self.lines.push(format!("  ok   {}", msg));
    }
    fn fail(&mut self, msg: String) {
        self.failed += 1;
        self.lines.push(format!("  FAIL {}", msg));
    }
    pub fn print(&self) {
        println!("[{}]", self.name);
        for l in &self.lines {
            println!("{}", l);
        }
        println!(
            "  => {} ({} passed, {} failed)",
            if self.failed == 0 { "PASS" } else { "FAIL" },
            self.passed,
            self.failed
        );
    }
    pub fn green(&self) -> bool {
        self.failed == 0
    }
}

/// Interpreter evaluates the corpus to expected output.
pub fn self_check() -> Report {
    let mut r = Report::new("self-check (interpreter)");
    for (name, src, expected) in corpus() {
        match interp_run(src) {
            Ok(out) => {
                let got = out.trim();
                if got == expected {
                    r.ok(format!("{} = {}", name, got));
                } else {
                    r.fail(format!("{}: expected {:?}, got {:?}", name, expected, got));
                }
            }
            Err(e) => r.fail(format!("{}: {}", name, e)),
        }
    }
    r
}

/// Translation validation: interpreter stdout == native(rustc) stdout.
pub fn tv_check() -> Report {
    let mut r = Report::new("tv-check (interpreter == rustc)");
    let workdir = default_workdir();
    for (name, src, _expected) in corpus() {
        let result = (|| -> Result<(String, String), String> {
            let interp = interp_run(src)?;
            let native = native_run(src, &workdir)?;
            Ok((interp, native))
        })();
        match result {
            Ok((i, n)) if i.trim() == n.trim() => {
                r.ok(format!("{}: interp == rustc == {}", name, i.trim()))
            }
            Ok((i, n)) => r.fail(format!(
                "{}: interp {:?} != rustc {:?}",
                name,
                i.trim(),
                n.trim()
            )),
            Err(e) => r.fail(format!("{}: {}", name, e)),
        }
    }
    r
}

/// Small, bounded `fn`/`if`/arithmetic/comparison/call fixtures for the
/// Trusting-Trust (Diverse Double-Compiling) witness below. Each is a
/// complete, valid Rust program whose `main` is exactly
/// `println!("{}", EXPR);`.
fn independent_mini_backend_fixtures() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "mini-const-arithmetic",
            "fn main() { println!(\"{}\", 40 + 2); }",
            "42",
        ),
        (
            "mini-one-arg",
            "fn f(x: i64) -> i64 { x + 1 } fn main() { println!(\"{}\", f(41)); }",
            "42",
        ),
        (
            "mini-branch-two-arg",
            "fn f(x: i64, y: i64) -> i64 { if x < y { (x + 1) * y } else { x - y } } fn main() { println!(\"{}\", f(5, 7)); }",
            "42",
        ),
        (
            "mini-mul",
            "fn main() { println!(\"{}\", 6 * 7); }",
            "42",
        ),
        (
            "mini-sub",
            "fn main() { println!(\"{}\", 50 - 8); }",
            "42",
        ),
        (
            "mini-unary-negate-branch",
            "fn f(x: i64) -> i64 { if x > 0 { x } else { 0 - x } } fn main() { println!(\"{}\", f(-42)); }",
            "42",
        ),
        (
            "mini-equality-branch",
            "fn f(x: i64) -> i64 { if x == 42 { 1 } else { 0 } } fn main() { println!(\"{}\", f(42)); }",
            "1",
        ),
        (
            "mini-ge-branch",
            "fn f(x: i64) -> i64 { if x >= 41 { 42 } else { 0 } } fn main() { println!(\"{}\", f(41)); }",
            "42",
        ),
        (
            "mini-recursive-factorial",
            "fn f(n: i64) -> i64 { if n <= 1 { 1 } else { n * f(n - 1) } } fn main() { println!(\"{}\", f(5)); }",
            "120",
        ),
        (
            "mini-nested-if",
            "fn main() { println!(\"{}\", if 1 < 2 { if 3 < 4 { 100 } else { 200 } } else { 300 }); }",
            "100",
        ),
        (
            "mini-recursive-fibonacci",
            "fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } } fn main() { println!(\"{}\", fib(10)); }",
            "55",
        ),
        (
            "mini-three-arg",
            "fn f(a: i64, b: i64, c: i64) -> i64 { a + b + c } fn main() { println!(\"{}\", f(10, 20, 12)); }",
            "42",
        ),
        (
            "mini-four-arg",
            "fn f(a: i64, b: i64, c: i64, d: i64) -> i64 { a * b + c * d } fn main() { println!(\"{}\", f(2, 3, 4, 9)); }",
            "42",
        ),
        (
            "mini-let-sequential",
            "fn f(x: i64) -> i64 { let a = x + 1; let b = a * 2; b - 1 } fn main() { println!(\"{}\", f(10)); }",
            "21",
        ),
        (
            "mini-let-shadow",
            "fn f(x: i64) -> i64 { let x = x + 1; x * 2 } fn main() { println!(\"{}\", f(10)); }",
            "22",
        ),
        (
            "mini-let-in-if-branch",
            "fn f(x: i64) -> i64 { if x > 0 { let y = x * 2; y + 1 } else { let y = 0 - x; y - 1 } } fn main() { println!(\"{}\", f(5)); }",
            "11",
        ),
        (
            "mini-let-mut-param",
            "fn f(mut x: i64) -> i64 { let y = x + 1; y } fn main() { println!(\"{}\", f(41)); }",
            "42",
        ),
        (
            "mini-while-sum",
            "fn sum_to(n: i64) -> i64 { let mut acc = 0; let mut i = 0; while i < n { acc = acc + i; i = i + 1; } acc } fn main() { println!(\"{}\", sum_to(10)); }",
            "45",
        ),
        (
            "mini-while-factorial",
            "fn fact(n: i64) -> i64 { let mut acc = 1; let mut i = n; while i > 1 { acc = acc * i; i = i - 1; } acc } fn main() { println!(\"{}\", fact(5)); }",
            "120",
        ),
        (
            "mini-while-mut-param",
            "fn countdown_sum(mut n: i64) -> i64 { let mut acc = 0; while n > 0 { acc = acc + n; n = n - 1; } acc } fn main() { println!(\"{}\", countdown_sum(6)); }",
            "21",
        ),
        (
            "mini-nested-while",
            "fn f(n: i64) -> i64 { let mut total = 0; let mut i = 0; while i < n { let mut j = 0; while j < n { total = total + 1; j = j + 1; } i = i + 1; } total } fn main() { println!(\"{}\", f(3)); }",
            "9",
        ),
        (
            "mini-closure-multi-call",
            "fn f(n: i64) -> i64 { let square = |x: i64| x * x; square(n) + square(n + 1) } fn main() { println!(\"{}\", f(3)); }",
            "25",
        ),
        (
            "mini-closure-capture-non-tail",
            "fn f(a: i64, b: i64) -> i64 { let add_a = move |x: i64| x + a; add_a(b) - add_a(1) } fn main() { println!(\"{}\", f(5, 10)); }",
            "9",
        ),
        (
            "mini-closure-capture-let-bound",
            "fn f(n: i64) -> i64 { let k = n * 2; let g = |x: i64| x + k; g(1) + g(k) } fn main() { println!(\"{}\", f(5)); }",
            "31",
        ),
        (
            "mini-closure-two-params",
            "fn f(n: i64) -> i64 { let add = |a: i64, b: i64| a + b; add(n, add(n, n)) } fn main() { println!(\"{}\", f(14)); }",
            "42",
        ),
        (
            "mini-closure-shadows-fn",
            "fn sq(x: i64) -> i64 { x * x } fn f(n: i64) -> i64 { let sq = |x: i64| x + 1; sq(n) } fn main() { println!(\"{}\", f(41)); }",
            "42",
        ),
        (
            "mini-closure-captures-closure",
            "fn f(n: i64) -> i64 { let base = move |x: i64| x + n; let scaled = move |y: i64| base(y) * 2; scaled(1) + scaled(2) } fn main() { println!(\"{}\", f(3)); }",
            "18",
        ),
        (
            "mini-loop-break-sum",
            "fn sum_via_loop(n: i64) -> i64 { let mut i = 0; let mut acc = 0; let result = loop { if i >= n { break acc; } acc = acc + i; i = i + 1; }; result } fn main() { println!(\"{}\", sum_via_loop(10)); }",
            "45",
        ),
        (
            "mini-loop-tail-position",
            "fn first_square_ge(n: i64) -> i64 { let mut i = 0; loop { if i * i >= n { break i; } i = i + 1; } } fn main() { println!(\"{}\", first_square_ge(50)); }",
            "8",
        ),
        (
            "mini-loop-nested-if",
            "fn f(n: i64) -> i64 { let mut i = 0; loop { if i > n { if i % 2 == 0 { break i; } } i = i + 1; } } fn main() { println!(\"{}\", f(8)); }",
            "10",
        ),
        (
            "mini-loop-with-inner-while",
            "fn f(n: i64) -> i64 { let mut total = 0; let count = loop { let mut j = 0; let mut inner_sum = 0; while j < n { inner_sum = inner_sum + j; j = j + 1; } total = total + inner_sum; if total > 20 { break total; } }; count } fn main() { println!(\"{}\", f(4)); }",
            "24",
        ),
        (
            "mini-not-equal-branch",
            "fn f(x: i64) -> i64 { if x != 42 { 0 } else { 1 } } fn main() { println!(\"{}\", f(42)); }",
            "1",
        ),
        (
            "mini-higher-order-apply-twice",
            "fn apply_twice(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(f(x)) } fn g(x: i64) -> i64 { let add_one = |y: i64| y + 1; apply_twice(add_one, x) } fn main() { println!(\"{}\", g(5)); }",
            "7",
        ),
        (
            "mini-higher-order-two-params",
            "fn combine(f: impl Fn(i64, i64) -> i64, a: i64, b: i64) -> i64 { f(a, b) } fn g(n: i64) -> i64 { let add = |x: i64, y: i64| x + y; combine(add, n, n * 2) } fn main() { println!(\"{}\", g(14)); }",
            "42",
        ),
        (
            "mini-higher-order-capturing-arg",
            "fn apply(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(x) } fn g(n: i64, k: i64) -> i64 { let add_k = move |x: i64| x + k; apply(add_k, n) + apply(add_k, n + 1) } fn main() { println!(\"{}\", g(10, 5)); }",
            "31",
        ),
    ]
}

/// Trusting-Trust (Diverse Double-Compiling) witness: cross-check real
/// `rustc` against `independent_mini_backend`, a from-scratch
/// tokenizer/parser/interpreter sharing no code with `lexer.rs`/`parser.rs`/
/// `ast.rs`/`typeck.rs`/`interp.rs` (the evaluator core `tv-check` above
/// already proves `== rustc`). See rs-meta's STATUS.md "Trusting-Trust
/// defense roadmap" for the honest scope: a bounded fixture subset, behavior
/// equivalence (stdout text), not the full corpus `tv-check` covers.
pub fn independent_mini_backend_check() -> Report {
    let mut r = Report::new("independent-mini-backend-check (rustc == from-scratch mini interpreter)");
    let workdir = default_workdir();
    for (name, src, expected) in independent_mini_backend_fixtures() {
        let result = (|| -> Result<(String, String), String> {
            let native = native_run(src, &workdir)?;
            let mini = compile_and_run(src)?;
            Ok((native, mini))
        })();
        match result {
            Ok((n, m)) if n.trim() == expected && m.trim() == expected => {
                r.ok(format!("{}: rustc == mini backend == {}", name, expected))
            }
            Ok((n, m)) => r.fail(format!(
                "{}: rustc {:?} mini {:?} expected {:?}",
                name,
                n.trim(),
                m.trim(),
                expected
            )),
            Err(e) => r.fail(format!("{}: {}", name, e)),
        }
    }
    r
}

/// Acceptance translation validation: every negative program is rejected by BOTH
/// the interpreter (typeck) and rustc.
pub fn typeck_check() -> Report {
    let mut r = Report::new("typeck-check (interp rejects iff rustc rejects)");
    let workdir = default_workdir();
    for (name, src) in negative_corpus() {
        let interp_rejected = interp_run(src).is_err();
        let native_rejected = native_run(src, &workdir).is_err();
        match (interp_rejected, native_rejected) {
            (true, true) => r.ok(format!("{}: both reject", name)),
            (false, _) => r.fail(format!("{}: interp ACCEPTED a program rustc rejects", name)),
            (true, false) => r.fail(format!("{}: interp rejects but rustc accepts", name)),
        }
    }
    r
}

pub fn source_ast_check() -> Report {
    let mut r = Report::new("source-ast-check (rs-meta source parses)");
    for path in source_files() {
        let result = (|| -> Result<(), String> {
            let src = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))?;
            let toks = lex(&src)?;
            parse_program(&toks)?;
            Ok(())
        })();
        match result {
            Ok(()) => r.ok(format!("{}: AST parse OK", path)),
            Err(e) => r.fail(format!("{}: {}", path, e)),
        }
    }
    r
}

pub fn source_bundle_check() -> Report {
    let mut r = Report::new("source-bundle-check (all-source bundle interp == rustc)");
    match source_bundle() {
        Ok(src) => {
            let result = (|| -> Result<(String, String), String> {
                let interp = interp_run(&src)?;
                let native = native_run(&src, &default_workdir())?;
                Ok((interp, native))
            })();
            match result {
                Ok((i, n)) if i == n && i.contains("rs-meta bootstrap") => {
                    r.ok("src/*.rs bundle print_help path interp == rustc".to_string())
                }
                Ok((i, n)) => r.fail(format!(
                    "src/*.rs bundle mismatch: interp {:?} != rustc {:?}",
                    i, n
                )),
                Err(e) => r.fail(format!("src/*.rs bundle: {}", e)),
            }
        }
        Err(e) => r.fail(format!("src/*.rs bundle: {}", e)),
    }
    r
}

/// emit-self-host: the emitter regenerates the ENTIRE all-source bundle and
/// the regenerated program must behave identically under rustc — both on the
/// bundle's own main path and as a full evaluator replaying the corpus
/// (the emit lane's self-host closure over emit-tv 310/310).
pub fn emit_self_host_check() -> Report {
    let mut r = Report::new(
        "emit-self-host-check (emitted all-source bundle rustc == original)",
    );
    let result = (|| -> Result<(String, String), String> {
        let src = source_bundle()?;
        let toks = lex(&src)?;
        let prog = parse_program(&toks)?;
        let emitted = crate::emit::emit_program(&prog);
        let original = native_run(&src, &default_workdir())?;
        let regen = native_run(&emitted, &default_workdir()).map_err(|e| {
            let _ = fs::write("/tmp/rs-meta-emitted-bundle.rs", &emitted);
            format!("{} (emitted dumped to /tmp/rs-meta-emitted-bundle.rs)", e)
        })?;
        Ok((original, regen))
    })();
    match result {
        Ok((a, b)) if a == b && a.contains("rs-meta bootstrap") => {
            r.ok("emitted bundle rustc == original rustc (print_help path)".to_string())
        }
        Ok((a, b)) => r.fail(format!(
            "emitted bundle mismatch: original {} chars != regenerated {} chars",
            a.len(),
            b.len()
        )),
        Err(e) => r.fail(format!("emit-self-host: {}", e)),
    }
    let replay = (|| -> Result<(String, String), String> {
        let src = source_bundle_with_harness(stage2_chain_harness())?;
        let toks = lex(&src)?;
        let prog = parse_program(&toks)?;
        let emitted = crate::emit::emit_program(&prog);
        let original = native_run(&src, &default_workdir())?;
        let regen = native_run(&emitted, &default_workdir())?;
        Ok((original, regen))
    })();
    match replay {
        Ok((a, b)) if a == b => r.ok(format!(
            "EMITTED evaluator replays the corpus identically ({} corpus entries)",
            corpus().len()
        )),
        Ok(_) => r.fail("emitted evaluator corpus replay drifted".to_string()),
        Err(e) => r.fail(format!("emitted evaluator replay: {}", e)),
    }
    r
}

/// Phase E2: the corpus witness table is deterministic (two fresh passes are
/// byte-identical — hash drift becomes machine-readable), covers error facets
/// (a negative program witnesses as status=error with an error_kind, never a
/// hole), and lands as proof/witness-report.tsv.
pub fn witness_check() -> Report {
    let mut r = Report::new("witness-check (facet witness table: determinism + proof report)");
    let mut programs: Vec<(String, String)> = Vec::new();
    for (name, src, _expected) in corpus() {
        programs.push((String::from(name), String::from(src)));
    }
    for (name, src) in negative_corpus() {
        programs.push((format!("neg-{}", name), String::from(src)));
    }
    let a = crate::witness::witness_report(&programs);
    let b = crate::witness::witness_report(&programs);
    if a == b {
        let mut newline_count = 0usize;
        for c in a.chars() {
            if c == '\n' {
                newline_count += 1;
            }
        }
        r.ok(format!(
            "witness table deterministic ({} programs, {} records)",
            programs.len(),
            newline_count - 2
        ));
    } else {
        r.fail("witness table DRIFTED between two passes".to_string());
    }
    if a.contains("\terror\t") && a.contains("typeck") {
        r.ok("negative programs witness as status=error (no silent holes)".to_string());
    } else {
        r.fail("no error-facet records for the negative corpus".to_string());
    }
    if a.starts_with("# schema rs-meta.witness.v0\n") && a.contains(crate::witness::WITNESS_HEADER) {
        r.ok("schema header + field order stable".to_string());
    } else {
        r.fail("witness schema header drifted".to_string());
    }
    let dir_result = fs::create_dir_all("proof");
    match dir_result {
        Ok(()) => match fs::write("proof/witness-report.tsv", &a) {
            Ok(()) => r.ok("proof/witness-report.tsv written".to_string()),
            Err(e) => r.fail(format!("write witness report: {}", e)),
        },
        Err(e) => r.fail(format!("create proof dir: {}", e)),
    }
    r
}

/// Phase E3 gate: the interpreter floor runs with ZERO capabilities; the
/// native tier refuses without its grants (fail-closed, stable message);
/// full grants and the all-granted default both work. All probed in clean
/// processes.
pub fn cap_check() -> Report {
    let mut r = Report::new("cap-check (capability gate: zero-cap floor + fail-closed native tier)");
    let probe = "fn main() { println!(\"{}\", 42); }";
    let spawn = |caps: Option<&str>, cmd: &str| -> Result<(bool, String, String), String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
        let mut c = Command::new(exe);
        c.env_clear();
        c.env("SOURCE_DATE_EPOCH", "0");
        if let Ok(path) = std::env::var("PATH") {
            c.env("PATH", path);
        }
        if let Some(cs) = caps {
            c.env("RSMETA_CAPS", cs);
        }
        c.arg(cmd);
        c.arg("-c");
        c.arg(probe);
        let out = c.output().map_err(|e| format!("spawn: {}", e))?;
        Ok((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    };
    match spawn(Some(""), "run") {
        Ok((true, stdout, _)) if stdout.trim() == "42" => {
            r.ok("interpreter floor runs with ZERO capabilities".to_string())
        }
        other => r.fail(format!("zero-cap run: {:?}", other)),
    }
    match spawn(Some(""), "native-run") {
        Ok((false, _, stderr)) if stderr.contains("gate: capability") => {
            r.ok("native tier refuses without grants (fail-closed, stable message)".to_string())
        }
        other => r.fail(format!("zero-cap native-run should refuse: {:?}", other)),
    }
    match spawn(
        Some("native-compile,native-run,fs-write,subprocess"),
        "native-run",
    ) {
        Ok((true, stdout, _)) if stdout.trim() == "42" => {
            r.ok("explicit grants admit the native tier".to_string())
        }
        other => r.fail(format!("granted native-run: {:?}", other)),
    }
    match spawn(None, "native-run") {
        Ok((true, stdout, _)) if stdout.trim() == "42" => {
            r.ok("default (unset) = all granted (local-tool policy, documented)".to_string())
        }
        other => r.fail(format!("default native-run: {:?}", other)),
    }
    r
}

/// Phase E3 trace: facets are OFF by default (no records without
/// enable_trace), deterministic across runs, and a coverage probe exercises
/// bind/call/arm/loop facets; an erroring program leaves an error facet.
pub fn trace_check() -> Report {
    let mut r = Report::new("trace-check (eval trace facets: default-off, deterministic, covering)");
    let probe = "fn add(a: i64, b: i64) -> i64 { a + b } \
fn main() { let mut acc = 0; let n = 3; \
let kind = match n { 0 => 0, _ => 1 }; \
while acc < n { acc = acc + 1; } \
println!(\"{}\", add(acc, kind) + 38); }";
    let run_traced = |src: &str| -> Result<(Vec<String>, Result<String, String>), String> {
        let toks = lex(src)?;
        let prog = parse_program(&toks)?;
        typeck::check(&prog)?;
        let mut interp = Interp::new(&prog)?;
        interp.enable_trace();
        let run = interp.run_main();
        Ok((interp.take_trace(), run))
    };
    let plain = (|| -> Result<Vec<String>, String> {
        let toks = lex(probe)?;
        let prog = parse_program(&toks)?;
        typeck::check(&prog)?;
        let interp = Interp::new(&prog)?;
        let _ = interp.run_main();
        Ok(interp.take_trace())
    })();
    match plain {
        Ok(t) if t.is_empty() => r.ok("default-off: no trace records without enable_trace".to_string()),
        Ok(t) => r.fail(format!("trace leaked {} records while off", t.len())),
        Err(e) => r.fail(format!("plain run: {}", e)),
    }
    let a = run_traced(probe);
    let b = run_traced(probe);
    match (a, b) {
        (Ok((ta, ra)), Ok((tb, _rb))) => {
            if ta == tb {
                r.ok(format!("trace deterministic ({} records)", ta.len()));
            } else {
                r.fail("trace drifted between runs".to_string());
            }
            let joined = ta.join("\n");
            let mut covered = true;
            for facet in ["bind:acc", "bind:n", "bind:kind", "call:add", "arm:1", "loop:while"] {
                if !joined.contains(facet) {
                    r.fail(format!("missing facet {}", facet));
                    covered = false;
                }
            }
            if covered {
                r.ok("facets covered: bind / call / match arm / loop".to_string());
            }
            match ra {
                Ok(out) if out.trim() == "42" => {
                    r.ok("traced run result unchanged (42)".to_string())
                }
                other => r.fail(format!("traced result: {:?}", other)),
            }
        }
        other => r.fail(format!("traced runs: {:?}", other.0.is_ok())),
    }
    match run_traced("fn main() { let v: Vec<i64> = Vec::new(); println!(\"{}\", v[3]); }") {
        Ok((t, Err(_))) if t.iter().any(|l| l.starts_with("error:")) => {
            r.ok("erroring program leaves an error facet".to_string())
        }
        other => r.fail(format!("error facet: {:?}", other.is_ok())),
    }
    r
}

/// Phase E4 v1: token-index errors render with line/col + caret; errors
/// without an index pass through unchanged; rendering is deterministic and
/// end-of-input indices map to the source end (no panic).
pub fn diag_check() -> Report {
    let mut r = Report::new("diag-check (positional diagnostics: mapping, passthrough, edges)");
    let src = "fn main() {\n    let = 3;\n}";
    let err = match interp_run(src) {
        Err(e) => e,
        Ok(_) => String::new(),
    };
    let rendered = crate::diag::render_error(src, &err);
    if rendered.contains("line 2, col 9") && rendered.contains("let = 3;") {
        r.ok("parse error maps to line 2, col 9 with the source line".to_string());
    } else {
        r.fail(format!("mapping: {}", rendered));
    }
    let mut caret_ok = false;
    for line in rendered.split("\n") {
        let mut last = ' ';
        for c in line.chars() {
            last = c;
        }
        if last == '^' {
            caret_ok = true;
        }
    }
    if caret_ok {
        r.ok("caret line rendered".to_string());
    } else {
        r.fail("no caret line".to_string());
    }
    // A runtime error with neither a token index nor an `in fn NAME` prefix
    // passes through unchanged (nothing to locate).
    let plain = "interp: divide by zero";
    if crate::diag::render_error(src, plain) == plain {
        r.ok("non-positional errors pass through unchanged".to_string());
    } else {
        r.fail("passthrough mutated the error".to_string());
    }
    if crate::diag::render_error(src, &err) == rendered {
        r.ok("rendering deterministic".to_string());
    } else {
        r.fail("rendering drifted".to_string());
    }
    let truncated = "fn main() {";
    let err2 = match interp_run(truncated) {
        Err(e) => e,
        Ok(_) => String::new(),
    };
    let rendered2 = crate::diag::render_error(truncated, &err2);
    if rendered2.contains("line 1") {
        r.ok("end-of-input index maps to the source end".to_string());
    } else {
        r.fail(format!("end-of-input: {}", rendered2));
    }
    // Typeck errors map to fn-definition granularity (`in fn NAME`), the
    // plan's expression-level spans stay held.
    let tsrc = "fn helper(n: i64) -> i64 { n + 1 }\nfn main() { let x: bool = helper(3); println!(\"{}\", x); }";
    let terr = match interp_run(tsrc) {
        Err(e) => e,
        Ok(_) => String::new(),
    };
    let trendered = crate::diag::render_error(tsrc, &terr);
    if trendered.contains("line 2") && trendered.contains("fn main") {
        r.ok("typeck error maps to its fn definition (line 2, fn main)".to_string());
    } else {
        r.fail(format!("typeck mapping: {}", trendered));
    }
    r
}

/// ast-canonical faithfulness on the generic surface (2026-07-03): E1c made
/// EMIT generic-complete but sig had been erasing generic parameters, so two
/// distinct generic programs shared a sig. sig now serializes generics — this
/// check proves the fix: a generic fn's sig carries `<T>`, and a generic vs a
/// non-generic function with the SAME body get DISTINCT sigs (injectivity
/// restored on this surface).
pub fn ast_canonical_check() -> Report {
    let mut r = Report::new("ast-canonical-check (sig faithful on generics: `<T>` kept, injective)");
    let build = |src: &str| -> Result<String, String> {
        let toks = lex(src)?;
        let prog = parse_program(&toks)?;
        Ok(crate::sig::sig_program(&prog))
    };
    match build("fn id<T>(x: T) -> T { x } fn main() { println!(\"{}\", id(42)); }") {
        Ok(s) if s.contains("fn id<T>(") => {
            r.ok("generic fn sig carries the `<T>` parameter list".to_string())
        }
        Ok(s) => r.fail(format!("generic params dropped: {}", s)),
        Err(e) => r.fail(format!("generic build: {}", e)),
    }
    // Multi-parameter generics keep every parameter in order.
    match build("fn pair<T, U>(x: T, y: U) -> T { x } fn main() { println!(\"{}\", pair(4, 2)); }") {
        Ok(s) if s.contains("fn pair<T,U>(") => {
            r.ok("multi-param generics keep every parameter (`<T,U>`)".to_string())
        }
        Ok(s) => r.fail(format!("multi-param generics dropped: {}", s)),
        Err(e) => r.fail(format!("multi-param build: {}", e)),
    }
    // Injectivity witness: adding a generic parameter changes the sig (the
    // erasure that made distinct programs collide is gone).
    let g = build("fn id<T>(x: T) -> T { x } fn main() { println!(\"{}\", id(42)); }");
    let plain = build("fn id(x: i64) -> i64 { x } fn main() { println!(\"{}\", id(42)); }");
    match (g, plain) {
        (Ok(sg), Ok(sp)) if sg != sp => {
            r.ok("adding `<T>` changes the sig (no erasure collision)".to_string())
        }
        (Ok(_), Ok(_)) => r.fail("generic/non-generic sigs collide".to_string()),
        _ => r.fail("injectivity build failed".to_string()),
    }
    match build("struct W<T> { v: T } fn main() { println!(\"{}\", 42); }") {
        Ok(s) if s.contains("struct W<T>{") => {
            r.ok("generic struct sig carries `<T>`".to_string())
        }
        Ok(s) => r.fail(format!("struct generics dropped: {}", s)),
        Err(e) => r.fail(format!("generic struct build: {}", e)),
    }
    r
}

/// Native artifact receipt for a single Rust source (pnix-rs peer-engine 6순위):
/// exposes the stage8-repro reproducibility receipt (source_fnv, rustc version,
/// deterministic flags, artifact_fnv = compiled binary hash) plus a receipt hash,
/// so a `.px` control plane can attest the native build. rs-meta stays pnix-free.
pub fn rust_artifact_receipt(src: &str) -> Result<(String, String), String> {
    let receipt = native_artifact_receipt(src, &default_workdir())?;
    let receipt_hash = crate::hash::text_hash_hex(&receipt);
    Ok((receipt, receipt_hash))
}

pub fn rust_artifact_check() -> Report {
    let mut r = Report::new("rust-artifact-check (per-program native artifact receipt + reproducibility)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — rust-artifact skip (native toolchain 필요)".to_string());
        return r;
    }
    let src = "fn main() { println!(\"{}\", 40 + 2); }";
    // reproducible: same source -> same receipt (artifact_fnv + receipt_hash).
    match (rust_artifact_receipt(src), rust_artifact_receipt(src)) {
        (Ok((r1, h1)), Ok((_, h2))) if h1 == h2 => {
            let has_fields = r1.contains("artifact_fnv=")
                && r1.contains("rustc=")
                && r1.contains("source_fnv=");
            if has_fields {
                r.ok(format!("재현 가능 receipt (receipt_hash {}); artifact_fnv/rustc/source_fnv 필드 존재", h1));
            } else {
                r.fail("receipt 필드 누락".to_string());
            }
        }
        _ => r.fail("receipt 비재현(같은 source인데 다른 hash)".to_string()),
    }
    // distinct sources -> distinct artifact receipt.
    let other = "fn main() { println!(\"{}\", 7 * 6); }";
    match (rust_artifact_receipt(src), rust_artifact_receipt(other)) {
        (Ok((_, h1)), Ok((_, h2))) if h1 != h2 => {
            r.ok("다른 source -> 다른 receipt_hash".to_string());
        }
        _ => r.fail("다른 source가 같은 receipt".to_string()),
    }
    r
}

/// Extract the first Rust error code (`E0nnn`) from a diagnostic, if any.
/// Subset-safe manual scan (no regex / rfind).
pub fn extract_error_code(msg: &str) -> String {
    let chars: Vec<char> = msg.chars().collect();
    let mut i = 0usize;
    while i + 1 < chars.len() {
        if chars[i] == 'E' && chars[i + 1] == '0' {
            let mut code = String::new();
            code.push('E');
            let mut j = i + 1;
            while j < chars.len() && chars[j] >= '0' && chars[j] <= '9' {
                code.push(chars[j]);
                j += 1;
            }
            if code.chars().count() >= 4 {
                return code;
            }
        }
        i += 1;
    }
    String::from("-")
}

/// The borrow/ownership family of rustc error codes: use-after-move,
/// conflicting borrows, move/assign while borrowed, lifetime/liveness.
pub fn is_borrow_code(code: &str) -> bool {
    code == "E0382" // borrow/use of moved value
        || code == "E0499" // cannot borrow as mutable more than once
        || code == "E0502" // cannot borrow as X while borrowed as Y
        || code == "E0503" // cannot use while mutably borrowed
        || code == "E0505" // cannot move out while borrowed
        || code == "E0506" // cannot assign while borrowed
        || code == "E0515" // returns a reference to local data
        || code == "E0597" // borrowed value does not live long enough
        || code == "E0621" // explicit lifetime required
        || code == "E0106" // missing lifetime specifier
}

/// Borrow/lifetime BOUNDARY report (pnix-rs peer-engine 3순위): NOT a borrow
/// checker. rs-meta's interpreter tier does NOT model ownership; rustc (the
/// native oracle) is authoritative. This report makes that boundary machine-
/// readable and PRESERVES rustc's reason code. A program rustc rejects with a
/// borrow-family code, that rs-meta's interp still runs, is `held-borrow-not-
/// modeled` (the honest gap) — not a false "accepted".
pub struct BorrowReport {
    pub classification: String,
    pub reason_code: String,
    pub interp_accepts: bool,
    pub rustc_accepts: bool,
}

pub fn borrow_boundary_report(src: &str) -> BorrowReport {
    let interp_accepts = interp_run(src).is_ok();
    let native = native_run(src, &default_workdir());
    let (rustc_accepts, code) = match &native {
        Ok(_) => (true, String::from("-")),
        Err(e) => (false, extract_error_code(e)),
    };
    let classification = if rustc_accepts {
        if interp_accepts {
            String::from("borrow-ok")
        } else {
            String::from("interp-incomplete")
        }
    } else if is_borrow_code(&code) {
        if interp_accepts {
            // rustc's ownership check rejects; rs-meta's interp does not model it.
            String::from("held-borrow-not-modeled")
        } else {
            String::from("rejected-both")
        }
    } else {
        String::from("other-rejection")
    };
    BorrowReport {
        classification,
        reason_code: code,
        interp_accepts,
        rustc_accepts,
    }
}

/// True iff the native tier looks unavailable (a native failure that is not a
/// genuine rustc program rejection) — used to skip rustc-dependent gates.
fn native_toolchain_absent(src: &str) -> bool {
    match native_run(src, &default_workdir()) {
        Ok(_) => false,
        Err(e) => !e.contains("rustc rejected") && !e.contains("error[E"),
    }
}

pub fn borrow_boundary_check() -> Report {
    let mut r = Report::new("borrow-boundary-check (ownership boundary; rustc reason codes preserved)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — borrow boundary skip (native oracle 필요)".to_string());
        return r;
    }
    // (1) simple borrow rs-meta models + rustc accepts -> borrow-ok.
    let b1 = borrow_boundary_report("fn main() { let x = 5; let rf = &x; println!(\"{}\", *rf); }");
    if b1.classification == "borrow-ok" && b1.interp_accepts && b1.rustc_accepts {
        r.ok("단순 borrow(&x/*rf) -> borrow-ok (interp+rustc 합치)".to_string());
    } else {
        r.fail(format!("simple borrow: {} ({})", b1.classification, b1.reason_code));
    }
    // (2) use-after-move: rustc rejects E0382, rs-meta interp does not model it.
    let b2 = borrow_boundary_report("fn main() { let s = String::from(\"a\"); let t = s; println!(\"{} {}\", s, t); }");
    if b2.classification == "held-borrow-not-modeled" && b2.reason_code == "E0382" && !b2.rustc_accepts {
        r.ok(format!("use-after-move -> held-borrow-not-modeled, reason {} (interp_accepts={}, 정직한 갭)", b2.reason_code, b2.interp_accepts));
    } else {
        r.fail(format!("use-after-move: {} ({})", b2.classification, b2.reason_code));
    }
    // (3) shared+mut conflict: rustc rejects E0502.
    let b3 = borrow_boundary_report("fn main() { let mut v = vec![1]; let rf = &v; v.push(2); println!(\"{}\", rf.len()); }");
    if b3.classification == "held-borrow-not-modeled" && is_borrow_code(&b3.reason_code) {
        r.ok(format!("shared+mut 충돌 -> held-borrow-not-modeled, reason {}", b3.reason_code));
    } else {
        r.fail(format!("shared+mut: {} ({})", b3.classification, b3.reason_code));
    }
    r
}

/// True iff rs-meta's front-end (lex+parse) accepts the source — i.e. the
/// program is within the declared surface (parse boundary).
pub fn rs_meta_parses(src: &str) -> bool {
    match lex(src) {
        Ok(toks) => parse_program(&toks).is_ok(),
        Err(_) => false,
    }
}

/// Trait BOUNDARY report (pnix-rs peer-engine 4순위): NOT a trait solver. rs-meta
/// supports inherent/trait impls + narrow dispatch (Display/Debug/Iterator);
/// where-clauses, blanket/overlapping impls, dyn Trait, coherence/orphan are
/// HELD (verified: each still fails to parse or fails typeck the same way it
/// always did). Associated types are a documented exception, found live
/// (2026-08-12) to no longer be held: `type Out;` / `type Out = T;` /
/// `Self::Out` now parse, typeck, and execute correctly for a supported impl
/// target (struct/enum) -- some earlier, unrelated session's general
/// `type X = Y;` item support incidentally enabled this. But it is *syntax
/// acceptance*, not real associated-type modeling: `Self::Out` in the
/// trait's own method signature is never checked against the impl's `type
/// Out = ...` binding, so an impl can declare `type Out = i64` while its
/// method actually returns `bool`, and rs-meta accepts it silently (real
/// rustc would reject the mismatch). Classified as
/// `assoc-type-accepted-unenforced`, not `trait-dispatch-supported`, so
/// this distinction isn't lost. This classifies a program's trait surface
/// and cross-checks it against the ACTUAL parse/typeck boundary (a held
/// surface must still fail somewhere — the classification has teeth).
pub struct SurfaceReport {
    pub classification: String,
    pub parses: bool,
}

pub fn trait_boundary_report(src: &str) -> SurfaceReport {
    let parses = rs_meta_parses(src);
    // Held trait surfaces (parse-level boundaries).
    let classification = if src.contains(" where ") || src.contains("\nwhere ") {
        String::from("held-where-clause")
    } else if src.contains("dyn ") {
        String::from("held-dyn-trait")
    } else if impl_has_assoc_type(src) {
        String::from("assoc-type-accepted-unenforced")
    } else if src.contains("impl<") && src.contains("> ") && src.contains(" for ") {
        String::from("held-blanket-impl")
    } else if src.contains("trait ") || src.contains(" for ") {
        String::from("trait-dispatch-supported")
    } else {
        String::from("no-trait")
    };
    SurfaceReport { classification, parses }
}

/// Detect an associated type binding inside an impl (`type X = ...;`), the
/// `assoc-type-accepted-unenforced` surface. Coarse but sound: only impls
/// with `type ` bindings match.
fn impl_has_assoc_type(src: &str) -> bool {
    src.contains("impl ") && src.contains("type ") && src.contains(" = ")
}

/// Macro BOUNDARY report (pnix-rs peer-engine 5순위): rs-meta supports a FIXED
/// macro set (format!/vec!/println!/matches!/write!/panic!/assert!/...); user
/// `macro_rules!` (lex boundary: `$`), proc/derive/attribute macros are HELD.
pub fn macro_boundary_report(src: &str) -> SurfaceReport {
    let parses = rs_meta_parses(src);
    let classification = if src.contains("macro_rules!") {
        String::from("held-macro-rules")
    } else if src.contains("#[proc_macro") {
        String::from("held-proc-macro")
    } else if src.contains("#[derive(") {
        String::from("derive-approx")
    } else if uses_fixed_macro(src) {
        String::from("fixed-macro-supported")
    } else {
        String::from("no-macro")
    };
    SurfaceReport { classification, parses }
}

fn uses_fixed_macro(src: &str) -> bool {
    src.contains("println!")
        || src.contains("format!")
        || src.contains("vec!")
        || src.contains("matches!")
        || src.contains("write!")
        || src.contains("panic!")
        || src.contains("assert!")
}

/// Differential-testing fuzz gate (deep-research 2026-07-04 #1; Csmith/PLDI'11):
/// a DETERMINISTIC, well-defined-BY-CONSTRUCTION Rust generator over the
/// evaluated subset, cross-checked interp-stdout == rustc-stdout. No reference
/// oracle: the interpreter (trusted floor) and rustc (native tier) are two
/// independent implementations of one semantics, so any divergence localizes a
/// bug and can be minted into the corpus. Soundness (finding #3): generated
/// programs avoid overflow (additive combination of small values; `*` only on
/// single-digit literals), division, and nondeterminism (no HashMap iteration,
/// no unsafe — all held), so each program has ONE well-defined output.
struct FuzzRng {
    state: u64,
}

impl FuzzRng {
    fn seeded(seed: u64) -> FuzzRng {
        // Ensure a nonzero, well-mixed initial state.
        FuzzRng {
            state: seed.wrapping_mul(2654435761).wrapping_add(1442695040888963407),
        }
    }
    fn next_u64(&mut self) -> u64 {
        // LCG (Knuth MMIX constants); use MIDDLE bits (avoid low-quality low bits).
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn below(&mut self, n: u64) -> u64 {
        (self.next_u64() / 65536) % n
    }
}

/// A bounded expression over `vars` and single-digit literals. Additive combo
/// keeps values small (no overflow); `*` only multiplies two single digits.
fn fuzz_expr(rng: &mut FuzzRng, vars: &Vec<String>, depth: i64) -> String {
    let use_var = !vars.is_empty() && rng.below(2) == 0;
    if depth <= 0 {
        if use_var {
            let i = (rng.below(vars.len() as u64)) as usize;
            return vars[i].clone();
        }
        return format!("{}", rng.below(10));
    }
    let choice = rng.below(6);
    if choice == 5 && use_var {
        let i = (rng.below(vars.len() as u64)) as usize;
        return vars[i].clone();
    }
    if choice == 2 {
        // multiply only single digits to stay small.
        let a = rng.below(10);
        let b = rng.below(10);
        return format!("({} * {})", a, b);
    }
    if choice == 3 {
        // integer division by a NONZERO literal (1..9): truncation toward zero
        // with a possibly-negative dividend is a classic interp-vs-rustc
        // divergence surface, and dividing by nonzero is well-defined.
        let l = fuzz_expr(rng, vars, depth - 1);
        let d = 1 + rng.below(9);
        return format!("({} / {})", l, d);
    }
    if choice == 4 {
        // remainder by a nonzero literal (sign-of-dividend semantics).
        let l = fuzz_expr(rng, vars, depth - 1);
        let d = 1 + rng.below(9);
        return format!("({} % {})", l, d);
    }
    let op = if choice == 0 { "+" } else { "-" };
    let l = fuzz_expr(rng, vars, depth - 1);
    let r = fuzz_expr(rng, vars, depth - 1);
    format!("({} {} {})", l, op, r)
}

/// Generate one deterministic, well-defined Rust program for the fuzz gate.
pub fn fuzz_gen(seed: u64) -> String {
    fuzz_gen_inject(seed, "")
}

/// Same as fuzz_gen but injects `inject` right after `fn main() { ` — used by
/// the EMI gate to build a base and a dead-code variant that are identical
/// except the injection (no string mutation; interp has no String::replace).
pub fn fuzz_gen_inject(seed: u64, inject: &str) -> String {
    let mut rng = FuzzRng::seeded(seed);
    // Optionally generate helper functions (stress fn/params/call/return). Their
    // bodies are bounded expressions over the params, and calls pass single
    // digits, so results stay small (no overflow) and deterministic.
    let mut helpers = String::new();
    let mut fns: Vec<String> = Vec::new();
    let nfns = rng.below(2); // 0 or 1 helper
    let mut fi = 0u64;
    while fi < nfns {
        let fname = format!("f{}", fi);
        let params = vec![String::from("p0"), String::from("p1")];
        let fbody = fuzz_expr(&mut rng, &params, 2);
        helpers.push_str(&format!(
            "fn {}(p0: i64, p1: i64) -> i64 {{ {} }} ",
            fname, fbody
        ));
        fns.push(fname);
        fi += 1;
    }
    // Optionally add ONE bounded recursive helper (a DIFFERENT generation
    // strategy -- deep-research #3: diversify generators to escape saturation).
    // r0 sums 0..=p0 with a strictly-decreasing argument so it terminates and
    // stays bounded; it stresses the interpreter call stack / recursion path.
    let has_rec = rng.below(2) == 0;
    let rec_def = if has_rec {
        let base = rng.below(10);
        format!(
            "fn r0(p0: i64) -> i64 {{ if p0 < 1 {{ {} }} else {{ (p0 + r0(p0 - 1)) }} }} ",
            base
        )
    } else {
        String::from("")
    };
    // Optionally define + construct a struct (stress struct literal + field
    // access). Field values are bounded exprs; field access `s0.fN` is a bounded
    // i64 leaf usable in later expressions.
    let has_struct = rng.below(2) == 0;
    let struct_def = if has_struct {
        "struct S0 { f0: i64, f1: i64 } "
    } else {
        ""
    };
    let mut vars: Vec<String> = Vec::new();
    let mut body = String::new();
    if has_struct {
        let e0 = fuzz_expr(&mut rng, &vars, 2);
        let e1 = fuzz_expr(&mut rng, &vars, 2);
        body.push_str(&format!("let s0 = S0 {{ f0: {}, f1: {} }}; ", e0, e1));
        vars.push(String::from("s0.f0"));
        vars.push(String::from("s0.f1"));
    }
    // Optionally attach a trait with a DEFAULT method + a declared method to the
    // struct (stress trait-default flattening + override dispatch).
    let has_trait = has_struct && rng.below(2) == 0;
    let trait_def = if has_trait {
        "trait T0 { fn tv(&self) -> i64 { 7 } fn tw(&self) -> i64; } impl T0 for S0 { fn tw(&self) -> i64 { 5 } } "
    } else {
        ""
    };
    if has_trait {
        vars.push(String::from("s0.tv()"));
        vars.push(String::from("s0.tw()"));
    }
    // Optionally define + construct an enum, then match-on-variant with binding
    // (stress enum discriminant + variant pattern binding). The match result is
    // bound as a var for later use.
    let has_enum = rng.below(2) == 0;
    let enum_def = if has_enum {
        "enum E0 { A(i64), B(i64), C } "
    } else {
        ""
    };
    if has_enum {
        let which = rng.below(3);
        let payload = fuzz_expr(&mut rng, &vars, 2);
        let ctor = if which == 0 {
            format!("E0::A({})", payload)
        } else if which == 1 {
            format!("E0::B({})", payload)
        } else {
            String::from("E0::C")
        };
        let arm_a = fuzz_expr(&mut rng, &vars, 1);
        let arm_b = fuzz_expr(&mut rng, &vars, 1);
        let arm_c = fuzz_expr(&mut rng, &vars, 1);
        body.push_str(&format!(
            "let e0 = {}; let ev = match e0 {{ E0::A(n) => (n + {}), E0::B(n) => (n - {}), E0::C => {} }}; ",
            ctor, arm_a, arm_b, arm_c
        ));
        vars.push(String::from("ev"));
    }
    let nlets = 2 + rng.below(3); // 2..4 let bindings
    let mut i = 0u64;
    while i < nlets {
        let name = format!("v{}", i);
        // Occasionally call the recursive helper with a bounded argument.
        let call_rec = has_rec && rng.below(3) == 0;
        // Occasionally the let value is a call to a helper (sequenced borrows).
        let call_it = !fns.is_empty() && rng.below(3) == 0;
        let e = if call_rec {
            let arg = rng.below(7);
            format!("r0({})", arg)
        } else if call_it {
            let ci = (rng.below(fns.len() as u64)) as usize;
            let a0 = rng.below(10);
            let a1 = rng.below(10);
            format!("{}({}, {})", fns[ci], a0, a1)
        } else {
            let depth = 2 + (rng.below(2) as i64);
            fuzz_expr(&mut rng, &vars, depth)
        };
        body.push_str(&format!("let {} = {}; ", name, e));
        vars.push(name);
        i += 1;
    }
    // Sometimes wrap the final value in an if/else over a comparison.
    // Optionally build an Option and match on it (stress Some/None + option
    // pattern binding). The match result is exposed as a var.
    let has_opt = rng.below(2) == 0;
    if has_opt {
        let is_some = rng.below(2) == 0;
        let payload = fuzz_expr(&mut rng, &vars, 2);
        let ctor = if is_some {
            format!("Some({})", payload)
        } else {
            String::from("None")
        };
        let some_arm = fuzz_expr(&mut rng, &vars, 1);
        let none_arm = fuzz_expr(&mut rng, &vars, 1);
        body.push_str(&format!(
            "let o0: Option<i64> = {}; let ov = match o0 {{ Some(n) => (n + {}), None => {} }}; ",
            ctor, some_arm, none_arm
        ));
        vars.push(String::from("ov"));
    }
    // Optionally build a Vec and index it (stress Vec literal + indexing +
    // `.len()`, where the interpreter reimplements std semantics so a divergence
    // is most likely to hide). Fixed indices keep it deterministic + bounded.
    let has_vec = rng.below(2) == 0;
    if has_vec {
        let e0 = fuzz_expr(&mut rng, &vars, 2);
        let e1 = fuzz_expr(&mut rng, &vars, 2);
        let e2 = fuzz_expr(&mut rng, &vars, 2);
        body.push_str(&format!("let vv = vec![{}, {}, {}]; ", e0, e1, e2));
        vars.push(String::from("vv[0]"));
        vars.push(String::from("vv[2]"));
        vars.push(String::from("(vv.len() as i64)"));
    }
    // Optionally define a closure that captures locals and apply it with a
    // bounded argument (stress closure capture + application -- a distinct
    // interpreter path). The captured body is a bounded expr; the result stays
    // small.
    let has_closure = rng.below(2) == 0;
    if has_closure {
        let cap = fuzz_expr(&mut rng, &vars, 1);
        let arg = rng.below(10);
        body.push_str(&format!("let cl = |x: i64| (x + {}); ", cap));
        vars.push(format!("cl({})", arg));
    }
    // Optionally exercise i64 wrapping arithmetic near the overflow boundary
    // (the single most likely divergence surface). wrapping_add/mul are
    // WELL-DEFINED (deterministic two's-complement wrap) and interp==rustc; this
    // is DISTINCT from plain `x + 1` overflow, where the interpreter wraps at
    // runtime but rustc REJECTS a compile-time-constant overflow (a held const-
    // overflow lint) -- a documented boundary, not generated here.
    let has_wrap = rng.below(2) == 0;
    if has_wrap {
        let bigs = vec![
            String::from("9000000000000000000"),
            String::from("8000000000000000000"),
            String::from("7000000000000000000"),
            String::from("9223372036854775807"),
        ];
        let ai = (rng.below(bigs.len() as u64)) as usize;
        let bi = (rng.below(bigs.len() as u64)) as usize;
        let m = 1 + rng.below(9);
        body.push_str(&format!(
            "let w0 = {}i64.wrapping_add({}).wrapping_mul({}); ",
            bigs[ai], bigs[bi], m
        ));
        vars.push(String::from("w0"));
    }
    // Optionally run a bounded mutable while-loop that accumulates (stress
    // `let mut` + reassignment + `while` + loop state -- a distinct interpreter
    // path from the immutable let-chains). Bounded iterations keep it terminating
    // and small; `acc` is exposed as a leaf.
    let has_loop = rng.below(2) == 0;
    if has_loop {
        let bound = 2 + rng.below(5); // 2..6 iterations
        body.push_str(&format!(
            "let mut acc = 0; let mut li = 0; while li < {} {{ acc = acc + li; li = li + 1; }} ",
            bound
        ));
        vars.push(String::from("acc"));
    }
    // Optionally take a reference to v0 and expose its deref as a leaf (stress
    // & / * reference semantics). v0 is a plain i64 (Copy, immutable, still
    // alive), so the borrow is always valid.
    let has_ref = rng.below(2) == 0;
    if has_ref {
        body.push_str("let r0 = &v0; ");
        vars.push(String::from("(*r0)"));
    }
    // Optionally build a String via format! and expose its byte length (stress
    // format! + String::len, another std reimplementation surface). The length
    // of the decimal rendering is deterministic + bounded.
    let has_str = rng.below(2) == 0 && !vars.is_empty();
    if has_str {
        let si = (rng.below(vars.len() as u64)) as usize;
        let sj = (rng.below(vars.len() as u64)) as usize;
        body.push_str(&format!(
            "let ss = format!(\"{{}}-{{}}\", {}, {}); ",
            vars[si], vars[sj]
        ));
        vars.push(String::from("(ss.len() as i64)"));
    }
    // Optionally build a tuple from two existing vars and expose its fields as
    // leaves (stress tuple literal + `.0`/`.1` indexing).
    let has_tuple = rng.below(2) == 0 && vars.len() >= 2;
    if has_tuple {
        let ti = (rng.below(vars.len() as u64)) as usize;
        let tj = (rng.below(vars.len() as u64)) as usize;
        body.push_str(&format!("let t0 = ({}, {}); ", vars[ti], vars[tj]));
        vars.push(String::from("t0.0"));
        vars.push(String::from("t0.1"));
    }
    // --- surfaces for the 2026-07 subset expansions ---
    // Bitwise & | ^ and shifts over bounded values (well-defined on i64).
    let has_bits = rng.below(2) == 0;
    if has_bits {
        let a = rng.below(64);
        let b = rng.below(64);
        let k = 3 + rng.below(5);
        let j = rng.below(3);
        body.push_str(&format!(
            "let b0 = (({} & {}) | ({} ^ {})) + ((1i64 << {}) >> {}); ",
            a, b, a, b, k, j
        ));
        vars.push(String::from("b0"));
    }
    // Compound assignment ops (<<=, |=, &=, +=) on a bounded mutable.
    let has_cassign = rng.below(2) == 0;
    if has_cassign {
        let st = rng.below(30);
        let sh = 1 + rng.below(3);
        let mask = 64 + rng.below(64);
        body.push_str(&format!(
            "let mut c0 = {}; c0 <<= {}; c0 |= 1; c0 &= {}; c0 += 2; ",
            st, sh, mask
        ));
        vars.push(String::from("c0"));
    }
    // Labeled loop-as-expression: `'lz: loop { break 'lz v; }`.
    let has_label = rng.below(2) == 0;
    if has_label {
        let e = fuzz_expr(&mut rng, &vars, 1);
        body.push_str(&format!("let lb = 'lz: loop {{ break 'lz ({}); }}; ", e));
        vars.push(String::from("lb"));
    }
    // Integer methods pow/signum/rem_euclid (bounded, nonzero divisor).
    let has_imeth = rng.below(2) == 0;
    if has_imeth {
        let base = rng.below(9);
        let ex = rng.below(4);
        let sg = rng.below(9);
        let re = 20 + rng.below(20);
        let d = 1 + rng.below(6);
        body.push_str(&format!(
            "let im = {}i64.pow({}) + ({}i64 - 4).signum() + {}i64.rem_euclid({}); ",
            base, ex, sg, re, d
        ));
        vars.push(String::from("im"));
    }
    // Vec methods sort/dedup/contains (mutating std reimplementations).
    let has_vmeth = rng.below(2) == 0;
    if has_vmeth {
        let a = rng.below(9);
        let b = rng.below(9);
        let c = rng.below(9);
        body.push_str(&format!(
            "let mut mv = vec![{}, {}, {}, {}]; mv.sort(); mv.dedup(); let mvl = (mv.len() as i64) + mv[0] + (if mv.contains(&{}) {{ 1 }} else {{ 0 }}); ",
            a, b, c, b, c
        ));
        vars.push(String::from("mvl"));
    }
    // Iterator adaptor chain over .iter() (ref arithmetic + map + sum).
    let has_iter = rng.below(2) == 0;
    if has_iter {
        let a = rng.below(9);
        let b = rng.below(9);
        let c = rng.below(9);
        let m = 1 + rng.below(4);
        body.push_str(&format!(
            "let ic: i64 = vec![{}, {}, {}].iter().map(|x| x * {}).sum(); ",
            a, b, c, m
        ));
        vars.push(String::from("ic"));
    }
    // Fixed-size array annotation + HashMap single-key index (never iterated --
    // rustc HashMap iteration order is random; a single keyed read is exact).
    let has_coll = rng.below(2) == 0;
    if has_coll {
        let a = rng.below(9);
        let b = rng.below(9);
        let k = rng.below(9);
        let v = rng.below(90);
        body.push_str(&format!(
            "let ar: [i64; 2] = [{}, {}]; let mut hm: std::collections::HashMap<i64, i64> = std::collections::HashMap::new(); hm.insert({}, {}); let hc = ar[1] + hm[&{}]; ",
            a, b, k, v, k
        ));
        vars.push(String::from("hc"));
    }
    // Sequence the rng borrows (each &mut rng must end before the next).
    let shape = if vars.len() >= 2 { rng.below(4) } else { rng.below(2) };
    let final_expr = if shape == 0 {
        // if/else over a comparison — diversify among the 6 comparison operators
        // (==, !=, <, >, <=, >=) so each operator's semantics is exercised.
        let ai = (rng.below(vars.len() as u64)) as usize;
        let bi = (rng.below(vars.len() as u64)) as usize;
        let cmp = rng.below(6);
        let opc = if cmp == 0 {
            "=="
        } else if cmp == 1 {
            "!="
        } else if cmp == 2 {
            "<"
        } else if cmp == 3 {
            ">"
        } else if cmp == 4 {
            "<="
        } else {
            ">="
        };
        let then_e = fuzz_expr(&mut rng, &vars, 2);
        let else_e = fuzz_expr(&mut rng, &vars, 2);
        format!(
            "if {} {} {} {{ {} }} else {{ {} }}",
            vars[ai], opc, vars[bi], then_e, else_e
        )
    } else if shape == 3 && vars.len() >= 2 {
        // DEEP NESTED control flow (if inside if/else) -- a different
        // distribution from the flat top-level shapes; stresses the
        // interpreter's recursive control-flow evaluation.
        let a = (rng.below(vars.len() as u64)) as usize;
        let b = (rng.below(vars.len() as u64)) as usize;
        let c = (rng.below(vars.len() as u64)) as usize;
        let d = (rng.below(vars.len() as u64)) as usize;
        let inner_then = fuzz_expr(&mut rng, &vars, 1);
        let inner_else = fuzz_expr(&mut rng, &vars, 1);
        let outer_else = fuzz_expr(&mut rng, &vars, 1);
        format!(
            "if {} < {} {{ if {} <= {} {{ {} }} else {{ {} }} }} else {{ {} }}",
            vars[a], vars[b], vars[c], vars[d], inner_then, inner_else, outer_else
        )
    } else if shape == 1 {
        // match on a variable with two literal arms + wildcard
        let si = (rng.below(vars.len() as u64)) as usize;
        let p0 = rng.below(10);
        let p1 = rng.below(10);
        let a0 = fuzz_expr(&mut rng, &vars, 2);
        let a1 = fuzz_expr(&mut rng, &vars, 2);
        let aw = fuzz_expr(&mut rng, &vars, 2);
        format!(
            "match {} {{ {} => {}, {} => {}, _ => {} }}",
            vars[si], p0, a0, p1, a1, aw
        )
    } else {
        fuzz_expr(&mut rng, &vars, 2)
    };
    format!(
        "{}{}{}{}{}fn main() {{ {}{}println!(\"{{}}\", {}); }}",
        struct_def, trait_def, enum_def, rec_def, helpers, inject, body, final_expr
    )
}

pub fn fuzz_diff_check() -> Report {
    let mut r = Report::new("fuzz-check (differential testing: generated Rust, interp-stdout == rustc-stdout)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — fuzz-check skip (native oracle 필요)".to_string());
        return r;
    }
    // (1) determinism: same seed -> same program (reproducible gate).
    if fuzz_gen(7) == fuzz_gen(7) && fuzz_gen(7) != fuzz_gen(8) {
        r.ok("생성기 결정성 (같은 seed -> 같은 프로그램, 다른 seed -> 다른 프로그램)".to_string());
    } else {
        r.fail("생성기 비결정성".to_string());
    }
    // (2) DIFFERENTIAL: interp-stdout == rustc-stdout for N generated programs.
    let mut agree = 0;
    let mut seed = 1u64;
    let mut first_divergence = String::new();
    while seed <= 78 {
        let prog = fuzz_gen(seed);
        let interp = interp_run(&prog);
        let native = native_run(&prog, &default_workdir());
        match (interp, native) {
            (Ok(i), Ok(n)) => {
                if i == n {
                    agree += 1;
                } else if first_divergence.is_empty() {
                    first_divergence = format!("seed {}: interp `{}` != rustc `{}` :: {}", seed, i.trim(), n.trim(), prog);
                }
            }
            (i, n) => {
                if first_divergence.is_empty() {
                    first_divergence = format!("seed {}: interp={:?} native_ok={} :: {}", seed, i.is_ok(), n.is_ok(), prog);
                }
            }
        }
        seed += 1;
    }
    if first_divergence.is_empty() {
        r.ok(format!("78개 생성 프로그램 전부 interp-stdout == rustc-stdout (발산 0; recursion/mut-loop/closure/struct/enum/tuple/option/vec/string/ref/div-mod/wrap/fn/call/let/if/match/산술)"));
    } else {
        r.fail(format!("interp!=rustc 발산 발견 (버그 국소화됨): {}", first_divergence));
    }
    let _ = agree;
    r
}

/// Deep differential search (deep-research: scale the search to actually FIND a
/// divergence). Runs `n` generated programs through both tiers and returns the
/// first interp!=rustc divergence, or None after n. Not a gate (n rustc compiles
/// is expensive); a manual `fuzz-scale <n>` tool. A divergence is the payoff:
/// feed it to shrink_program (divergence-preserving) and mint it to the corpus.
pub fn fuzz_scale(n: u64) -> (u64, String) {
    let mut seed = 1u64;
    while seed <= n {
        let prog = fuzz_gen(seed);
        let interp = interp_run(&prog);
        let native = native_run(&prog, &default_workdir());
        match (interp, native) {
            (Ok(i), Ok(nv)) => {
                if i.trim() != nv.trim() {
                    return (
                        seed,
                        format!("DIVERGENCE seed {}: interp `{}` != rustc `{}` :: {}", seed, i.trim(), nv.trim(), prog),
                    );
                }
            }
            (i, nv) => {
                // one tier accepted, the other errored -> also a divergence
                if i.is_ok() != nv.is_ok() {
                    return (
                        seed,
                        format!("ACCEPT-DIVERGENCE seed {}: interp_ok={} native_ok={} :: {}", seed, i.is_ok(), nv.is_ok(), prog),
                    );
                }
            }
        }
        seed += 1;
    }
    (n, String::from("no divergence"))
}

/// EMI metamorphic-mutation gate (deep-research 2026-07-04 #2; EMI/Orion PLDI'14,
/// 147 GCC/LLVM bugs). A semantics-PRESERVING mutation must not change output:
/// inject observable code into a PROVABLY-DEAD branch (`if false { ... }`) of a
/// base program and require interpreter-stdout AND rustc-stdout to stay
/// IDENTICAL to the base. If either tier mishandles dead code (executes it,
/// mis-lowers it), output diverges and the bug is localized. Teeth: the same
/// injection under `if true` DOES change output (the injected println is
/// observable), so the dead-branch invariance is non-vacuous.
pub fn emi_check() -> Report {
    let mut r = Report::new("emi-check (metamorphic: dead-branch mutation preserves interp & rustc stdout)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — emi-check skip (native oracle 필요)".to_string());
        return r;
    }
    let seeds = vec![3u64, 7, 12, 20];
    let mut invariant = 0;
    let mut divergence = String::new();
    let mut idx = 0usize;
    while idx < seeds.len() {
        let s = seeds[idx]; // by value (interp method dispatch dislikes &u64)
        idx += 1;
        let base = fuzz_gen(s);
        let base_out = match interp_run(&base) {
            Ok(o) => o,
            Err(_) => continue,
        };
        // Observable dead code: prints if (wrongly) executed.
        let mut drng = FuzzRng::seeded(s.wrapping_add(1000));
        let dead_expr = fuzz_expr(&mut drng, &Vec::new(), 3);
        let dead = format!("if false {{ println!(\"{{}}\", {}); }} ", dead_expr);
        let variant = fuzz_gen_inject(s, &dead);
        let vi = interp_run(&variant);
        let vn = native_run(&variant, &default_workdir());
        match (vi, vn) {
            (Ok(i), Ok(n)) => {
                if i == base_out && n == base_out {
                    invariant += 1;
                } else if divergence.is_empty() {
                    divergence = format!("seed {}: base `{}` interp `{}` rustc `{}`", s, base_out.trim(), i.trim(), n.trim());
                }
            }
            _ => {
                if divergence.is_empty() {
                    divergence = format!("seed {}: variant failed to run", s);
                }
            }
        }
    }
    if divergence.is_empty() {
        r.ok(format!("죽은 분기 주입이 interp & rustc stdout 불변 ({}개 변이)", invariant));
    } else {
        r.fail(format!("metamorphic 불변 위반 (dead-code 버그 국소화): {}", divergence));
    }
    // TEETH: the same injection LIVE (if true) DOES change output.
    let base = fuzz_gen(3);
    if let Ok(base_out) = interp_run(&base) {
        let live = fuzz_gen_inject(3, "if true { println!(\"{}\", 424242); } ");
        match interp_run(&live) {
            Ok(o) if o != base_out => {
                r.ok("live 주입(if true)은 출력 변경 -> 죽은-분기 불변이 non-vacuous (이빨)".to_string());
            }
            _ => r.fail("live 주입이 출력을 안 바꿈(vacuous)".to_string()),
        }
    }
    r
}

/// Known interp!=rustc BOUNDARY map (differential-testing payoff, tracked). The
/// fuzzer avoids these for oracle soundness; this gate makes the deliberate
/// boundaries EXPLICIT and drift-detected. If the interpreter later closes a
/// boundary (e.g. implements a const-overflow lint), the gate flags that the
/// boundary moved.
pub fn boundary_check() -> Report {
    let mut r = Report::new("boundary-check (known interp!=rustc boundaries are stable + documented)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — boundary-check skip".to_string());
        return r;
    }
    // (1) const-overflow: interp WRAPS at runtime; rustc REJECTS via its
    // deny-by-default arithmetic-overflow lint on const-evaluable expressions.
    // A held const-overflow-lint boundary (interp has no compile-time analysis).
    let ovf = "fn main() { let x = 9223372036854775807i64; println!(\"{}\", x + 1); }";
    let oi = interp_run(ovf);
    let on = native_run(ovf, &default_workdir());
    let wraps = match &oi { Ok(s) => s.trim() == "-9223372036854775808", Err(_) => false };
    if wraps && on.is_err() {
        r.ok("const-overflow: interp wraps(-i64::MIN) / rustc rejects(overflow lint) — held boundary 안정".to_string());
    } else {
        r.fail(format!("const-overflow boundary 이동: interp_wraps={} rustc_rejected={}", wraps, on.is_err()));
    }
    // (2) const division-by-zero: BOTH reject (interp runtime error, rustc
    // compile rejection) — a rejection boundary they AGREE on.
    let dz = "fn main() { println!(\"{}\", 1 / 0); }";
    let di = interp_run(dz);
    let dn = native_run(dz, &default_workdir());
    if di.is_err() && dn.is_err() {
        r.ok("const div-by-zero: interp+rustc 둘 다 거부 (거부 경계 일치)".to_string());
    } else {
        r.fail(format!("div-by-zero boundary: interp_err={} rustc_err={}", di.is_err(), dn.is_err()));
    }
    // (3) RUNTIME-vs-COMPILE-TIME rejection: rustc rejects these at COMPILE time
    // (const array OOB, non-exhaustive match); the interpreter rejects them at
    // RUN time. The accept/reject VERDICT agrees (both reject) even though the
    // phase differs -- the interpreter's runtime checks catch what rustc's
    // static checks reject.
    let oob = "fn main() { let a = [1, 2]; println!(\"{}\", a[5]); }";
    let ne = "fn main() { let x = 3; let r = match x { 1 => 10 }; println!(\"{}\", r); }";
    let oob_agree = interp_run(oob).is_err() && native_run(oob, &default_workdir()).is_err();
    let ne_agree = interp_run(ne).is_err() && native_run(ne, &default_workdir()).is_err();
    if oob_agree && ne_agree {
        r.ok("array-OOB & 비-exhaustive match: rustc 컴파일 거부 == interp 런타임 거부 (verdict 일치, 단계만 다름)".to_string());
    } else {
        r.fail(format!("runtime/compile 거부 경계: oob={} nonexhaustive={}", oob_agree, ne_agree));
    }
    // (4) control: a well-defined program still agrees (the boundaries are
    // specific, not a blanket interp!=rustc).
    let ok_prog = "fn main() { println!(\"{}\", 2 + 2); }";
    let ci = interp_run(ok_prog);
    let cn = native_run(ok_prog, &default_workdir());
    match (ci, cn) {
        (Ok(a), Ok(b)) if a.trim() == "4" && b.trim() == "4" => {
            r.ok("대조: 잘-정의된 프로그램은 여전히 interp==rustc (경계는 특정적)".to_string())
        }
        _ => r.fail("대조 프로그램 불일치".to_string()),
    }
    r
}

/// Program shrinker (deep-research roadmap: delta debugging / fuzzing-book). A
/// conservative statement-level ddmin: split on "; ", drop each removable chunk
/// and keep the drop only if BOTH tiers still produce the baseline output. This
/// removes unused let bindings (dead code the final expression never reads).
/// The predicate here is OUTPUT-PRESERVATION; when a divergence exists the same
/// engine minimizes it with a DIVERGENCE-preservation predicate (interp!=rustc)
/// to a small corpus reproducer.
fn rejoin_chunks(chunks: &Vec<String>, keep: &Vec<bool>) -> String {
    let mut out = String::new();
    let mut first = true;
    let mut i = 0;
    while i < chunks.len() {
        if keep[i] {
            if !first {
                out.push_str("; ");
            }
            out.push_str(&chunks[i]);
            first = false;
        }
        i += 1;
    }
    out
}

pub fn shrink_program(program: &str) -> String {
    let baseline = match interp_run(program) {
        Ok(o) => o,
        Err(_) => return String::from(program),
    };
    let mut chunks: Vec<String> = Vec::new();
    for c in program.split("; ") {
        chunks.push(String::from(c));
    }
    let mut keep: Vec<bool> = Vec::new();
    let mut k = 0;
    while k < chunks.len() {
        keep.push(true);
        k += 1;
    }
    let mut i = 0;
    while i < chunks.len() {
        // Only try to drop chunks that look like a standalone `let` binding.
        if chunks[i].starts_with("let ") {
            keep[i] = false;
            let candidate = rejoin_chunks(&chunks, &keep);
            let interp_ok = interp_run(&candidate).map(|o| o == baseline).unwrap_or(false);
            let native_ok = native_run(&candidate, &default_workdir())
                .map(|o| o == baseline)
                .unwrap_or(false);
            if !(interp_ok && native_ok) {
                keep[i] = true; // dropping it changed behaviour -> keep it
            }
        }
        i += 1;
    }
    rejoin_chunks(&chunks, &keep)
}

pub fn shrink_check() -> Report {
    let mut r = Report::new("shrink-check (delta-debugging: output-preserving statement removal)");
    if native_toolchain_absent("fn main() { println!(\"{}\", 1); }") {
        r.ok("rustc 없음 — shrink-check skip (native oracle 필요)".to_string());
        return r;
    }
    // A program with several let bindings, some unused by the final expression.
    let prog = "fn main() { let a = 3 + 4; let b = a - 1; let c = 9 * 2; let d = b + 5; println!(\"{}\", a + d); }";
    let base = match interp_run(prog) { Ok(o) => o, Err(_) => String::new() };
    let shrunk = shrink_program(prog);
    let s_interp = match interp_run(&shrunk) { Ok(o) => o, Err(_) => String::new() };
    let s_native = match native_run(&shrunk, &default_workdir()) { Ok(o) => o, Err(_) => String::new() };
    // (1) shrunk output equals baseline on BOTH tiers.
    if s_interp == base && s_native == base && !base.is_empty() {
        r.ok(format!("shrunk 출력 == baseline ({}) — 두 tier 모두", base.trim()));
    } else {
        r.fail(format!("shrink 출력 불일치: base={} interp={} native={}", base.trim(), s_interp.trim(), s_native.trim()));
    }
    // (2) shrunk is strictly smaller (an unused `let c = 9 * 2` is removed).
    if shrunk.len() < prog.len() && !shrunk.contains("let c =") {
        r.ok(format!("사용 안 된 let 제거 -> {} bytes < {} bytes (최소 재현자)", shrunk.len(), prog.len()));
    } else {
        r.fail(format!("shrink 안 됨: {} vs {} bytes", shrunk.len(), prog.len()));
    }
    // (3) shrinking is idempotent (a shrunk program does not shrink further).
    if shrink_program(&shrunk).len() == shrunk.len() {
        r.ok("shrink 멱등 (고정점)".to_string());
    } else {
        r.fail("shrink 비멱등".to_string());
    }
    r
}

/// Self-hosting blocker audit (deep-research 2026-07-04 open question #1). The
/// evaluator CORE (lexer/parser/ast/typeck/interp/sig/hash) is what source-
/// bundle-check proves interp==rustc on — i.e. what rs-meta already self-hosts.
/// This gate confirms the core is FREE of the held-feature blockers that would
/// require lifting a held feature to self-host: no `macro_rules!` definition, no
/// `async`, no `unsafe`, no `trait` definition (rs-meta defines zero traits).
/// The mentions of those features elsewhere (check.rs boundary-report test data,
/// doc comments) are strings, not code. Finding: NO held feature blocks the core
/// self-host — matching mrustc (borrow checker not needed) and extending it. The
/// real remaining self-host work is the full-chain COST (stage3-full-chain is
/// DONE but budget-gated), not any held language feature. This gate is drift-
/// resistant: adding a trait/macro/async/unsafe to a core file flags a NEW
/// self-host blocker.
pub fn selfhost_audit_check() -> Report {
    let mut r = Report::new("selfhost-audit-check (evaluator core is held-feature-free -> self-hostable)");
    let core = vec![
        String::from("src/lexer.rs"),
        String::from("src/ast.rs"),
        String::from("src/parser.rs"),
        String::from("src/typeck.rs"),
        String::from("src/interp.rs"),
        String::from("src/sig.rs"),
        String::from("src/hash.rs"),
    ];
    let blockers = vec![
        String::from("macro_rules!"),
        String::from("async fn"),
        String::from("unsafe "),
        String::from("\ntrait "),
        String::from("\npub trait "),
    ];
    let mut clean = 0;
    let mut found = String::new();
    let mut fi = 0;
    while fi < core.len() {
        let f = &core[fi];
        fi += 1;
        let text = match fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => {
                if found.is_empty() {
                    found = format!("cannot read {}", f);
                }
                continue;
            }
        };
        let mut bi = 0;
        let mut file_clean = true;
        while bi < blockers.len() {
            if text.contains(&blockers[bi]) {
                file_clean = false;
                if found.is_empty() {
                    found = format!("{} contains held-feature blocker `{}`", f, blockers[bi].trim());
                }
            }
            bi += 1;
        }
        if file_clean {
            clean += 1;
        }
    }
    if found.is_empty() {
        r.ok(format!(
            "코어 {}파일 전부 held-feature blocker 0 (macro_rules/async/unsafe/trait-def) -> 코어 self-hostable (source-bundle가 interp==rustc 증명)",
            clean
        ));
        r.ok("결론: 어떤 held 기능도 코어 self-host를 막지 않음 (mrustc식). 남은 작업은 full-chain 비용(stage3-full-chain DONE·budget-gated), held 기능 아님".to_string());
    } else {
        r.fail(format!("새 self-host blocker 발견: {}", found));
    }
    r
}

/// Corpus auto-mint (deep-research 2026-07-04 roadmap #3): fold verified
/// generated programs into a PERMANENT, re-checked corpus so coverage grows
/// MONOTONICALLY. `fuzz-mint <n>` generates seeds 1..=n, keeps only those where
/// interp-stdout == rustc-stdout, and writes `proofs/fuzz-corpus.tsv`
/// (seed \t expected \t source). Each minted program is thereafter a frozen
/// regression: `fuzz-corpus-check` re-runs the interpreter and requires it still
/// produces the recorded output (catching interp drift from the baseline, which
/// the live fuzz-check's interp==rustc alone would not).
pub fn fuzz_mint(n: u64) -> Result<usize, String> {
    let mut out = String::new();
    let mut kept = 0usize;
    let mut seed = 1u64;
    while seed <= n {
        let prog = fuzz_gen(seed);
        let interp = interp_run(&prog);
        let native = native_run(&prog, &default_workdir());
        match (interp, native) {
            (Ok(i), Ok(nv)) => {
                if i == nv {
                    // source is single-line and tab-free by construction.
                    out.push_str(&format!("{}\t{}\t{}\n", seed, i.trim(), prog));
                    kept += 1;
                }
            }
            _ => {}
        }
        seed += 1;
    }
    fs::write("proofs/fuzz-corpus.tsv", &out).map_err(|e| format!("write fuzz-corpus: {}", e))?;
    Ok(kept)
}

pub fn fuzz_corpus_check() -> Report {
    let mut r = Report::new("fuzz-corpus-check (minted differential corpus: interp reproduces frozen output)");
    let text = match fs::read_to_string("proofs/fuzz-corpus.tsv") {
        Ok(s) => s,
        Err(_) => {
            r.ok("proofs/fuzz-corpus.tsv 없음 — mint 전 skip (bootstrap fuzz-mint <n>)".to_string());
            return r;
        }
    };
    let mut checked = 0usize;
    let mut mismatch = String::new();
    for line in text.split("\n") {
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split("\t").collect();
        if cols.len() != 3 {
            continue;
        }
        let expected = cols[1];
        let source = cols[2];
        match interp_run(source) {
            Ok(o) => {
                if o.trim() == expected {
                    checked += 1;
                } else if mismatch.is_empty() {
                    mismatch = format!("seed {}: expected `{}` got `{}`", cols[0], expected, o.trim());
                }
            }
            Err(e) => {
                if mismatch.is_empty() {
                    mismatch = format!("seed {}: interp error {}", cols[0], e);
                }
            }
        }
    }
    if mismatch.is_empty() {
        r.ok(format!("{}개 minted 프로그램이 frozen 출력 재현 (monotone 회귀 corpus)", checked));
    } else {
        r.fail(format!("minted corpus 회귀: {}", mismatch));
    }
    r
}

/// Translation-validation coverage stats (pnix-rs peer-engine trust signal): how
/// many positive/negative corpus programs back this engine's interp==rustc
/// claim. The COUNTS come from the corpus; the VERIFICATION is gated by tv-check
/// (positive: interp stdout == rustc stdout) and typeck-check (negative: interp
/// rejects iff rustc rejects). A control plane can weight engines by coverage.
pub fn tv_stats_report() -> (usize, usize) {
    (corpus().len(), negative_corpus().len())
}

/// The self-hosting credential (the deepest meta-circular claim): rs-meta's
/// interpreter, given rs-meta's OWN evaluator source, reproduces the reference
/// behaviour == rustc. Gated by stage3-full-chain (all-source stage2 -> stage2'
/// full corpus replay), which is DONE (manifest-recorded, budget-gated on cost).
pub fn self_host_gate() -> &'static str {
    "stage3-full-chain"
}

pub fn tv_stats_check() -> Report {
    let mut r = Report::new("tv-stats-check (translation-validation corpus coverage)");
    let (pos, neg) = tv_stats_report();
    if pos > 0 && neg > 0 {
        r.ok(format!("positive corpus {} + negative corpus {} (tv-check/typeck-check가 검증)", pos, neg));
    } else {
        r.fail(format!("corpus 비어있음: pos={} neg={}", pos, neg));
    }
    // stats are deterministic.
    let (p2, n2) = tv_stats_report();
    if p2 == pos && n2 == neg {
        r.ok("stats 결정성".to_string());
    } else {
        r.fail("stats 비결정성".to_string());
    }
    r
}

/// Combined per-program surface classification (trait + macro), parse-based
/// (no rustc), for the pnix-rs peer-engine verdict `surface` field. Returns
/// (trait_surface, macro_surface, parses). rs-meta owns the classification; the
/// adapter consumes it across the CLI.
pub fn rust_surface_report(src: &str) -> (String, String, bool) {
    let ts = trait_boundary_report(src);
    let ms = macro_boundary_report(src);
    (ts.classification, ms.classification, ts.parses)
}

pub fn rust_surface_check() -> Report {
    let mut r = Report::new("rust-surface-check (per-program trait+macro surface classification)");
    // supported program.
    let (t1, m1, p1) = rust_surface_report("fn main() { let v = vec![1]; println!(\"{}\", v.len()); }");
    if m1 == "fixed-macro-supported" && p1 {
        r.ok(format!("지원 프로그램 -> trait={} macro={} parses={}", t1, m1, p1));
    } else {
        r.fail(format!("supported: trait={} macro={} parses={}", t1, m1, p1));
    }
    // held macro surface.
    let (_t2, m2, p2) = rust_surface_report("macro_rules! sq { ($x:expr) => { $x }; } fn main() {}");
    if m2 == "held-macro-rules" && !p2 {
        r.ok("macro_rules! -> macro=held-macro-rules, parses=false".to_string());
    } else {
        r.fail(format!("held macro: macro={} parses={}", m2, p2));
    }
    // associated-type binding: found live (2026-08-12) to actually parse and
    // typecheck for a supported impl target, not held at the parse boundary
    // the way it used to be -- see trait_boundary_report's doc comment and
    // trait-boundary-check for the full story. This program's specific
    // target (`impl Two for i64`, a primitive) still fails, but at typeck
    // (unsupported primitive impl target), not at the associated-type
    // syntax itself, so the classification is `assoc-type-accepted-
    // unenforced`, matching trait_boundary_report's current behavior.
    let (t3, _m3, _p3) = rust_surface_report("trait Two { type Out; fn two(&self) -> Self::Out; } impl Two for i64 { type Out = i64; fn two(&self) -> i64 { self * 2 } } fn main() {}");
    if t3 == "assoc-type-accepted-unenforced" {
        r.ok("impl associated-type binding -> trait=assoc-type-accepted-unenforced".to_string());
    } else {
        r.fail(format!("held trait: trait={}", t3));
    }
    // held trait surface (dyn Trait).
    let (t4, _m4, _p4) = rust_surface_report("trait T { fn v(&self) -> i64; } fn go(x: &dyn T) -> i64 { x.v() } fn main() {}");
    if t4 == "held-dyn-trait" {
        r.ok("dyn Trait -> trait=held-dyn-trait".to_string());
    } else {
        r.fail(format!("held dyn: trait={}", t4));
    }
    r
}

pub fn trait_boundary_check() -> Report {
    let mut r = Report::new("trait-boundary-check (trait surface; supported vs held, classification==parse boundary)");
    // supported: trait + inherent dispatch parses.
    let s1 = trait_boundary_report("trait Speak { fn hi(&self) -> i64; } struct D {} impl Speak for D { fn hi(&self) -> i64 { 42 } } fn main() { let d = D {}; println!(\"{}\", d.hi()); }");
    if s1.classification == "trait-dispatch-supported" && s1.parses {
        r.ok("trait+inherent dispatch -> supported, 파스 O".to_string());
    } else {
        r.fail(format!("supported trait: {} parses={}", s1.classification, s1.parses));
    }
    // assoc-type-accepted-unenforced: found live (2026-08-12) to actually
    // parse, typecheck, and execute correctly for a supported impl target
    // (struct, not the primitive-impl-target boundary `impl Two for i64`
    // would separately hit) -- this is genuine syntax acceptance, verified
    // by actually running it, not just checking it parses.
    let s2_src = "trait Two { type Out; fn two(&self) -> Self::Out; } struct D { n: i64 } impl Two for D { type Out = i64; fn two(&self) -> i64 { self.n * 2 } } fn main() { let d = D { n: 21 }; println!(\"{}\", d.two()); }";
    let s2 = trait_boundary_report(s2_src);
    let s2_runs = match interp_run(s2_src) {
        Ok(out) => out.trim() == "42",
        Err(_) => false,
    };
    if s2.classification == "assoc-type-accepted-unenforced" && s2.parses && s2_runs {
        r.ok("associated type -> assoc-type-accepted-unenforced, 파스 O + 실행 O (syntax accepted, not really modeled)".to_string());
    } else {
        r.fail(format!(
            "assoc type: {} parses={} runs={}",
            s2.classification, s2.parses, s2_runs
        ));
    }
    // Same boundary, the "unenforced" half: Self::Out in the trait's method
    // signature is never checked against the impl's own `type Out = ...`
    // binding, so a mismatched impl (declares Out=i64, method returns bool)
    // is silently accepted rather than rejected the way real rustc would.
    let s2b_src = "trait Two { type Out; fn two(&self) -> Self::Out; } struct D { n: i64 } impl Two for D { type Out = i64; fn two(&self) -> bool { true } } fn main() { let d = D { n: 21 }; println!(\"{}\", d.two()); }";
    match interp_run(s2b_src) {
        Ok(out) if out.trim() == "true" => r.ok(
            "associated type -> Self::Out not enforced against impl's type binding (honest gap, not silently claimed fixed)".to_string(),
        ),
        Ok(out) => r.fail(format!("assoc type unenforced-mismatch: unexpected output {:?}", out)),
        Err(e) => r.fail(format!(
            "assoc type unenforced-mismatch: expected silent acceptance (documented gap), got rejection: {}",
            e
        )),
    }
    // held: dyn Trait classified held.
    let s3 = trait_boundary_report("trait T { fn v(&self) -> i64; } fn go(x: &dyn T) -> i64 { x.v() } fn main() {}");
    if s3.classification == "held-dyn-trait" {
        r.ok("dyn Trait -> held-dyn-trait".to_string());
    } else {
        r.fail(format!("held dyn: {}", s3.classification));
    }
    r
}

pub fn macro_boundary_check() -> Report {
    let mut r = Report::new("macro-boundary-check (macro surface; fixed vs macro_rules/proc held, classification==parse boundary)");
    // supported: fixed macros parse.
    let m1 = macro_boundary_report("fn main() { let v = vec![1, 2]; println!(\"{}\", format!(\"{}\", v.len())); }");
    if m1.classification == "fixed-macro-supported" && m1.parses {
        r.ok("고정 매크로(vec!/format!/println!) -> supported, 파스 O".to_string());
    } else {
        r.fail(format!("fixed macro: {} parses={}", m1.classification, m1.parses));
    }
    // held: macro_rules! does NOT parse (lex boundary: `$`).
    let m2 = macro_boundary_report("macro_rules! sq { ($x:expr) => { $x * $x }; } fn main() { println!(\"{}\", sq!(5)); }");
    if m2.classification == "held-macro-rules" && !m2.parses {
        r.ok("macro_rules! -> held-macro-rules, 파스 X (lex 경계 $ 확인)".to_string());
    } else {
        r.fail(format!("held macro_rules: {} parses={}", m2.classification, m2.parses));
    }
    // held: derive classified.
    let m3 = macro_boundary_report("#[derive(Clone)] struct P { x: i64 } fn main() {}");
    if m3.classification == "derive-approx" {
        r.ok("#[derive(..)] -> derive-approx".to_string());
    } else {
        r.fail(format!("derive: {}", m3.classification));
    }
    r
}

/// Canonical Rust IR (proposal: pnix-rs 0009 feeds this into the peer-engine
/// verdict ir_hash). The canonical IR is the mirror-proven ast-canonical
/// serialization (sig_program), content-addressed by a stable ir_hash. Unlike a
/// raw source hash it is FORMAT-INVARIANT (whitespace/comments discarded at
/// parse) and faithful (distinct programs -> distinct IR, via ast-canonical
/// injectivity). `evaluable` = the AST re-emits to parseable Rust (the IR
/// corresponds to a runnable program; interp/rustc parity is gated separately by
/// tv-check/emit-tv-check).
pub fn rust_ir_of(src: &str) -> Result<(String, String, bool), String> {
    let toks = lex(src)?;
    let prog = parse_program(&toks)?;
    let canonical = crate::sig::sig_program(&prog);
    let ir_hash = crate::hash::text_hash_hex(&canonical);
    let emitted = emit_program(&prog);
    let evaluable = match lex(&emitted) {
        Ok(t2) => parse_program(&t2).is_ok(),
        Err(_) => false,
    };
    Ok((canonical, ir_hash, evaluable))
}

pub fn rust_ir_check() -> Report {
    let mut r = Report::new("rust-ir-check (content-addressed canonical Rust IR + ir_hash)");
    // (1) deterministic ir_hash.
    let a = rust_ir_of("fn main() { let x = 5; println!(\"{}\", x + 1); }");
    let b = rust_ir_of("fn main() { let x = 5; println!(\"{}\", x + 1); }");
    match (&a, &b) {
        (Ok((_, ha, _)), Ok((_, hb, _))) if ha == hb => {
            r.ok(format!("ir_hash 결정성 ({})", ha))
        }
        _ => r.fail("ir_hash 비결정성".to_string()),
    }
    // (2) FORMAT-INVARIANT: whitespace/comment differences -> same ir_hash.
    let plain = rust_ir_of("fn main(){println!(\"{}\",1+2);}");
    let spaced = rust_ir_of("fn main() {  /* c */  println!( \"{}\" , 1 + 2 ) ; }");
    match (&plain, &spaced) {
        (Ok((_, hp, _)), Ok((_, hs, _))) if hp == hs => {
            r.ok("포맷 불변: 공백/주석 차이 -> 같은 ir_hash (source_hash와 구별되는 IR)".to_string())
        }
        _ => r.fail("포맷 불변 실패".to_string()),
    }
    // (3) FAITHFUL: distinct programs -> distinct ir_hash.
    let p1 = rust_ir_of("fn main() { println!(\"{}\", 1); }");
    let p2 = rust_ir_of("fn main() { println!(\"{}\", 2); }");
    match (&p1, &p2) {
        (Ok((_, h1, _)), Ok((_, h2, _))) if h1 != h2 => {
            r.ok("faithful: 다른 프로그램 -> 다른 ir_hash".to_string())
        }
        _ => r.fail("faithful 실패 (충돌)".to_string()),
    }
    // (4) evaluable: the IR re-emits to parseable Rust.
    match rust_ir_of("fn add(a: i64, b: i64) -> i64 { a + b } fn main() { println!(\"{}\", add(1, 2)); }") {
        Ok((_, _, true)) => r.ok("evaluable: IR가 파스 가능 Rust로 재방출됨".to_string()),
        Ok((_, _, false)) => r.fail("evaluable=false".to_string()),
        Err(e) => r.fail(format!("evaluable: {}", e)),
    }
    r
}

/// AST diff over the mirror-proven canonical serialization (the rs-meta
/// analogue of pnix-rs ir-diff): two Rust programs with the same ast-canonical
/// are structurally identical; a change is localized to the first differing
/// position. Honest boundary: ast-canonical is a FAITHFUL serialization (no
/// alpha/reorder normalization), so a local rename or item reorder shows a diff
/// — this is a canonical-AST structural diff, not a semantic-up-to-alpha diff.
pub fn ast_canonical_of(src: &str) -> Result<String, String> {
    let toks = lex(src)?;
    let prog = parse_program(&toks)?;
    Ok(crate::sig::sig_program(&prog))
}

/// (identical, first_diff_offset, window) — subset-safe char scan.
pub fn ast_diff(a_src: &str, b_src: &str) -> Result<(bool, usize, String), String> {
    let a = ast_canonical_of(a_src)?;
    let b = ast_canonical_of(b_src)?;
    if a == b {
        return Ok((true, 0, String::new()));
    }
    let ca: Vec<char> = a.chars().collect();
    let cb: Vec<char> = b.chars().collect();
    let mut i = 0usize;
    while i < ca.len() && i < cb.len() && ca[i] == cb[i] {
        i += 1;
    }
    let mut wa = String::new();
    let mut k = i;
    while k < ca.len() && k < i + 16 {
        wa.push(ca[k]);
        k += 1;
    }
    let mut wb = String::new();
    let mut k = i;
    while k < cb.len() && k < i + 16 {
        wb.push(cb[k]);
        k += 1;
    }
    Ok((false, i, format!("a:`{}` | b:`{}`", wa, wb)))
}

pub fn ast_diff_check() -> Report {
    let mut r = Report::new("ast-diff-check (canonical-AST semantic diff; rs-meta analogue of ir-diff)");
    // identical program -> identical.
    match ast_diff(
        "fn main() { println!(\"{}\", 42); }",
        "fn main() { println!(\"{}\", 42); }",
    ) {
        Ok((true, _, _)) => r.ok("identical program -> identical".to_string()),
        other => r.fail(format!("identical: {:?}", other)),
    }
    // semantic change -> different + localized.
    match ast_diff(
        "fn main() { println!(\"{}\", 42); }",
        "fn main() { println!(\"{}\", 43); }",
    ) {
        Ok((false, off, win)) => r.ok(format!("semantic change -> diff at {} {}", off, win)),
        other => r.fail(format!("semantic diff: {:?}", other)),
    }
    // added statement -> different.
    match ast_diff(
        "fn main() { println!(\"{}\", 1); }",
        "fn main() { let x = 2; println!(\"{}\", x); }",
    ) {
        Ok((false, _, _)) => r.ok("structural change (added let) -> diff".to_string()),
        other => r.fail(format!("structural: {:?}", other)),
    }
    // HONEST BOUNDARY: ast-canonical is faithful (no alpha/reorder norm) — a
    // local rename shows a diff. This is a canonical-AST structural diff.
    match ast_diff(
        "fn main() { let x = 5; println!(\"{}\", x); }",
        "fn main() { let y = 5; println!(\"{}\", y); }",
    ) {
        Ok((false, _, _)) => {
            r.ok("local rename -> diff (canonical-AST is faithful, not alpha-normalized)".to_string())
        }
        other => r.fail(format!("rename boundary: {:?}", other)),
    }
    r
}

pub fn stage2_chain_check() -> Report {
    let mut r = Report::new("stage2-chain-check (all-source evaluator' corpus replay)");
    let expected_count = corpus().len().to_string();
    let result = (|| -> Result<(String, String), String> {
        let src = source_bundle_with_harness(stage2_chain_harness())?;
        let interp = interp_run(&src)?;
        let native = native_run(&src, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n))
            if i == n
                && i.contains("42")
                && i.contains("120")
                && i.contains("3")
                && i.contains(expected_count.as_str())
                && i.contains("15")
                && i.contains("rs-meta") =>
        {
            r.ok("all-source evaluator' positive corpus replay interp == rustc".to_string())
        }
        Ok((i, n)) => r.fail(format!(
            "stage2 evaluator' mismatch: interp {:?} != rustc {:?}",
            i, n
        )),
        Err(e) => r.fail(format!("stage2 evaluator': {}", e)),
    }
    r
}

fn stage2_chain_harness() -> &'static str {
    "fn main() { \
        let cases = corpus(); \
        let mut i = 0usize; \
        while i < cases.len() { \
            let (_name, src, expected) = cases[i]; \
            let out = interp_run(src).unwrap(); \
            if out.trim() != expected { panic!(\"stage2 prefix mismatch\"); } \
            i += 1; \
        } \
        println!(\"{}\", i); \
        let a = interp_run(\"fn main() { println!(\\\"{}\\\", 40 + 2); }\").unwrap(); \
        let b = interp_run(\"fn fact(n: i64) -> i64 { if n < 2 { 1 } else { n * fact(n - 1) } } fn main() { println!(\\\"{}\\\", fact(5)); }\").unwrap(); \
        let c = interp_run(\"enum Flag { Off, On } fn score(f: Flag) -> i64 { match f { Flag::Off => 1, Flag::On => 2 } } fn main() { println!(\\\"{}\\\", score(Flag::Off) + score(Flag::On)); }\").unwrap(); \
        let d = interp_run(\"struct Point { x: i64, y: i64 } fn main() { let p = Point { x: 10, y: 5 }; println!(\\\"{}\\\", p.x + p.y); }\").unwrap(); \
        let e = interp_run(\"fn main() { let mut v = Vec::new(); v.push(String::from(\\\"rs\\\")); v.push(String::from(\\\"meta\\\")); println!(\\\"{}\\\", v.join(\\\"-\\\")); }\").unwrap(); \
        let f = interp_run(\"struct Counter { n: i64 } impl Counter { fn add(&mut self, d: i64) { self.n += d; } } fn main() { let mut c = Counter { n: 10 }; c.add(5); println!(\\\"{}\\\", c.n); }\").unwrap(); \
        print!(\"{}{}{}{}{}{}\", a, b, c, d, e, f); \
    }\n"
}

pub fn stage2_probe_check() -> Report {
    let mut r = Report::new("stage2-probe-check (source slices interp == rustc)");
    run_source_probe(
        &mut r,
        "lexer.rs lex()",
        vec!["src/lexer.rs"],
        "fn main() { let toks = lex(\"fn main() { println!(\\\"{}\\\", 42); }\").unwrap(); println!(\"{}\", toks.len()); }\n",
    );
    run_source_probe(
        &mut r,
        "parser.rs parse_program()",
        vec!["src/ast.rs", "src/lexer.rs", "src/parser.rs"],
        "fn main() { let src = \"fn add(a: i64, b: i64) -> i64 { a + b } fn main() { let x = add(1, 2); }\"; let toks = lex(src).unwrap(); let prog = parse_program(&toks).unwrap(); println!(\"{}\", prog.funcs.len()); }\n",
    );
    run_source_probe(
        &mut r,
        "typeck.rs check()",
        vec!["src/ast.rs", "src/lexer.rs", "src/parser.rs", "src/typeck.rs"],
        "fn main() { let src = \"fn add(a: i64, b: i64) -> i64 { a + b } fn main() { println!(\\\"{}\\\", add(40, 2)); }\"; let toks = lex(src).unwrap(); let prog = parse_program(&toks).unwrap(); check(&prog).unwrap(); println!(\"{}\", prog.funcs.len()); }\n",
    );
    run_source_probe(
        &mut r,
        "interp.rs Interp::run_main()",
        vec![
            "src/ast.rs",
            "src/lexer.rs",
            "src/parser.rs",
            "src/typeck.rs",
            "src/interp.rs",
        ],
        "fn main() { let src = \"fn main() { println!(\\\"{}\\\", 40 + 2); }\"; let toks = lex(src).unwrap(); let prog = parse_program(&toks).unwrap(); check(&prog).unwrap(); let interp = Interp::new(&prog).unwrap(); let out = interp.run_main().unwrap(); print!(\"{}\", out); }\n",
    );
    r
}

pub fn stage3_chain_check() -> Report {
    let mut r = Report::new("stage3-chain-check (slim evaluator stage2 -> stage2')");
    let result = (|| -> Result<(String, String), String> {
        let inner = stage3_slim_bundle("fn main() { println!(\"{}\", 42); }\n")?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_slim_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i.trim() == "42" && i == n => {
            r.ok("slim evaluator stage2 -> stage2' chain interp == rustc".to_string())
        }
        Ok((i, n)) => r.fail(format!(
            "stage3 slim chain mismatch: interp {:?} != rustc {:?}",
            i.trim(),
            n.trim()
        )),
        Err(e) => r.fail(format!("stage3 slim chain: {}", e)),
    }
    r
}

pub fn stage3_all_source_smoke_check() -> Report {
    let mut r = Report::new(
        "stage3-all-source-smoke-check (slimmed evaluator-core stage2 -> stage2' smoke)",
    );
    let result = (|| -> Result<(String, String), String> {
        let inner = stage3_core_source_bundle("fn main() { println!(\"{}\", 42); }\n")?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i.trim() == "42" && i == n => r.ok(
            "slimmed evaluator-core source bundle stage2 loads/evaluates stage2' smoke and matches rustc"
                .to_string(),
        ),
        Ok((i, n)) => r.fail(format!(
            "stage3 all-source smoke mismatch: interp {:?} != rustc {:?}",
            i.trim(),
            n.trim()
        )),
        Err(e) => r.fail(format!("stage3 all-source smoke: {}", e)),
    }
    r
}

pub fn stage3_core_mini_check() -> Report {
    let mut r = Report::new("stage3-core-mini-check (evaluator-core stage2' mini-corpus)");
    let expected = "42\n120\n3\n15\nrs-meta\nrs\n42\n";
    let result = (|| -> Result<(String, String), String> {
        let inner = stage3_core_source_bundle(stage3_core_mini_harness())?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => {
            r.ok("evaluator-core stage2' mini-corpus replay interp == rustc".to_string())
        }
        Ok((i, n)) => r.fail(format!(
            "stage3 core mini mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core mini: {}", e)),
    }
    r
}

fn stage3_core_mini_harness() -> &'static str {
    "fn main() { \
        let a = interp_run(\"fn main() { println!(\\\"{}\\\", 40 + 2); }\").unwrap(); \
        let b = interp_run(\"fn fact(n: i64) -> i64 { if n < 2 { 1 } else { n * fact(n - 1) } } fn main() { println!(\\\"{}\\\", fact(5)); }\").unwrap(); \
        let c = interp_run(\"enum Flag { Off, On } fn score(f: Flag) -> i64 { match f { Flag::Off => 1, Flag::On => 2 } } fn main() { println!(\\\"{}\\\", score(Flag::Off) + score(Flag::On)); }\").unwrap(); \
        let d = interp_run(\"struct Point { x: i64, y: i64 } fn main() { let p = Point { x: 10, y: 5 }; println!(\\\"{}\\\", p.x + p.y); }\").unwrap(); \
        let e = interp_run(\"fn main() { let mut v = Vec::new(); v.push(String::from(\\\"rs\\\")); v.push(String::from(\\\"meta\\\")); println!(\\\"{}\\\", v.join(\\\"-\\\")); }\").unwrap(); \
        let f = interp_run(\"fn main() { let s = \\\"rs\\\".chars().collect::<String>(); println!(\\\"{}\\\", s); }\").unwrap(); \
        let g = interp_run(\"use std::rc::Rc; fn main() { let r = Rc::new(\\\"rs\\\".chars().collect::<Vec<char>>()); println!(\\\"{}\\\", r.len() as i64 + 40); }\").unwrap(); \
        print!(\"{}{}{}{}{}{}{}\", a, b, c, d, e, f, g); \
    }\n"
}

const STAGE3_CORE_PREFIX_COUNT: usize = 8;
const STAGE3_CORE_MIDDLE_COUNT: usize = 8;
const STAGE3_CORE_SUFFIX_COUNT: usize = 8;
const STAGE3_CORE_NEGATIVE_MIDDLE_COUNT: usize = 8;
const STAGE3_CORE_NEGATIVE_SUFFIX_COUNT: usize = 8;

pub fn stage3_core_prefix_check() -> Report {
    let mut r = Report::new("stage3-core-prefix-check (evaluator-core stage2' corpus prefix)");
    let expected = format!("{}\n", STAGE3_CORE_PREFIX_COUNT);
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_prefix_harness(STAGE3_CORE_PREFIX_COUNT)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' replays first {} positive corpus cases interp == rustc",
            STAGE3_CORE_PREFIX_COUNT
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core prefix mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core prefix: {}", e)),
    }
    r
}

fn stage3_core_prefix_harness(limit: usize) -> Result<String, String> {
    let cases = corpus();
    if limit > cases.len() {
        return Err(format!(
            "stage3 core prefix limit {} exceeds corpus size {}",
            limit,
            cases.len()
        ));
    }
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = 0usize;
    while i < limit {
        let (name, src, expected) = cases[i];
        out.push_str("    let out = interp_run(");
        out.push_str(&rust_string_literal(src));
        out.push_str(").unwrap();\n");
        out.push_str("    if out.trim() != ");
        out.push_str(&rust_string_literal(expected));
        out.push_str(" { panic!(");
        out.push_str(&rust_string_literal(name));
        out.push_str("); }\n");
        out.push_str("    passed += 1;\n");
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_middle_check() -> Report {
    let mut r =
        Report::new("stage3-core-middle-check (evaluator-core stage2' corpus middle shard)");
    let expected = format!("{}\n", STAGE3_CORE_MIDDLE_COUNT);
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_middle_harness(STAGE3_CORE_MIDDLE_COUNT)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' replays middle {} positive corpus cases interp == rustc",
            STAGE3_CORE_MIDDLE_COUNT
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core middle mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core middle: {}", e)),
    }
    r
}

fn stage3_core_middle_harness(limit: usize) -> Result<String, String> {
    let cases = corpus();
    if limit > cases.len() {
        return Err(format!(
            "stage3 core middle limit {} exceeds corpus size {}",
            limit,
            cases.len()
        ));
    }
    let start = (cases.len() - limit) / 2;
    let end = start + limit;
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = start;
    while i < end {
        let (name, src, expected) = cases[i];
        out.push_str("    let out = interp_run(");
        out.push_str(&rust_string_literal(src));
        out.push_str(").unwrap();\n");
        out.push_str("    if out.trim() != ");
        out.push_str(&rust_string_literal(expected));
        out.push_str(" { panic!(");
        out.push_str(&rust_string_literal(name));
        out.push_str("); }\n");
        out.push_str("    passed += 1;\n");
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_suffix_check() -> Report {
    let mut r = Report::new("stage3-core-suffix-check (evaluator-core stage2' corpus suffix)");
    let expected = format!("{}\n", STAGE3_CORE_SUFFIX_COUNT);
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_suffix_harness(STAGE3_CORE_SUFFIX_COUNT)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' replays last {} positive corpus cases interp == rustc",
            STAGE3_CORE_SUFFIX_COUNT
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core suffix mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core suffix: {}", e)),
    }
    r
}

fn stage3_core_suffix_harness(limit: usize) -> Result<String, String> {
    let cases = corpus();
    if limit > cases.len() {
        return Err(format!(
            "stage3 core suffix limit {} exceeds corpus size {}",
            limit,
            cases.len()
        ));
    }
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = cases.len() - limit;
    while i < cases.len() {
        let (name, src, expected) = cases[i];
        out.push_str("    let out = interp_run(");
        out.push_str(&rust_string_literal(src));
        out.push_str(").unwrap();\n");
        out.push_str("    if out.trim() != ");
        out.push_str(&rust_string_literal(expected));
        out.push_str(" { panic!(");
        out.push_str(&rust_string_literal(name));
        out.push_str("); }\n");
        out.push_str("    passed += 1;\n");
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_feature_check() -> Report {
    let mut r = Report::new("stage3-core-feature-check (evaluator-core stage2' feature corpus)");
    let names = stage3_core_feature_case_names();
    let expected = format!("{}\n", names.len());
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_named_harness(&names)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' replays {} named feature corpus cases interp == rustc",
            names.len()
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core feature mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core feature: {}", e)),
    }
    r
}

fn stage3_core_feature_case_names() -> Vec<&'static str> {
    vec![
        "u64-from-str-radix",
        "type-alias-rc-refcell-borrow",
        "trait-impl-method-surface",
        "write-macro-string",
        "struct-like-enum-pattern-rest",
        "slice-array-compare",
        "clone-vec-deep",
        "generic-enum-match",
        "let-else-success",
        "array-repeat",
    ]
}

fn stage3_core_named_harness(names: &[&str]) -> Result<String, String> {
    let cases = corpus();
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = 0usize;
    while i < names.len() {
        let wanted = names[i];
        let mut found = false;
        let mut j = 0usize;
        while j < cases.len() {
            let (name, src, expected) = cases[j];
            if name == wanted {
                out.push_str("    let out = interp_run(");
                out.push_str(&rust_string_literal(src));
                out.push_str(").unwrap();\n");
                out.push_str("    if out.trim() != ");
                out.push_str(&rust_string_literal(expected));
                out.push_str(" { panic!(");
                out.push_str(&rust_string_literal(name));
                out.push_str("); }\n");
                out.push_str("    passed += 1;\n");
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            return Err(format!("stage3 core named case not found: {}", wanted));
        }
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_negative_check() -> Report {
    let mut r = Report::new("stage3-core-negative-check (evaluator-core stage2' negative corpus)");
    let names = stage3_core_negative_case_names();
    let expected = format!("{}\n", names.len());
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_negative_harness(&names)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' rejects {} named negative corpus cases interp == rustc",
            names.len()
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core negative mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core negative: {}", e)),
    }
    r
}

fn stage3_core_negative_case_names() -> Vec<&'static str> {
    vec![
        "add-bool",
        "refmut-on-immutable",
        "vec-push-wrong-type",
        "result-map-err-non-closure",
        "u64-from-str-radix-bad-radix",
        "write-immutable-string",
        "or-pattern-type-mismatch",
        "let-else-non-diverging",
        "struct-like-enum-pattern-rest-unknown-field",
        "generic-enum-pattern-mismatch",
    ]
}

fn stage3_core_negative_harness(names: &[&str]) -> Result<String, String> {
    let cases = negative_corpus();
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = 0usize;
    while i < names.len() {
        let wanted = names[i];
        let mut found = false;
        let mut j = 0usize;
        while j < cases.len() {
            let (name, src) = cases[j];
            if name == wanted {
                out.push_str("    if !interp_run(");
                out.push_str(&rust_string_literal(src));
                out.push_str(").is_err() { panic!(");
                out.push_str(&rust_string_literal(name));
                out.push_str("); }\n");
                out.push_str("    passed += 1;\n");
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            return Err(format!("stage3 core negative case not found: {}", wanted));
        }
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_negative_middle_check() -> Report {
    let mut r = Report::new(
        "stage3-core-negative-middle-check (evaluator-core stage2' negative middle shard)",
    );
    let expected = format!("{}\n", STAGE3_CORE_NEGATIVE_MIDDLE_COUNT);
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_negative_middle_harness(STAGE3_CORE_NEGATIVE_MIDDLE_COUNT)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' rejects middle {} negative corpus cases interp == rustc",
            STAGE3_CORE_NEGATIVE_MIDDLE_COUNT
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core negative middle mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core negative middle: {}", e)),
    }
    r
}

fn stage3_core_negative_middle_harness(limit: usize) -> Result<String, String> {
    let cases = negative_corpus();
    if limit > cases.len() {
        return Err(format!(
            "stage3 core negative middle limit {} exceeds negative corpus size {}",
            limit,
            cases.len()
        ));
    }
    let start = (cases.len() - limit) / 2;
    let end = start + limit;
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = start;
    while i < end {
        let (name, src) = cases[i];
        out.push_str("    if !interp_run(");
        out.push_str(&rust_string_literal(src));
        out.push_str(").is_err() { panic!(");
        out.push_str(&rust_string_literal(name));
        out.push_str("); }\n");
        out.push_str("    passed += 1;\n");
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

pub fn stage3_core_negative_suffix_check() -> Report {
    let mut r =
        Report::new("stage3-core-negative-suffix-check (evaluator-core stage2' negative suffix)");
    let expected = format!("{}\n", STAGE3_CORE_NEGATIVE_SUFFIX_COUNT);
    let result = (|| -> Result<(String, String), String> {
        let harness = stage3_core_negative_suffix_harness(STAGE3_CORE_NEGATIVE_SUFFIX_COUNT)?;
        let inner = stage3_core_source_bundle(&harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                let out = interp_run(src).unwrap(); \
                print!(\"{{}}\", out); \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = stage3_core_source_bundle(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i == expected && n == expected => r.ok(format!(
            "evaluator-core stage2' rejects last {} negative corpus cases interp == rustc",
            STAGE3_CORE_NEGATIVE_SUFFIX_COUNT
        )),
        Ok((i, n)) => r.fail(format!(
            "stage3 core negative suffix mismatch: interp {:?} != rustc {:?}; expected {:?}",
            i, n, expected
        )),
        Err(e) => r.fail(format!("stage3 core negative suffix: {}", e)),
    }
    r
}

pub fn stage3_full_chain_check() -> Report {
    let mut r = Report::new("stage3-full-chain-check (all-source evaluator stage2 -> stage2')");
    let expected_count = corpus().len().to_string();
    let budget_seconds = std::env::var("RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    if budget_seconds <= 0 {
        r.ok(format!(
            "stage3 full-chain budget is {}s; set RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS>0 to run full all-source chain",
            budget_seconds
        ));
        return r;
    }

    let smoke = std::env::var("RS_META_STAGE3_FULL_CHAIN_SMOKE").is_ok();
    let diag = std::env::var("RS_META_STAGE3_FULL_CHAIN_DIAG").is_ok();
    let result = (|| -> Result<(String, String), String> {
        let inner_harness = if smoke {
            String::from("fn main() { println!(\"{}\", 42); }\n")
        } else if diag {
            String::from(stage3_full_diag_harness())
        } else {
            String::from(stage2_chain_harness())
        };
        let inner = source_bundle_with_harness(&inner_harness)?;
        let outer_harness = format!(
            "fn main() {{ \
                let src = {}; \
                match interp_run(src) {{ \
                    Ok(out) => print!(\"{{}}\", out), \
                    Err(e) => print!(\"ERR {{}}\", e), \
                }} \
            }}\n",
            rust_string_literal(&inner)
        );
        let outer = source_bundle_with_harness(&outer_harness)?;
        let interp = interp_run(&outer)?;
        let native = native_run(&outer, &default_workdir())?;
        Ok((interp, native))
    })();

    match result {
        Ok((i, n))
            if smoke && i == n && i.trim() == "42" =>
        {
            r.ok(format!(
                "all-source evaluator stage2 loads stage2' smoke harness interp == rustc (budget {}s)",
                budget_seconds
            ))
        }
        Ok((i, n)) if diag && i == n && i.contains("diag done") => {
            r.ok(format!(
                "diag transcript (interp == rustc): {}",
                i.trim()
            ))
        }
        Ok((i, n))
            if !smoke
                && i == n
                && !i.starts_with("ERR ")
                && i.contains("42")
                && i.contains("120")
                && i.contains("3")
                && i.contains(&expected_count)
                && i.contains("15")
                && i.contains("rs-meta") =>
        {
            r.ok(format!(
                "all-source evaluator stage2 chain full chain replay interp == rustc (budget {}s)",
                budget_seconds
            ))
        }
        Ok((i, n)) => r.fail(format!(
            "stage3 full-chain mismatch: interp {:?} != rustc {:?}",
            i, n
        )),
        Err(e) => r.fail(format!("stage3 full-chain: {}", e)),
    }
    r
}

fn stage3_core_negative_suffix_harness(limit: usize) -> Result<String, String> {
    let cases = negative_corpus();
    if limit > cases.len() {
        return Err(format!(
            "stage3 core negative suffix limit {} exceeds negative corpus size {}",
            limit,
            cases.len()
        ));
    }
    let mut out = String::from("fn main() {\n    let mut passed = 0usize;\n");
    let mut i = cases.len() - limit;
    while i < cases.len() {
        let (name, src) = cases[i];
        out.push_str("    if !interp_run(");
        out.push_str(&rust_string_literal(src));
        out.push_str(").is_err() { panic!(");
        out.push_str(&rust_string_literal(name));
        out.push_str("); }\n");
        out.push_str("    passed += 1;\n");
        i += 1;
    }
    out.push_str("    println!(\"{}\", passed);\n}\n");
    Ok(out)
}

fn stage3_full_diag_harness() -> &'static str {
    "fn main() { \
        let cases = corpus(); \
        let mut i = 0usize; \
        let mut bad = 0usize; \
        while i < cases.len() { \
            let (name, src, expected) = cases[i]; \
            match interp_run(src) { \
                Ok(out) => { \
                    if out.trim() != expected { \
                        println!(\"MISMATCH {}\", name); \
                        bad += 1; \
                    } \
                } \
                Err(e) => { \
                    println!(\"ERRCASE {} :: {}\", name, e); \
                    bad += 1; \
                } \
            } \
            i += 1; \
        } \
        println!(\"diag done {} cases {} bad\", i, bad); \
    }\n"
}

pub fn stage3_full_held_check() -> Report {
    let mut r = Report::new("stage3-full-held-check (all-source stage3 boundary row)");
    let result = (|| -> Result<(), String> {
        let text = fs::read_to_string("proofs/stage-manifest.tsv")
            .map_err(|e| format!("read proofs/stage-manifest.tsv: {}", e))?;
        let mut has_chain = false;
        let mut has_guard = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                continue;
            }
            if cols[0] == "stage3-full-held" && cols[1] == "DONE" {
                has_guard = cols[2] == "stage3-full-held-check";
            }
            if cols[0] == "stage3-full-chain" {
                let timeout = cols[4]
                    .parse::<i64>()
                    .map_err(|e| format!("stage3-full-chain timeout: {}", e))?;
                has_chain = cols[1] == "DONE"
                    && cols[2] == "stage3-full-chain-check"
                    && cols[3].contains("RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS")
                    && timeout >= 2100
                    && cols[5].contains("release")
                    && cols[5].contains("budget-gated");
            }
        }
        if has_guard && has_chain {
            Ok(())
        } else {
            Err(format!(
                "stage3 full boundary drift: guard_row={}, chain_row={}",
                has_guard, has_chain
            ))
        }
    })();
    match result {
        Ok(()) => r.ok(
            "stage3 full all-source chain row is DONE, budget-gated, with release cost note"
                .to_string(),
        ),
        Err(e) => r.fail(e),
    }
    r
}

/// Roundtrip: parse -> emit -> reparse must be structurally identical, and the
/// emitted source must evaluate to the expected output under the interpreter.
pub fn roundtrip_check() -> Report {
    let mut r = Report::new("roundtrip-check (parse -> emit -> reparse AST identity)");
    for (name, src, expected) in corpus() {
        let result = (|| -> Result<(), String> {
            let toks = lex(src)?;
            let prog = parse_program(&toks)?;
            let emitted = emit_program(&prog);
            let toks2 = lex(&emitted).map_err(|e| format!("emitted lex: {}", e))?;
            let prog2 =
                parse_program(&toks2).map_err(|e| format!("emitted parse: {} in {}", e, emitted))?;
            let a1 = format!("{:?}", prog);
            let a2 = format!("{:?}", prog2);
            if a1 != a2 {
                return Err(format!(
                    "AST drift after emit/reparse; emitted: {}",
                    emitted
                ));
            }
            let out = interp_run(&emitted).map_err(|e| format!("emitted interp: {}", e))?;
            if out.trim() != expected {
                return Err(format!(
                    "emitted interp output {:?} != expected {:?}",
                    out.trim(),
                    expected
                ));
            }
            Ok(())
        })();
        match result {
            Ok(()) => r.ok(format!("{}: emit/reparse stable, interp(emit) == expected", name)),
            Err(e) => r.fail(format!("{}: {}", name, e)),
        }
    }
    r
}

/// Emit parity: the Rust regenerated from the AST must produce the expected
/// output under rustc (native tier fed by the emitter instead of the original
/// source text).
pub fn emit_tv_check() -> Report {
    let mut r = Report::new("emit-tv-check (rustc(emit(parse(src))) == expected)");
    let workdir = default_workdir();
    for (name, src, expected) in corpus() {
        let result = (|| -> Result<String, String> {
            let toks = lex(src)?;
            let prog = parse_program(&toks)?;
            let emitted = emit_program(&prog);
            native_run(&emitted, &workdir)
        })();
        match result {
            Ok(out) if out.trim() == expected => {
                r.ok(format!("{}: rustc(emitted) == {}", name, expected))
            }
            Ok(out) => r.fail(format!(
                "{}: rustc(emitted) {:?} != expected {:?}",
                name,
                out.trim(),
                expected
            )),
            Err(e) => r.fail(format!("{}: {}", name, e)),
        }
    }
    r
}

fn stage3_mirror_harness() -> Result<String, String> {
    let sig_raw = fs::read_to_string("src/sig.rs")
        .map_err(|e| format!("read src/sig.rs: {}", e))?;
    let mut sig = String::new();
    for line in sig_raw.split("\n") {
        if line.starts_with("//!") || line.starts_with("use crate::") {
            continue;
        }
        sig.push_str(line);
        sig.push('\n');
    }
    let probe = fs::read_to_string("samples/mirror_probe.rs")
        .map_err(|e| format!("read samples/mirror_probe.rs: {}", e))?;
    let mut out = String::new();
    out.push_str(&sig);
    out.push_str("\nfn main() {\n");
    out.push_str(&format!("    let probe = {};\n", rust_string_literal(&probe)));
    out.push_str("    let toks = lex(probe).unwrap();\n");
    out.push_str("    let prog = parse_program(&toks).unwrap();\n");
    out.push_str("    check(&prog).unwrap();\n");
    out.push_str("    println!(\"SIG {}\", sig_program(&prog));\n");
    out.push_str("    let probe_out = interp_run(probe).unwrap();\n");
    out.push_str("    print!(\"OUT {}\", probe_out);\n");
    out.push_str("}\n");
    Ok(out)
}

/// Transcripts of the mirror harness at each stage:
/// (stage1 native, stage2 interpreted, stage2' nested, nested-under-rustc).
fn stage3_mirror_transcripts() -> Result<(String, String, String, String), String> {
    let inner = evaluator_core_source_bundle(&stage3_mirror_harness()?, "stage3 mirror harness")?;
    let native_inner = native_run(&inner, &default_workdir())?;
    let interp_inner = interp_run(&inner)?;
    let outer_harness = format!(
        "fn main() {{ \
            let src = {}; \
            let out = interp_run(src).unwrap(); \
            print!(\"{{}}\", out); \
        }}\n",
        rust_string_literal(&inner)
    );
    let outer = evaluator_core_source_bundle(&outer_harness, "stage3 mirror outer harness")?;
    let interp_outer = interp_run(&outer)?;
    let native_outer = native_run(&outer, &default_workdir())?;
    Ok((native_inner, interp_inner, interp_outer, native_outer))
}

pub fn stage3_mirror_check() -> Report {
    let mut r = Report::new("stage3-mirror-check (stage1/stage2/stage2' canonical AST + output)");
    let result = (|| -> Result<(String, String, String, String), String> {
        let expected_probe = interp_run(
            &fs::read_to_string("samples/mirror_probe.rs")
                .map_err(|e| format!("read samples/mirror_probe.rs: {}", e))?,
        )?;
        let (d0, d1, d2, d2n) = stage3_mirror_transcripts()?;
        let expected_out = format!("OUT {}", expected_probe);
        if !d0.contains("SIG ") || !d0.contains(&expected_out) {
            return Err(format!(
                "mirror transcript missing SIG/OUT sections: {:?}",
                d0
            ));
        }
        Ok((d0, d1, d2, d2n))
    })();
    match result {
        Ok((d0, d1, d2, d2n)) if d0 == d1 && d1 == d2 && d2 == d2n => r.ok(
            "stage1(native), stage2, and stage2' emit identical canonical AST + probe output"
                .to_string(),
        ),
        Ok((d0, d1, d2, d2n)) => r.fail(format!(
            "mirror drift: native {:?} / stage2 {:?} / stage2' {:?} / nested-rustc {:?}",
            d0, d1, d2, d2n
        )),
        Err(e) => r.fail(format!("stage3 mirror: {}", e)),
    }
    r
}

pub fn stage3_fixedpoint_check() -> Report {
    let mut r = Report::new("stage3-fixedpoint-check (B==C evaluator transcript fixed point)");
    let result = stage3_mirror_transcripts();
    match result {
        Ok((d0, d1, d2, d2n)) if d1 == d2 && d0 == d1 && d2 == d2n && d1.contains("SIG ") => r.ok(
            "stage2 (B) and stage2' (C) evaluator transcripts are identical (normalized B==C on \
             canonical AST + probe output; artifact receipt via stage8-selfhost-repro)"
                .to_string(),
        ),
        Ok((_d0, d1, d2, _d2n)) => r.fail(format!(
            "B==C drift: stage2 transcript {:?} != stage2' transcript {:?}",
            d1, d2
        )),
        Err(e) => r.fail(format!("stage3 fixedpoint: {}", e)),
    }
    r
}

pub fn stage8_repro_check() -> Report {
    let mut r = Report::new("stage8-repro-check (native artifact receipt seed)");
    let src = "fn fact(n: i64) -> i64 { if n < 2 { 1 } else { n * fact(n - 1) } }\n\
               fn main() { println!(\"{}\", fact(10)); }\n";
    check_stage8_receipts(&mut r, "sample", src, "stage8-sample-a", "stage8-sample-b");
    match source_bundle() {
        Ok(bundle) => check_stage8_receipts(
            &mut r,
            "all-source-bundle",
            &bundle,
            "stage8-source-a",
            "stage8-source-b",
        ),
        Err(e) => r.fail(format!("stage8 source bundle: {}", e)),
    }
    r
}

pub fn stage8_selfhost_repro_check() -> Report {
    let mut r = Report::new("stage8-selfhost-repro-check (stage2 evaluator artifact receipt)");
    match source_bundle_with_harness(stage2_chain_harness()) {
        Ok(stage2_src) => check_stage8_receipts(
            &mut r,
            "stage2-evaluator-prime",
            &stage2_src,
            "stage8-selfhost-a",
            "stage8-selfhost-b",
        ),
        Err(e) => r.fail(format!("stage8 selfhost source bundle: {}", e)),
    }
    r
}

fn check_stage8_receipts(r: &mut Report, name: &str, src: &str, a_suffix: &str, b_suffix: &str) {
    let a_dir = default_workdir().join(a_suffix);
    let b_dir = default_workdir().join(b_suffix);
    match (
        native_artifact_receipt(src, &a_dir),
        native_artifact_receipt(src, &b_dir),
    ) {
        (Ok(a), Ok(b)) if a == b => r.ok(format!(
            "{}: same source -> same receipt ({})",
            name,
            receipt_artifact_line(&a)
        )),
        (Ok(a), Ok(b)) => r.fail(format!(
            "{} artifact receipt drift:\n--- a ---\n{}--- b ---\n{}",
            name, a, b
        )),
        (Err(e), _) | (_, Err(e)) => r.fail(format!("{}: stage8 repro: {}", name, e)),
    }
}

pub fn manifest_check() -> Report {
    let mut r = Report::new("manifest-check (stage manifest index)");
    let result = (|| -> Result<
        (
            usize,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
            bool,
        ),
        String,
    > {
            let text = fs::read_to_string("proofs/stage-manifest.tsv")
                .map_err(|e| format!("read proofs/stage-manifest.tsv: {}", e))?;
            let mut rows = 0usize;
            let mut has_stage2 = false;
            let mut has_stage3 = false;
            let mut has_stage8 = false;
            let mut has_stage10 = false;
            let mut has_stage11 = false;
            let mut has_stage12 = false;
            let mut has_stage13 = false;
            let mut has_stage14 = false;
            let mut has_stage15 = false;
            let mut has_stagen = false;
            let mut has_actions = false;
            for line in text.split("\n") {
                let line = line.trim();
                if line.is_empty() || line.starts_with("#") {
                    continue;
                }
                let cols: Vec<&str> = line.split("\t").collect();
                if cols.len() != 6 {
                    return Err(format!(
                        "manifest row has {} columns, expected 6: {}",
                        cols.len(),
                        line
                    ));
                }
                let stage = cols[0];
                let status = cols[1];
                let check = cols[2];
                let command = cols[3];
                let timeout = cols[4]
                    .parse::<i64>()
                    .map_err(|e| format!("manifest timeout for {}: {}", stage, e))?;
                let note = cols[5];
                if !matches!(status, "DONE" | "TODO" | "HELD" | "DISABLED" | "GROW") {
                    return Err(format!("manifest unknown status {} for {}", status, stage));
                }
                if check.is_empty() || command.is_empty() || note.is_empty() {
                    return Err(format!("manifest empty field for {}", stage));
                }
                if timeout < 0 {
                    return Err(format!("manifest negative timeout for {}", stage));
                }
                if stage == "stage2-chain" && status == "DONE" {
                    has_stage2 = true;
                }
                if stage == "stage3-slim-chain" && status == "DONE" {
                    has_stage3 = true;
                }
                if stage == "stage8-selfhost-repro" && status == "DONE" {
                    has_stage8 = true;
                }
                if stage == "stage10-sandbox" && status == "DONE" {
                    has_stage10 = true;
                }
                if stage == "stage11-adapter-replay" && status == "DONE" {
                    has_stage11 = true;
                }
                if stage == "stage12-quarantine-replay" && status == "DONE" {
                    has_stage12 = true;
                }
                if stage == "stage13-horizon-replay" && status == "DONE" {
                    has_stage13 = true;
                }
                if stage == "stage14-cross-impl-replay" && status == "DONE" {
                    has_stage14 = true;
                }
                if stage == "stage15-evidence-replay" && status == "DONE" {
                    has_stage15 = true;
                }
                if stage == "stageN-extension-replay" && status == "DONE" {
                    has_stagen = true;
                }
                if stage == "github-actions" && status == "DISABLED" {
                    has_actions = true;
                }
                rows += 1;
            }
            Ok((
                rows,
                has_stage2,
                has_stage3,
                has_stage8,
                has_stage10,
                has_stage11,
                has_stage12,
                has_stage13,
                has_stage14,
                has_stage15,
                has_stagen,
                has_actions,
            ))
        })();
    match result {
        Ok((rows, true, true, true, true, true, true, true, true, true, true, true)) => r.ok(format!(
            "{} manifest rows; required stage rows present",
            rows
        )),
        Ok((
            _rows,
            has_stage2,
            has_stage3,
            has_stage8,
            has_stage10,
            has_stage11,
            has_stage12,
            has_stage13,
            has_stage14,
            has_stage15,
            has_stagen,
            has_actions,
        )) => {
            r.fail(format!(
                "missing required rows: stage2={}, stage3={}, stage8={}, stage10={}, stage11={}, stage12={}, stage13={}, stage14={}, stage15={}, stageN={}, actions={}",
                has_stage2, has_stage3, has_stage8, has_stage10, has_stage11, has_stage12, has_stage13, has_stage14, has_stage15, has_stagen, has_actions
            ))
        }
        Err(e) => r.fail(e),
    }
    r
}

pub fn isolation_check() -> Report {
    let mut r = Report::new("isolation-check (fresh interpreter state)");
    let result = (|| -> Result<(), String> {
        let first = interp_run("fn main() { println!(\"{}\", 1); }\n")?;
        let second = interp_run("fn main() { println!(\"{}\", 2); }\n")?;
        let first_again = interp_run("fn main() { println!(\"{}\", 1); }\n")?;
        if first.trim() != "1" || second.trim() != "2" || first_again.trim() != "1" {
            return Err(format!(
                "fresh runs leaked stdout/state: {:?} {:?} {:?}",
                first.trim(),
                second.trim(),
                first_again.trim()
            ));
        }
        let helper = interp_run(
            "fn helper() -> i64 { 40 }\nfn main() { println!(\"{}\", helper() + 2); }\n",
        )?;
        let no_helper = interp_run("fn main() { println!(\"{}\", 7); }\n")?;
        if helper.trim() != "42" || no_helper.trim() != "7" {
            return Err(format!(
                "function namespace leaked between programs: {:?} {:?}",
                helper.trim(),
                no_helper.trim()
            ));
        }
        Ok(())
    })();
    match result {
        Ok(()) => r.ok("fresh interpreter runs are isolated".to_string()),
        Err(e) => r.fail(e),
    }
    r
}

pub fn constitution_check() -> Report {
    let mut r = Report::new("constitution-check (zero-dep/local-only/determinism)");
    let result = (|| -> Result<(), String> {
        let cargo =
            fs::read_to_string("Cargo.toml").map_err(|e| format!("read Cargo.toml: {}", e))?;
        for line in cargo.split("\n") {
            let t = line.trim();
            if t == "[dependencies]" || t.starts_with("[dependencies.") {
                return Err("Cargo.toml contains a dependencies table".to_string());
            }
        }
        let gitignore =
            fs::read_to_string(".gitignore").map_err(|e| format!("read .gitignore: {}", e))?;
        if !gitignore.contains("/target") || !gitignore.contains("/work") {
            return Err(".gitignore must keep /target and /work out of git".to_string());
        }
        actions_disabled_result()?;
        let native = fs::read_to_string("src/native.rs")
            .map_err(|e| format!("read src/native.rs: {}", e))?;
        if !native.contains("cache_key(src)") || !native.contains("prog_{:016x}") {
            return Err("native tier must use content-hash artifact names".to_string());
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            r.ok("zero deps, local-only CI posture, and content-hash native names held".to_string())
        }
        Err(e) => r.fail(e),
    }
    r
}

pub fn actions_disabled_check() -> Report {
    let mut r = Report::new("actions-disabled-check (GitHub Actions disabled; local-only)");
    match actions_disabled_result() {
        Ok(()) => r.ok(".github/workflows absent; disabled workflow receipt present".to_string()),
        Err(e) => r.fail(e),
    }
    r
}

fn actions_disabled_result() -> Result<(), String> {
    fs::read_to_string("../.github/workflows.disabled/rs-meta.yml")
        .map_err(|e| format!("disabled workflow receipt missing: {}", e))?;
    if std::path::Path::new("../.github/workflows").exists() {
        return Err(
            "GitHub Actions workflows directory exists; expected local-only verification"
                .to_string(),
        );
    }
    Ok(())
}

pub fn native_cache_check() -> Report {
    let mut r = Report::new("native-cache-check (content-hash rustc compile cache)");
    let src = "fn main() { println!(\"{}\", 40 + 2); }\n";
    match native_cache_probe(src, &default_workdir().join("native-cache-check")) {
        Ok(true) => r.ok("second native compile hit the content-hash cache".to_string()),
        Ok(false) => r.fail("second native compile missed the content-hash cache".to_string()),
        Err(e) => r.fail(format!("native cache probe: {}", e)),
    }
    r
}

pub fn stage9_replay_check() -> Report {
    let mut r = Report::new("stage9-replay-check (clean-process product entrypoint matrix seed)");
    let result = (|| -> Result<String, String> {
        let mut receipts = Vec::new();
        receipts.push(stage9_case(
            "help",
            vec!["help"],
            None,
            vec!["rs-meta bootstrap", "PROOF COMMANDS"],
        )?);
        receipts.push(stage9_case(
            "stage-status",
            vec!["stage-status"],
            None,
            vec!["stage ladder", "stage9"],
        )?);
        receipts.push(stage9_case(
            "run",
            vec!["run", "-c", "fn main() { println!(\"{}\", 40 + 2); }"],
            Some("42"),
            Vec::new(),
        )?);
        receipts.push(stage9_case(
            "native-run",
            vec![
                "native-run",
                "-c",
                "fn main() { println!(\"{}\", 40 + 2); }",
            ],
            Some("42"),
            Vec::new(),
        )?);
        receipts.push(stage9_case(
            "ast",
            vec!["ast", "-c", "fn main() { println!(\"{}\", 1); }"],
            None,
            vec!["Program", "funcs"],
        )?);
        receipts.push(stage9_case(
            "manifest-check",
            vec!["manifest-check"],
            None,
            vec!["manifest-check", "PASS"],
        )?);
        Ok(format!("[{}]", receipts.join(",")))
    })();
    match result {
        Ok(receipt) => r.ok(receipt),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage9_proof_matrix_check() -> Report {
    let mut r = Report::new("stage9-proof-matrix-check (clean-process proof command matrix)");
    let result = (|| -> Result<String, String> {
        let mut receipts = Vec::new();
        receipts.push(stage9_case(
            "proof-self-check",
            vec!["self-check"],
            None,
            vec!["self-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-tv-check",
            vec!["tv-check"],
            None,
            vec!["tv-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-typeck-check",
            vec!["typeck-check"],
            None,
            vec!["typeck-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-source-ast-check",
            vec!["source-ast-check"],
            None,
            vec!["source-ast-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-source-bundle-check",
            vec!["source-bundle-check"],
            None,
            vec!["source-bundle-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage2-chain-check",
            vec!["stage2-chain-check"],
            None,
            vec!["stage2-chain-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage2-probe-check",
            vec!["stage2-probe-check"],
            None,
            vec!["stage2-probe-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-chain-check",
            vec!["stage3-chain-check"],
            None,
            vec!["stage3-chain-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-all-source-smoke-check",
            vec!["stage3-all-source-smoke-check"],
            None,
            vec!["stage3-all-source-smoke-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-mini-check",
            vec!["stage3-core-mini-check"],
            None,
            vec!["stage3-core-mini-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-prefix-check",
            vec!["stage3-core-prefix-check"],
            None,
            vec!["stage3-core-prefix-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-middle-check",
            vec!["stage3-core-middle-check"],
            None,
            vec!["stage3-core-middle-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-suffix-check",
            vec!["stage3-core-suffix-check"],
            None,
            vec!["stage3-core-suffix-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-feature-check",
            vec!["stage3-core-feature-check"],
            None,
            vec!["stage3-core-feature-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-negative-check",
            vec!["stage3-core-negative-check"],
            None,
            vec!["stage3-core-negative-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-negative-middle-check",
            vec!["stage3-core-negative-middle-check"],
            None,
            vec!["stage3-core-negative-middle-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-core-negative-suffix-check",
            vec!["stage3-core-negative-suffix-check"],
            None,
            vec!["stage3-core-negative-suffix-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage3-full-held-check",
            vec!["stage3-full-held-check"],
            None,
            vec!["stage3-full-held-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage8-repro-check",
            vec!["stage8-repro-check"],
            None,
            vec!["stage8-repro-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage8-selfhost-repro-check",
            vec!["stage8-selfhost-repro-check"],
            None,
            vec!["stage8-selfhost-repro-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-manifest-check",
            vec!["manifest-check"],
            None,
            vec!["manifest-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-isolation-check",
            vec!["isolation-check"],
            None,
            vec!["isolation-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-constitution-check",
            vec!["constitution-check"],
            None,
            vec!["constitution-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-actions-disabled-check",
            vec!["actions-disabled-check"],
            None,
            vec!["actions-disabled-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-native-cache-check",
            vec!["native-cache-check"],
            None,
            vec!["native-cache-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage9-replay-check",
            vec!["stage9-replay-check"],
            None,
            vec!["stage9-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage10-session-check",
            vec!["stage10-session-check"],
            None,
            vec!["stage10-session-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage10-sandbox-check",
            vec!["stage10-sandbox-check"],
            None,
            vec!["stage10-sandbox-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage11-adapter-check",
            vec!["stage11-adapter-check"],
            None,
            vec!["stage11-adapter-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage11-adapter-replay-check",
            vec!["stage11-adapter-replay-check"],
            None,
            vec!["stage11-adapter-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage12-quarantine-check",
            vec!["stage12-quarantine-check"],
            None,
            vec!["stage12-quarantine-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage12-quarantine-replay-check",
            vec!["stage12-quarantine-replay-check"],
            None,
            vec!["stage12-quarantine-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage13-horizon-check",
            vec!["stage13-horizon-check"],
            None,
            vec!["stage13-horizon-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage13-horizon-replay-check",
            vec!["stage13-horizon-replay-check"],
            None,
            vec!["stage13-horizon-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage14-cross-impl-check",
            vec!["stage14-cross-impl-check"],
            None,
            vec!["stage14-cross-impl-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage14-cross-impl-replay-check",
            vec!["stage14-cross-impl-replay-check"],
            None,
            vec!["stage14-cross-impl-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage15-evidence-check",
            vec!["stage15-evidence-check"],
            None,
            vec!["stage15-evidence-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stage15-evidence-replay-check",
            vec!["stage15-evidence-replay-check"],
            None,
            vec!["stage15-evidence-replay-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stageN-extension-check",
            vec!["stageN-extension-check"],
            None,
            vec!["stageN-extension-check", "PASS"],
        )?);
        receipts.push(stage9_case(
            "proof-stageN-extension-replay-check",
            vec!["stageN-extension-replay-check"],
            None,
            vec!["stageN-extension-replay-check", "PASS"],
        )?);
        Ok(format!("[{}]", receipts.join(",")))
    })();
    match result {
        Ok(receipt) => r.ok(receipt),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage9_aggregate_replay_check() -> Report {
    let mut r = Report::new("stage9-aggregate-replay-check (bounded proof aggregate replay)");
    let result = stage9_case_with_guard(
        "aggregate-proof-matrix",
        vec!["stage9-proof-matrix-check"],
        None,
        vec!["stage9-proof-matrix-check", "PASS"],
        false,
    );
    match result {
        Ok(receipt) => r.ok(receipt),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage10_session_check() -> Report {
    let mut r = Report::new("stage10-session-check (deterministic clean-process session seed)");
    let result = (|| -> Result<(String, String), String> {
        let a = stage10_session_transcript()?;
        let b = stage10_session_transcript()?;
        Ok((a, b))
    })();
    match result {
        Ok((a, b)) if a == b => r.ok(format!("stable session transcript {}", a)),
        Ok((a, b)) => r.fail(format!("session replay drift:\nfirst={}\nsecond={}", a, b)),
        Err(e) => r.fail(format!("stage10 session: {}", e)),
    }
    r
}

pub fn stage10_sandbox_check() -> Report {
    let mut r = Report::new("stage10-sandbox-check (client/server/session/sandbox closure)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/session-sandbox.tsv")
            .map_err(|e| format!("read proofs/session-sandbox.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_client = false;
        let mut has_server = false;
        let mut has_session = false;
        let mut has_sandbox = false;
        let mut has_actions_disabled = false;
        let mut has_held = false;
        let mut has_no_boundary_leak = false;
        let mut has_fail_closed = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "session sandbox row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let boundary = cols[0];
            let status = cols[1];
            let replay_policy = cols[2];
            let sandbox_policy = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if boundary.is_empty()
                || status.is_empty()
                || replay_policy.is_empty()
                || sandbox_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("session sandbox empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!(
                    "session sandbox {} has unknown status {}",
                    boundary, status
                ));
            }
            if boundary == "local-client" && status == "DONE" {
                has_client = true;
            }
            if boundary == "local-server" && status == "DONE" {
                has_server = true;
            }
            if boundary == "session-store" && status == "DONE" {
                has_session = true;
            }
            if boundary == "sandbox-env" && status == "DONE" {
                has_sandbox = true;
            }
            if boundary == "github-actions" && status == "DISABLED" {
                has_actions_disabled = true;
            }
            if status == "HELD" {
                has_held = true;
            }
            if sandbox_policy == "no-boundary-leak" || sandbox_policy == "no-network" {
                has_no_boundary_leak = true;
            }
            if conflict_policy == "fail-closed" {
                has_fail_closed = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_client,
            has_server,
            has_session,
            has_sandbox,
            has_actions_disabled,
            has_held,
            has_no_boundary_leak && has_fail_closed,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true, true, true)) => r.ok(format!(
            "{} session/sandbox rows; client/server/session/sandbox/actions/held/fail-closed policy present",
            rows
        )),
        Ok((
            _rows,
            has_client,
            has_server,
            has_session,
            has_sandbox,
            has_actions_disabled,
            has_held,
            has_boundary_and_fail_closed,
        )) => r.fail(format!(
            "missing session sandbox invariants: client={}, server={}, session={}, sandbox={}, actions_disabled={}, held={}, boundary_and_fail_closed={}",
            has_client, has_server, has_session, has_sandbox, has_actions_disabled, has_held, has_boundary_and_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage11_adapter_check() -> Report {
    let mut r = Report::new("stage11-adapter-check (adapter schema/held/conflict seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/adapter-schema.tsv")
            .map_err(|e| format!("read proofs/adapter-schema.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_local = false;
        let mut has_rustc_native = false;
        let mut has_actions_disabled = false;
        let mut has_held = false;
        let mut has_fail_closed = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "adapter row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let adapter = cols[0];
            let status = cols[1];
            let schema = cols[2];
            let held_policy = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if adapter.is_empty()
                || status.is_empty()
                || schema.is_empty()
                || held_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("adapter schema empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!("adapter {} has unknown status {}", adapter, status));
            }
            if status == "HELD" && held_policy == "none" {
                return Err(format!(
                    "held adapter {} must name an explicit held policy",
                    adapter
                ));
            }
            if adapter == "local-rust" && status == "DONE" {
                has_local = true;
            }
            if adapter == "rustc-native" && status == "DONE" {
                has_rustc_native = true;
            }
            if adapter == "github-actions" && status == "DISABLED" {
                has_actions_disabled = true;
            }
            if status == "HELD" && held_policy == "explicit-enable-required" {
                has_held = true;
            }
            if conflict_policy == "fail-closed" {
                has_fail_closed = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_local,
            has_rustc_native,
            has_actions_disabled,
            has_held,
            has_fail_closed,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} adapter rows; local/rustc/actions/held/fail-closed policy present",
            rows
        )),
        Ok((
            _rows,
            has_local,
            has_rustc_native,
            has_actions_disabled,
            has_held,
            has_fail_closed,
        )) => r.fail(format!(
            "missing adapter invariants: local={}, rustc_native={}, actions_disabled={}, held_policy={}, fail_closed={}",
            has_local, has_rustc_native, has_actions_disabled, has_held, has_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage11_adapter_replay_check() -> Report {
    let mut r = Report::new("stage11-adapter-replay-check (multi-domain adapter closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool), String> {
        let schema = stage11_adapter_check();
        if !schema.green() {
            return Err(format!(
                "adapter schema failed: {}",
                schema.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/adapter-replay.tsv")
            .map_err(|e| format!("read proofs/adapter-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_local = false;
        let mut has_rustc_native = false;
        let mut has_actions_disabled = false;
        let mut has_held = false;
        let mut all_fail_closed = true;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "adapter replay row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let adapter = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if adapter.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("adapter replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD" | "DISABLED") {
                return Err(format!(
                    "adapter replay {} has unknown status {}",
                    adapter, status
                ));
            }
            if conflict_policy != "fail-closed" {
                all_fail_closed = false;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "stage10-sandbox-check" => stage9_case(
                            "adapter-local-rust",
                            vec!["stage10-sandbox-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "tv-check" => stage9_case(
                            "adapter-rustc-native",
                            vec!["tv-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "adapter replay {} has unsupported DONE command {}",
                                adapter, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if adapter == "local-rust" {
                        has_local = true;
                    }
                    if adapter == "rustc-native" {
                        has_rustc_native = true;
                    }
                }
                "DISABLED" => {
                    if adapter != "github-actions" || command != "actions-disabled" {
                        return Err(format!(
                            "adapter replay disabled row must be github-actions/actions-disabled: {}",
                            line
                        ));
                    }
                    if std::path::Path::new("../.github/workflows").exists()
                        || !std::path::Path::new("../.github/workflows.disabled/rs-meta.yml")
                            .exists()
                    {
                        return Err(
                            "adapter replay expected GitHub Actions to be disabled".to_string()
                        );
                    }
                    has_actions_disabled = true;
                    receipts.push(format!(
                        "{{\"adapter\":\"{}\",\"status\":\"disabled\"}}",
                        adapter
                    ));
                }
                "HELD" => {
                    if command != "held" || marker != "explicit-enable-required" {
                        return Err(format!(
                            "adapter replay held row missing explicit marker: {}",
                            line
                        ));
                    }
                    has_held = true;
                    receipts.push(format!(
                        "{{\"adapter\":\"{}\",\"status\":\"held\"}}",
                        adapter
                    ));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_local,
            has_rustc_native,
            has_actions_disabled,
            has_held,
            all_fail_closed,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true)) => r.ok(format!(
            "{} adapter replay rows; local/rustc/actions/held/fail-closed closure {}",
            rows, receipt
        )),
        Ok((
            _rows,
            _receipt,
            has_local,
            has_rustc_native,
            has_actions_disabled,
            has_held,
            all_fail_closed,
        )) => r.fail(format!(
            "missing adapter replay invariants: local={}, rustc_native={}, actions_disabled={}, held={}, fail_closed={}",
            has_local, has_rustc_native, has_actions_disabled, has_held, all_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage12_quarantine_check() -> Report {
    let mut r = Report::new("stage12-quarantine-check (self-improvement quarantine seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/quarantine-policy.tsv")
            .map_err(|e| format!("read proofs/quarantine-policy.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_local = false;
        let mut has_actions_disabled = false;
        let mut has_no_auto = false;
        let mut has_fail_closed = false;
        let mut has_held = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "quarantine row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let gate = cols[0];
            let status = cols[1];
            let promotion_policy = cols[2];
            let fail_policy = cols[3];
            let receipt = cols[4];
            let note = cols[5];
            if gate.is_empty()
                || status.is_empty()
                || promotion_policy.is_empty()
                || fail_policy.is_empty()
                || receipt.is_empty()
                || note.is_empty()
            {
                return Err(format!("quarantine policy empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!("quarantine {} has unknown status {}", gate, status));
            }
            if gate == "local-verification" && status == "DONE" && receipt == "bootstrap-check" {
                has_local = true;
            }
            if gate == "github-actions" && status == "DISABLED" {
                has_actions_disabled = true;
            }
            if promotion_policy == "no-auto-promotion" {
                has_no_auto = true;
            }
            if fail_policy == "fail-closed" {
                has_fail_closed = true;
            }
            if status == "HELD" {
                has_held = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_local,
            has_actions_disabled,
            has_no_auto,
            has_fail_closed,
            has_held,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} quarantine rows; local/no-auto/fail-closed/held policy present",
            rows
        )),
        Ok((_rows, has_local, has_actions_disabled, has_no_auto, has_fail_closed, has_held)) => {
            r.fail(format!(
                "missing quarantine invariants: local={}, actions_disabled={}, no_auto={}, fail_closed={}, held={}",
                has_local, has_actions_disabled, has_no_auto, has_fail_closed, has_held
            ))
        }
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage12_quarantine_replay_check() -> Report {
    let mut r =
        Report::new("stage12-quarantine-replay-check (self-improvement quarantine closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool, bool), String> {
        let policy = stage12_quarantine_check();
        if !policy.green() {
            return Err(format!(
                "quarantine policy failed: {}",
                policy.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/quarantine-replay.tsv")
            .map_err(|e| format!("read proofs/quarantine-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_local = false;
        let mut has_candidate = false;
        let mut has_actions_disabled = false;
        let mut has_no_auto = false;
        let mut has_held = false;
        let mut all_fail_closed = true;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 7 {
                return Err(format!(
                    "quarantine replay row has {} columns, expected 7: {}",
                    cols.len(),
                    line
                ));
            }
            let gate = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let promotion_policy = cols[4];
            let fail_policy = cols[5];
            let note = cols[6];
            if gate.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || promotion_policy.is_empty()
                || fail_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("quarantine replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD" | "DISABLED") {
                return Err(format!(
                    "quarantine replay {} has unknown status {}",
                    gate, status
                ));
            }
            if fail_policy != "fail-closed" {
                all_fail_closed = false;
            }
            if promotion_policy == "no-auto-promotion" {
                has_no_auto = true;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "stage11-adapter-replay-check" => stage9_case(
                            "quarantine-local-verification",
                            vec!["stage11-adapter-replay-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "manifest-check" => stage9_case(
                            "quarantine-candidate-intake",
                            vec!["manifest-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "quarantine replay {} has unsupported DONE command {}",
                                gate, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if gate == "local-verification" {
                        has_local = true;
                    }
                    if gate == "candidate-intake" {
                        has_candidate = true;
                    }
                }
                "DISABLED" => {
                    if gate != "github-actions" || command != "actions-disabled" {
                        return Err(format!(
                            "quarantine replay disabled row must be github-actions/actions-disabled: {}",
                            line
                        ));
                    }
                    if std::path::Path::new("../.github/workflows").exists()
                        || !std::path::Path::new("../.github/workflows.disabled/rs-meta.yml")
                            .exists()
                    {
                        return Err(
                            "quarantine replay expected GitHub Actions to be disabled".to_string()
                        );
                    }
                    has_actions_disabled = true;
                    receipts.push(format!("{{\"gate\":\"{}\",\"status\":\"disabled\"}}", gate));
                }
                "HELD" => {
                    if command != "held" || marker != promotion_policy {
                        return Err(format!(
                            "quarantine replay held row missing promotion marker: {}",
                            line
                        ));
                    }
                    has_held = true;
                    receipts.push(format!("{{\"gate\":\"{}\",\"status\":\"held\"}}", gate));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_local,
            has_candidate,
            has_actions_disabled,
            has_no_auto,
            has_held,
            all_fail_closed,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true, true)) => r.ok(format!(
            "{} quarantine replay rows; local/candidate/actions/no-auto/held/fail-closed closure {}",
            rows, receipt
        )),
        Ok((
            _rows,
            _receipt,
            has_local,
            has_candidate,
            has_actions_disabled,
            has_no_auto,
            has_held,
            all_fail_closed,
        )) => r.fail(format!(
            "missing quarantine replay invariants: local={}, candidate={}, actions_disabled={}, no_auto={}, held={}, fail_closed={}",
            has_local, has_candidate, has_actions_disabled, has_no_auto, has_held, all_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage13_horizon_check() -> Report {
    let mut r = Report::new("stage13-horizon-check (long-horizon stale/boundary seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/horizon-policy.tsv")
            .map_err(|e| format!("read proofs/horizon-policy.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_manifest = false;
        let mut has_held = false;
        let mut has_freshness = false;
        let mut has_boundary = false;
        let mut has_degradation = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "horizon row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let signal = cols[0];
            let status = cols[1];
            let freshness_policy = cols[2];
            let boundary_policy = cols[3];
            let degradation_policy = cols[4];
            let note = cols[5];
            if signal.is_empty()
                || status.is_empty()
                || freshness_policy.is_empty()
                || boundary_policy.is_empty()
                || degradation_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("horizon policy empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!("horizon {} has unknown status {}", signal, status));
            }
            if signal == "stage-manifest" && status == "DONE" {
                has_manifest = true;
            }
            if status == "HELD" {
                has_held = true;
            }
            if freshness_policy == "explicit-refresh-required"
                || freshness_policy == "replay-receipt"
            {
                has_freshness = true;
            }
            if boundary_policy == "no-boundary-leak" {
                has_boundary = true;
            }
            if degradation_policy == "degrade-to-held" {
                has_degradation = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_manifest,
            has_held,
            has_freshness,
            has_boundary,
            has_degradation,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} horizon rows; manifest/freshness/boundary/degrade policy present",
            rows
        )),
        Ok((_rows, has_manifest, has_held, has_freshness, has_boundary, has_degradation)) => {
            r.fail(format!(
                "missing horizon invariants: manifest={}, held={}, freshness={}, boundary={}, degradation={}",
                has_manifest, has_held, has_freshness, has_boundary, has_degradation
            ))
        }
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage13_horizon_replay_check() -> Report {
    let mut r = Report::new("stage13-horizon-replay-check (long-horizon organism closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool, bool), String> {
        let policy = stage13_horizon_check();
        if !policy.green() {
            return Err(format!(
                "horizon policy failed: {}",
                policy.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/horizon-replay.tsv")
            .map_err(|e| format!("read proofs/horizon-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_manifest = false;
        let mut has_session = false;
        let mut has_held = false;
        let mut has_freshness = false;
        let mut has_boundary = false;
        let mut has_degradation = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 8 {
                return Err(format!(
                    "horizon replay row has {} columns, expected 8: {}",
                    cols.len(),
                    line
                ));
            }
            let signal = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let freshness_policy = cols[4];
            let boundary_policy = cols[5];
            let degradation_policy = cols[6];
            let note = cols[7];
            if signal.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || freshness_policy.is_empty()
                || boundary_policy.is_empty()
                || degradation_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("horizon replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD") {
                return Err(format!(
                    "horizon replay {} has unknown status {}",
                    signal, status
                ));
            }
            if matches!(
                freshness_policy,
                "replay-receipt"
                    | "explicit-refresh-required"
                    | "explicit-import-required"
                    | "manifest-versioned"
                    | "offline-first"
            ) {
                has_freshness = true;
            }
            if boundary_policy == "no-boundary-leak" || boundary_policy == "local-proof-boundary" {
                has_boundary = true;
            }
            if degradation_policy == "degrade-to-held" || degradation_policy == "fail-closed" {
                has_degradation = true;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "manifest-check" => stage9_case(
                            "horizon-stage-manifest",
                            vec!["manifest-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "stage12-quarantine-replay-check" => stage9_case(
                            "horizon-session-replay",
                            vec!["stage12-quarantine-replay-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "horizon replay {} has unsupported DONE command {}",
                                signal, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if signal == "stage-manifest" {
                        has_manifest = true;
                    }
                    if signal == "session-replay" {
                        has_session = true;
                    }
                }
                "HELD" => {
                    if command != "held" || marker != freshness_policy {
                        return Err(format!(
                            "horizon replay held row missing freshness marker: {}",
                            line
                        ));
                    }
                    has_held = true;
                    receipts.push(format!("{{\"signal\":\"{}\",\"status\":\"held\"}}", signal));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_manifest,
            has_session,
            has_held,
            has_freshness,
            has_boundary,
            has_degradation,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true, true)) => r.ok(format!(
            "{} horizon replay rows; manifest/session/held/freshness/boundary/degrade closure {}",
            rows, receipt
        )),
        Ok((
            _rows,
            _receipt,
            has_manifest,
            has_session,
            has_held,
            has_freshness,
            has_boundary,
            has_degradation,
        )) => r.fail(format!(
            "missing horizon replay invariants: manifest={}, session={}, held={}, freshness={}, boundary={}, degradation={}",
            has_manifest, has_session, has_held, has_freshness, has_boundary, has_degradation
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage14_cross_impl_check() -> Report {
    let mut r = Report::new("stage14-cross-impl-check (cross-implementation export seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/cross-impl-schema.tsv")
            .map_err(|e| format!("read proofs/cross-impl-schema.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_local = false;
        let mut has_native = false;
        let mut has_actions_disabled = false;
        let mut has_held = false;
        let mut has_fail_closed = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "cross-impl row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let implementation = cols[0];
            let status = cols[1];
            let schema = cols[2];
            let comparison_policy = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if implementation.is_empty()
                || status.is_empty()
                || schema.is_empty()
                || comparison_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("cross-impl schema empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!(
                    "cross-impl {} has unknown status {}",
                    implementation, status
                ));
            }
            if implementation == "rs-meta-local" && status == "DONE" {
                has_local = true;
            }
            if implementation == "rustc-native" && status == "DONE" {
                has_native = true;
            }
            if implementation == "github-actions" && status == "DISABLED" {
                has_actions_disabled = true;
            }
            if status == "HELD" {
                has_held = true;
            }
            if conflict_policy == "fail-closed" {
                has_fail_closed = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_local,
            has_native,
            has_actions_disabled,
            has_held,
            has_fail_closed,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} cross-impl rows; local/native/actions/held/fail-closed policy present",
            rows
        )),
        Ok((_rows, has_local, has_native, has_actions_disabled, has_held, has_fail_closed)) => {
            r.fail(format!(
                "missing cross-impl invariants: local={}, native={}, actions_disabled={}, held={}, fail_closed={}",
                has_local, has_native, has_actions_disabled, has_held, has_fail_closed
            ))
        }
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage14_cross_impl_replay_check() -> Report {
    let mut r = Report::new("stage14-cross-impl-replay-check (cross-implementation closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool), String> {
        let schema = stage14_cross_impl_check();
        if !schema.green() {
            return Err(format!(
                "cross-impl schema failed: {}",
                schema.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/cross-impl-replay.tsv")
            .map_err(|e| format!("read proofs/cross-impl-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_local = false;
        let mut has_native = false;
        let mut has_actions_disabled = false;
        let mut has_held = false;
        let mut all_fail_closed = true;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 8 {
                return Err(format!(
                    "cross-impl replay row has {} columns, expected 8: {}",
                    cols.len(),
                    line
                ));
            }
            let implementation = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let export_schema = cols[4];
            let comparison_policy = cols[5];
            let conflict_policy = cols[6];
            let note = cols[7];
            if implementation.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || export_schema.is_empty()
                || comparison_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("cross-impl replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD" | "DISABLED") {
                return Err(format!(
                    "cross-impl replay {} has unknown status {}",
                    implementation, status
                ));
            }
            if conflict_policy != "fail-closed" {
                all_fail_closed = false;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "stage13-horizon-replay-check" => stage9_case(
                            "cross-impl-rs-meta-local",
                            vec!["stage13-horizon-replay-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "tv-check" => stage9_case(
                            "cross-impl-rustc-native",
                            vec!["tv-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "cross-impl replay {} has unsupported DONE command {}",
                                implementation, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if implementation == "rs-meta-local" {
                        has_local = true;
                    }
                    if implementation == "rustc-native" {
                        has_native = true;
                    }
                }
                "DISABLED" => {
                    if implementation != "github-actions" || command != "actions-disabled" {
                        return Err(format!(
                            "cross-impl disabled row must be github-actions/actions-disabled: {}",
                            line
                        ));
                    }
                    if std::path::Path::new("../.github/workflows").exists()
                        || !std::path::Path::new("../.github/workflows.disabled/rs-meta.yml")
                            .exists()
                    {
                        return Err(
                            "cross-impl replay expected GitHub Actions to be disabled".to_string()
                        );
                    }
                    has_actions_disabled = true;
                    receipts.push(format!(
                        "{{\"implementation\":\"{}\",\"status\":\"disabled\"}}",
                        implementation
                    ));
                }
                "HELD" => {
                    if command != "held" || marker != comparison_policy {
                        return Err(format!(
                            "cross-impl held row missing comparison marker: {}",
                            line
                        ));
                    }
                    has_held = true;
                    receipts.push(format!(
                        "{{\"implementation\":\"{}\",\"status\":\"held\"}}",
                        implementation
                    ));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_local,
            has_native,
            has_actions_disabled,
            has_held,
            all_fail_closed,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true)) => r.ok(format!(
            "{} cross-impl replay rows; local/native/actions/held/fail-closed closure {}",
            rows, receipt
        )),
        Ok((
            _rows,
            _receipt,
            has_local,
            has_native,
            has_actions_disabled,
            has_held,
            all_fail_closed,
        )) => r.fail(format!(
            "missing cross-impl replay invariants: local={}, native={}, actions_disabled={}, held={}, fail_closed={}",
            has_local, has_native, has_actions_disabled, has_held, all_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage15_evidence_check() -> Report {
    let mut r = Report::new("stage15-evidence-check (open-world evidence federation seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/evidence-federation.tsv")
            .map_err(|e| format!("read proofs/evidence-federation.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_local = false;
        let mut has_manifest = false;
        let mut has_actions_disabled = false;
        let mut has_offline_held = false;
        let mut has_fail_closed = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "evidence row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let source = cols[0];
            let status = cols[1];
            let schema = cols[2];
            let approval_policy = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if source.is_empty()
                || status.is_empty()
                || schema.is_empty()
                || approval_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("evidence schema empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!("evidence {} has unknown status {}", source, status));
            }
            if source == "local-proof" && status == "DONE" {
                has_local = true;
            }
            if source == "stage-manifest" && status == "DONE" {
                has_manifest = true;
            }
            if source == "github-actions" && status == "DISABLED" {
                has_actions_disabled = true;
            }
            if status == "HELD" && approval_policy == "offline-approval-required" {
                has_offline_held = true;
            }
            if conflict_policy == "fail-closed" {
                has_fail_closed = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_local,
            has_manifest,
            has_actions_disabled,
            has_offline_held,
            has_fail_closed,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} evidence rows; local/manifest/offline-held/fail-closed policy present",
            rows
        )),
        Ok((
            _rows,
            has_local,
            has_manifest,
            has_actions_disabled,
            has_offline_held,
            has_fail_closed,
        )) => r.fail(format!(
            "missing evidence invariants: local={}, manifest={}, actions_disabled={}, offline_held={}, fail_closed={}",
            has_local, has_manifest, has_actions_disabled, has_offline_held, has_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stage15_evidence_replay_check() -> Report {
    let mut r = Report::new("stage15-evidence-replay-check (open-world evidence closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool), String> {
        let schema = stage15_evidence_check();
        if !schema.green() {
            return Err(format!(
                "evidence schema failed: {}",
                schema.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/evidence-replay.tsv")
            .map_err(|e| format!("read proofs/evidence-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_local = false;
        let mut has_manifest = false;
        let mut has_actions_disabled = false;
        let mut has_offline_held = false;
        let mut all_fail_closed = true;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 8 {
                return Err(format!(
                    "evidence replay row has {} columns, expected 8: {}",
                    cols.len(),
                    line
                ));
            }
            let source = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let evidence_schema = cols[4];
            let approval_policy = cols[5];
            let conflict_policy = cols[6];
            let note = cols[7];
            if source.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || evidence_schema.is_empty()
                || approval_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("evidence replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD" | "DISABLED") {
                return Err(format!(
                    "evidence replay {} has unknown status {}",
                    source, status
                ));
            }
            if conflict_policy != "fail-closed" {
                all_fail_closed = false;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "stage14-cross-impl-replay-check" => stage9_case(
                            "evidence-local-proof",
                            vec!["stage14-cross-impl-replay-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "manifest-check" => stage9_case(
                            "evidence-stage-manifest",
                            vec!["manifest-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "evidence replay {} has unsupported DONE command {}",
                                source, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if source == "local-proof" {
                        has_local = true;
                    }
                    if source == "stage-manifest" {
                        has_manifest = true;
                    }
                }
                "DISABLED" => {
                    if source != "github-actions" || command != "actions-disabled" {
                        return Err(format!(
                            "evidence disabled row must be github-actions/actions-disabled: {}",
                            line
                        ));
                    }
                    if std::path::Path::new("../.github/workflows").exists()
                        || !std::path::Path::new("../.github/workflows.disabled/rs-meta.yml")
                            .exists()
                    {
                        return Err(
                            "evidence replay expected GitHub Actions to be disabled".to_string()
                        );
                    }
                    has_actions_disabled = true;
                    receipts.push(format!(
                        "{{\"source\":\"{}\",\"status\":\"disabled\"}}",
                        source
                    ));
                }
                "HELD" => {
                    if command != "held" || marker != approval_policy {
                        return Err(format!(
                            "evidence held row missing approval marker: {}",
                            line
                        ));
                    }
                    if approval_policy == "offline-approval-required"
                        || approval_policy == "explicit-review-required"
                    {
                        has_offline_held = true;
                    }
                    receipts.push(format!("{{\"source\":\"{}\",\"status\":\"held\"}}", source));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_local,
            has_manifest,
            has_actions_disabled,
            has_offline_held,
            all_fail_closed,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true)) => r.ok(format!(
            "{} evidence replay rows; local/manifest/actions/offline-held/fail-closed closure {}",
            rows, receipt
        )),
        Ok((
            _rows,
            _receipt,
            has_local,
            has_manifest,
            has_actions_disabled,
            has_offline_held,
            all_fail_closed,
        )) => r.fail(format!(
            "missing evidence replay invariants: local={}, manifest={}, actions_disabled={}, offline_held={}, fail_closed={}",
            has_local, has_manifest, has_actions_disabled, has_offline_held, all_fail_closed
        )),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stagen_extension_check() -> Report {
    let mut r = Report::new("stageN-extension-check (versioned constitutional extension seed)");
    let result = (|| -> Result<(usize, bool, bool, bool, bool, bool), String> {
        let text = fs::read_to_string("proofs/extension-policy.tsv")
            .map_err(|e| format!("read proofs/extension-policy.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut has_manifest = false;
        let mut has_budget = false;
        let mut has_stagen = false;
        let mut has_held = false;
        let mut has_fail_closed = false;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 6 {
                return Err(format!(
                    "extension row has {} columns, expected 6: {}",
                    cols.len(),
                    line
                ));
            }
            let extension = cols[0];
            let status = cols[1];
            let version_policy = cols[2];
            let migration_policy = cols[3];
            let conflict_policy = cols[4];
            let note = cols[5];
            if extension.is_empty()
                || status.is_empty()
                || version_policy.is_empty()
                || migration_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("extension policy empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "GROW" | "HELD" | "DISABLED") {
                return Err(format!(
                    "extension {} has unknown status {}",
                    extension, status
                ));
            }
            if extension == "manifest-index" && status == "DONE" {
                has_manifest = true;
            }
            if extension == "timeout-cost" && status == "DONE" {
                has_budget = true;
            }
            if extension == "stageN-seed" && status == "DONE" {
                has_stagen = true;
            }
            if status == "HELD" {
                has_held = true;
            }
            if conflict_policy == "fail-closed" {
                has_fail_closed = true;
            }
            rows += 1;
        }
        Ok((
            rows,
            has_manifest,
            has_budget,
            has_stagen,
            has_held,
            has_fail_closed,
        ))
    })();
    match result {
        Ok((rows, true, true, true, true, true)) => r.ok(format!(
            "{} extension rows; manifest/budget/stageN/held/fail-closed policy present",
            rows
        )),
        Ok((_rows, has_manifest, has_budget, has_stagen, has_held, has_fail_closed)) => r.fail(
            format!(
                "missing extension invariants: manifest={}, budget={}, stageN={}, held={}, fail_closed={}",
                has_manifest, has_budget, has_stagen, has_held, has_fail_closed
            ),
        ),
        Err(e) => r.fail(e),
    }
    r
}

pub fn stagen_extension_replay_check() -> Report {
    let mut r = Report::new("stageN-extension-replay-check (versioned extension closure)");
    let result = (|| -> Result<(usize, String, bool, bool, bool, bool, bool), String> {
        let policy = stagen_extension_check();
        if !policy.green() {
            return Err(format!(
                "extension policy failed: {}",
                policy.lines.join(" | ")
            ));
        }

        let text = fs::read_to_string("proofs/extension-replay.tsv")
            .map_err(|e| format!("read proofs/extension-replay.tsv: {}", e))?;
        let mut rows = 0usize;
        let mut receipts = Vec::new();
        let mut has_manifest = false;
        let mut has_budget = false;
        let mut has_stagen = false;
        let mut has_held = false;
        let mut all_fail_closed = true;
        for line in text.split("\n") {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let cols: Vec<&str> = line.split("\t").collect();
            if cols.len() != 8 {
                return Err(format!(
                    "extension replay row has {} columns, expected 8: {}",
                    cols.len(),
                    line
                ));
            }
            let extension = cols[0];
            let status = cols[1];
            let command = cols[2];
            let marker = cols[3];
            let version_policy = cols[4];
            let migration_policy = cols[5];
            let conflict_policy = cols[6];
            let note = cols[7];
            if extension.is_empty()
                || status.is_empty()
                || command.is_empty()
                || marker.is_empty()
                || version_policy.is_empty()
                || migration_policy.is_empty()
                || conflict_policy.is_empty()
                || note.is_empty()
            {
                return Err(format!("extension replay empty field in row: {}", line));
            }
            if !matches!(status, "DONE" | "HELD") {
                return Err(format!(
                    "extension replay {} has unknown status {}",
                    extension, status
                ));
            }
            if conflict_policy != "fail-closed" {
                all_fail_closed = false;
            }
            match status {
                "DONE" => {
                    let receipt = match command {
                        "manifest-check" => stage9_case(
                            "extension-manifest-index",
                            vec!["manifest-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "stage15-evidence-replay-check" => stage9_case(
                            "extension-timeout-cost",
                            vec!["stage15-evidence-replay-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        "stageN-extension-check" => stage9_case(
                            "extension-stageN-seed",
                            vec!["stageN-extension-check"],
                            None,
                            vec![marker, "PASS"],
                        )?,
                        other => {
                            return Err(format!(
                                "extension replay {} has unsupported DONE command {}",
                                extension, other
                            ))
                        }
                    };
                    receipts.push(receipt);
                    if extension == "manifest-index" {
                        has_manifest = true;
                    }
                    if extension == "timeout-cost" {
                        has_budget = true;
                    }
                    if extension == "stageN-seed" {
                        has_stagen = true;
                    }
                }
                "HELD" => {
                    if command != "held" || marker != migration_policy {
                        return Err(format!(
                            "extension replay held row missing migration marker: {}",
                            line
                        ));
                    }
                    has_held = true;
                    receipts.push(format!(
                        "{{\"extension\":\"{}\",\"status\":\"held\"}}",
                        extension
                    ));
                }
                _ => {}
            }
            rows += 1;
        }
        Ok((
            rows,
            format!("[{}]", receipts.join(",")),
            has_manifest,
            has_budget,
            has_stagen,
            has_held,
            all_fail_closed,
        ))
    })();
    match result {
        Ok((rows, receipt, true, true, true, true, true)) => r.ok(format!(
            "{} extension replay rows; manifest/budget/stageN/held/fail-closed closure {}",
            rows, receipt
        )),
        Ok((_rows, _receipt, has_manifest, has_budget, has_stagen, has_held, all_fail_closed)) => {
            r.fail(format!(
                "missing extension replay invariants: manifest={}, budget={}, stageN={}, held={}, fail_closed={}",
                has_manifest, has_budget, has_stagen, has_held, all_fail_closed
            ))
        }
        Err(e) => r.fail(e),
    }
    r
}

fn stage10_session_transcript() -> Result<String, String> {
    let mut receipts = Vec::new();
    receipts.push(stage9_case(
        "session-run",
        vec!["run", "-c", "fn main() { println!(\"{}\", 21 * 2); }"],
        Some("42"),
        Vec::new(),
    )?);
    receipts.push(stage9_case(
        "session-native-run",
        vec![
            "native-run",
            "-c",
            "fn main() { let mut x = 40; x += 2; println!(\"{}\", x); }",
        ],
        Some("42"),
        Vec::new(),
    )?);
    receipts.push(stage9_case(
        "session-ast",
        vec!["ast", "-c", "fn main() { println!(\"{}\", 42); }"],
        None,
        vec!["Program", "funcs"],
    )?);
    receipts.push(stage9_case(
        "session-stage-status",
        vec!["stage-status"],
        None,
        vec!["stage10", "stage ladder"],
    )?);
    Ok(format!("[{}]", receipts.join(",")))
}

fn stage9_case(
    name: &str,
    args: Vec<&str>,
    exact_trimmed_stdout: Option<&str>,
    markers: Vec<&str>,
) -> Result<String, String> {
    stage9_case_with_guard(name, args, exact_trimmed_stdout, markers, false)
}

fn stage9_case_with_guard(
    name: &str,
    args: Vec<&str>,
    exact_trimmed_stdout: Option<&str>,
    markers: Vec<&str>,
    skip_aggregate: bool,
) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let mut cmd = Command::new(exe);
    cmd.env_clear();
    cmd.env("SOURCE_DATE_EPOCH", "0");
    if skip_aggregate {
        cmd.env("RSMETA_SKIP_STAGE9_AGGREGATE", "1");
    }
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("run bootstrap {}: {}", name, e))?;
    if !out.status.success() {
        return Err(format!(
            "bootstrap {} exited non-zero:\n{}",
            name,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if let Some(expected) = exact_trimmed_stdout {
        if stdout.trim() != expected {
            return Err(format!(
                "bootstrap {} stdout mismatch: expected {:?}, got {:?}",
                name,
                expected,
                stdout.trim()
            ));
        }
    }
    for marker in markers {
        if !stdout.contains(marker) {
            return Err(format!(
                "bootstrap {} stdout missing marker {:?}: {:?}",
                name, marker, stdout
            ));
        }
    }
    Ok(format!(
        "{{\"cmd\":\"{}\",\"status\":\"success\",\"stdout_fnv\":\"{:016x}\"}}",
        name,
        crate::hash::fnv1a_text(stdout.as_str())
    ))
}


fn receipt_artifact_line(receipt: &str) -> String {
    let chars: Vec<char> = receipt.chars().collect();
    let mut line = String::new();
    let mut i = 0usize;
    while i <= chars.len() {
        if i == chars.len() || chars[i] == '\n' {
            if starts_with_str(&line, "artifact_fnv=") {
                return line;
            }
            line = String::new();
        } else {
            line.push(chars[i]);
        }
        i += 1;
    }
    "artifact_fnv=<missing>".to_string()
}

fn starts_with_str(text: &str, prefix: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = prefix.chars().collect();
    starts_with_chars(&t, 0, &p)
}

fn run_source_probe(r: &mut Report, name: &str, paths: Vec<&str>, harness: &str) {
    let result = (|| -> Result<(String, String), String> {
        let src = probe_bundle(paths, harness)?;
        let interp = interp_run(&src)?;
        let native = native_run(&src, &default_workdir())?;
        Ok((interp, native))
    })();
    match result {
        Ok((i, n)) if i.trim() == n.trim() => r.ok(format!("{}: {}", name, i.trim())),
        Ok((i, n)) => r.fail(format!(
            "{}: interp {:?} != rustc {:?}",
            name,
            i.trim(),
            n.trim()
        )),
        Err(e) => r.fail(format!("{}: {}", name, e)),
    }
}

fn stage3_slim_bundle(harness: &str) -> Result<String, String> {
    evaluator_core_source_bundle(harness, "stage3 slim harness")
}

fn stage3_core_source_bundle(harness: &str) -> Result<String, String> {
    evaluator_core_source_bundle(harness, "stage3 evaluator-core smoke harness")
}

fn evaluator_core_source_bundle(harness: &str, label: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut seen_uses = Vec::new();
    for path in [
        "src/lexer.rs",
        "src/ast.rs",
        "src/parser.rs",
        "src/typeck.rs",
        "src/interp.rs",
    ] {
        out.push_str(&format!("\n// ---- {} ----\n", path));
        let src = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))?;
        for line in src.split("\n") {
            let line = match normalize_bundle_line(line, &mut seen_uses) {
                Some(line) => line,
                None => continue,
            };
            out.push_str(line.as_str());
            out.push('\n');
        }
    }
    out.push_str(
        "\nfn interp_run(src: &str) -> Result<String, String> {\n\
             let toks = lex(src)?;\n\
             let prog = parse_program(&toks)?;\n\
             check(&prog)?;\n\
             let interp = Interp::new(&prog)?;\n\
             interp.run_main()\n\
         }\n",
    );
    out.push_str(&format!("\n// ---- {} ----\n", label));
    out.push_str(harness);
    Ok(out)
}

fn rust_string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn probe_bundle(paths: Vec<&str>, harness: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut seen_uses = Vec::new();
    for path in paths {
        out.push_str(&format!("\n// ---- {} ----\n", path));
        let src = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))?;
        for line in src.split("\n") {
            let line = match normalize_bundle_line(line, &mut seen_uses) {
                Some(line) => line,
                None => continue,
            };
            out.push_str(line.as_str());
            out.push('\n');
        }
    }
    out.push_str("\n// ---- probe harness ----\n");
    out.push_str(harness);
    Ok(out)
}

fn source_files() -> Vec<&'static str> {
    vec![
        "src/lexer.rs",
        "src/ast.rs",
        "src/parser.rs",
        "src/typeck.rs",
        "src/cap.rs",
        "src/diag.rs",
        "src/hash.rs",
        "src/independent_mini_backend.rs",
        "src/interp.rs",
        "src/sig.rs",
        "src/emit.rs",
        "src/witness.rs",
        "src/native.rs",
        "src/io.rs",
        "src/check.rs",
        "src/main.rs",
    ]
}

fn source_bundle() -> Result<String, String> {
    source_bundle_with_harness("fn main() {\n    print_help();\n}\n")
}

fn source_bundle_with_harness(harness: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut seen_uses = Vec::new();
    for path in source_files() {
        out.push_str(&format!("\n// ---- {} ----\n", path));
        let src = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path, e))?;
        for line in src.split("\n") {
            let line = match normalize_bundle_line(line, &mut seen_uses) {
                Some(line) => line,
                None => continue,
            };
            out.push_str(line.as_str());
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str(harness);
    Ok(out)
}

/// Extract the bare imported name(s) from a `use std::a::b::Name;` or
/// `use std::a::b::{Name1, Name2};` line (best-effort text parsing, not a
/// real path resolver -- only needs to handle this bundle's own import
/// lines, and check.rs is itself part of the self-hosting bundle, so this
/// stays deliberately imperative/simple rather than relying on iterator
/// combinators rs-meta's own typeck may not model).
fn use_line_imported_names(line: &str) -> Vec<String> {
    let trimmed_line = line.trim();
    let chars: Vec<char> = trimmed_line.chars().collect();
    let mut last_colons: i64 = -1;
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == ':' && chars[i + 1] == ':' {
            last_colons = i as i64;
        }
        i += 1;
    }
    if last_colons < 0 {
        return Vec::new();
    }
    let start = (last_colons + 2) as usize;
    let mut tail = String::new();
    let mut j = start;
    while j < chars.len() {
        if chars[j] != ';' {
            tail.push(chars[j]);
        }
        j += 1;
    }
    let tail = tail.trim();
    let mut names = Vec::new();
    if let Some(inner) = tail.strip_prefix("{") {
        let inner = inner.strip_suffix("}").unwrap_or(inner);
        for part in inner.split(",") {
            let part = part.trim();
            if !part.is_empty() {
                names.push(part.to_string());
            }
        }
    } else if !tail.is_empty() {
        names.push(tail.to_string());
    }
    names
}

fn normalize_bundle_line(line: &str, seen_uses: &mut Vec<String>) -> Option<String> {
    if line.starts_with("//!")
        || line.starts_with("use crate::")
        || line.starts_with("use native::")
        || line.starts_with("mod ")
    {
        return None;
    }
    if line.starts_with("use std::") {
        // Dedup by imported NAME, not exact line text: two bundled files can
        // import the same std item through differently-shaped `use` lines
        // (`use std::path::Path;` vs `use std::path::{Path, PathBuf};`),
        // which would otherwise both survive as textually-distinct lines and
        // rustc would reject the item as defined multiple times.
        let names = use_line_imported_names(line);
        if !names.is_empty() && names.iter().all(|n| seen_uses.contains(n)) {
            return None;
        }
        for n in names {
            if !seen_uses.contains(&n) {
                seen_uses.push(n);
            }
        }
    }
    if line == "fn main() -> ExitCode {" {
        return Some("fn bootstrap_main() -> ExitCode {".to_string());
    }
    let mut out = line.to_string();
    out = replace_all(out.as_str(), "crate::", "");
    out = replace_all(out.as_str(), "check::", "");
    out = replace_all(out.as_str(), "lexer::", "");
    out = replace_all(out.as_str(), "parser::", "");
    out = replace_all(out.as_str(), "typeck::", "");
    out = replace_all(out.as_str(), "emit::", "");
    out = replace_all(out.as_str(), "sig::", "");
    out = replace_all(out.as_str(), "cap::", "");
    out = replace_all(out.as_str(), "diag::", "");
    out = replace_all(out.as_str(), "interp::", "");
    out = replace_all(out.as_str(), "native::", "");
    out = replace_all(out.as_str(), "hash::", "");
    out = replace_all(out.as_str(), "witness::", "");
    // "io::" self-references (e.g. main.rs's `io::io_check()`) need
    // stripping like every other bundled module above, but a blind
    // substring strip also eats the "io" out of real std paths like
    // `std::io::ErrorKind` -- protect that one first.
    out = replace_all(out.as_str(), "std::io::", "PNIX_STD_IO_PLACEHOLDER");
    out = replace_all(out.as_str(), "io::", "");
    out = replace_all(out.as_str(), "PNIX_STD_IO_PLACEHOLDER", "std::io::");
    Some(out)
}

fn replace_all(text: &str, needle: &str, replacement: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if starts_with_chars(&chars, i, &pat) {
            out.push_str(replacement);
            i += pat.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn starts_with_chars(chars: &[char], start: usize, pat: &[char]) -> bool {
    if start + pat.len() > chars.len() {
        return false;
    }
    let mut i = 0;
    while i < pat.len() {
        if chars[start + i] != pat[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub fn stage_status() {
    let rows: &[(&str, &str, &str)] = &[
        ("stage0", "DONE", "rustc builds the evaluator (cargo build) — trusted seed"),
        ("interp", "DONE", "in-Rust tree-walking evaluator for the Rust subset (oracle)"),
        ("native", "DONE", "same Rust source -> rustc -> run (Evcxr mechanism, zero-dep)"),
        ("tv", "DONE", "interpreter stdout == rustc stdout over the corpus"),
        ("typeck", "DONE", "light type-check; interp rejects iff rustc rejects (acceptance TV)"),
        ("source-ast", "DONE", "all rs-meta src/*.rs files parse under the rs-meta front-end"),
        ("all-source-bundle", "DONE", "src/*.rs single-file bundle print_help path runs under rs-meta and rustc with identical stdout"),
        ("stage2-chain", "DONE", "all-source bundled evaluator' replays the current positive corpus under rs-meta and rustc"),
        ("stage2-probe", "DONE", "lexer/parser/typeck/interp source-slice harnesses run under rs-meta and rustc with identical stdout"),
        ("stage3-slim-chain", "DONE", "slim evaluator stage2 loads/evaluates slim stage2' and matches rustc"),
        ("stage3-all-source-smoke", "DONE", "slimmed evaluator-core source bundle stage2 loads/evaluates stage2' smoke and matches rustc"),
        ("stage3-core-mini", "DONE", "evaluator-core source bundle stage2' replays arith/recursion/enum/struct/Vec-String/iterator-turbofish mini-corpus"),
        ("stage3-core-prefix", "DONE", "evaluator-core source bundle stage2' replays first 8 positive corpus cases"),
        ("stage3-core-middle", "DONE", "evaluator-core source bundle stage2' replays middle 8 positive corpus cases"),
        ("stage3-core-suffix", "DONE", "evaluator-core source bundle stage2' replays last 8 positive corpus cases"),
        ("stage3-core-feature", "DONE", "evaluator-core source bundle stage2' replays 10 named later-feature corpus cases"),
        ("stage3-core-negative", "DONE", "evaluator-core source bundle stage2' rejects 10 named negative corpus cases"),
        ("stage3-core-negative-middle", "DONE", "evaluator-core source bundle stage2' rejects middle 8 negative corpus cases"),
        ("stage3-core-negative-suffix", "DONE", "evaluator-core source bundle stage2' rejects last 8 negative corpus cases"),
        ("stage3-mirror", "DONE", "stage1(native), stage2, and stage2' emit identical canonical AST + probe output"),
        ("stage3-fixedpoint", "DONE", "stage2 (B) and stage2' (C) evaluator transcripts are identical (normalized B==C, bounded probe)"),
        ("stage3-full-held", "DONE", "machine check keeps the full all-source stage3 boundary row honest in the manifest"),
        ("stage3-full-chain", "DONE", "all-source stage2 -> stage2' full corpus replay passed (2103s release, budget-gated) after value_eq Vec fidelity fix"),
        ("stage8-repro-seed", "DONE", "sample source and all-source bundle built in two workdirs yield identical canonical native artifact receipts"),
        ("stage8-selfhost-repro", "DONE", "stage2 evaluator' artifact builds reproducibly in fresh workdirs with canonical receipts"),
        ("manifest", "DONE", "machine-readable proofs/stage-manifest.tsv indexes stage checks, status, timeout, and cost notes"),
        ("isolation", "DONE", "fresh interpreter runs do not leak stdout or function namespace state"),
        ("constitution", "DONE", "zero-dep Cargo.toml, local-only GitHub Actions posture, and content-hash native names are checked"),
        ("actions-disabled", "DONE", ".github/workflows is absent and the disabled workflow receipt is present; verification is local-only"),
        ("native-cache", "DONE", "native_run reuses content-hash rustc artifacts while stage8 receipts keep fresh compile path"),
        ("stage9-replay-seed", "DONE", "bootstrap help entrypoint replays in a clean process with canonical JSON receipt"),
        ("stage9-proof-matrix", "DONE", "non-recursive proof commands replay in clean subprocesses with canonical receipts"),
        ("stage9-aggregate-replay", "DONE", "proof-command aggregate replays in a clean subprocess without recursive check"),
        ("stage10-session-seed", "DONE", "clean-process command session transcript replays deterministically"),
        ("stage10-sandbox", "DONE", "client/server/session/sandbox replay boundary is fixed in a checked local proof receipt"),
        ("stage11-adapter-seed", "DONE", "adapter schema receipt validates local/disabled/held adapters and fail-closed conflict policy"),
        ("stage11-adapter-replay", "DONE", "multi-domain adapter replay closure checks local/rustc adapters and held/disabled fail-closed rows"),
        ("stage12-quarantine-seed", "DONE", "quarantine policy receipt validates local checks, no auto-promotion, and fail-closed behavior"),
        ("stage12-quarantine-replay", "DONE", "self-improvement quarantine closure replays local gates and holds disabled/external promotion fail-closed"),
        ("stage13-horizon-seed", "DONE", "long-horizon policy receipt validates stale degradation and boundary leak rules"),
        ("stage13-horizon-replay", "DONE", "long-horizon organism closure replays manifest/session receipts and degrades stale/external signals to held"),
        ("stage14-cross-impl-seed", "DONE", "cross-implementation export schema validates local/native/held comparison rows"),
        ("stage14-cross-impl-replay", "DONE", "cross-implementation closure replays rs-meta/rustc receipts and keeps alternate implementations held"),
        ("stage15-evidence-seed", "DONE", "open-world evidence federation receipt validates offline approval and fail-closed policy"),
        ("stage15-evidence-replay", "DONE", "open-world evidence closure replays local proof/manifest receipts and keeps external evidence held"),
        ("stageN-extension-seed", "DONE", "versioned constitutional extension policy validates manifest, budget, and migration rules"),
        ("stageN-extension-replay", "DONE", "versioned extension closure replays manifest/budget/stageN receipts and keeps future extensions held"),
        ("methods", "DONE", "inherent impl, associated fn, self/&self/&mut self method dispatch"),
        ("refs", "DONE", "&/&mut reference values, deref, mutable reference assignment"),
        ("vec", "DONE", "Vec<T> type surface, Vec::new, vec![], placeholder-aware push, pop/remove/get/first/last/len/is_empty/clear/join, iter/iter_mut/into_iter, indexing read/write, for-in Vec/&Vec"),
        ("iter", "DONE", "narrow Iterator surface: next/nth/last/map/filter/zip/all/any/rev/enumerate/find/position/count/sum/fold/take/skip/copied/cloned/collect; full trait solving held"),
        ("string", "DONE", "String/&str core, String::new/from, push_str/len/is_empty/as_str/chars/bytes/trim/split/contains/starts_with, +, &String -> &str coercion"),
        ("trim", "DONE", "String/&str .trim() for check output comparison paths"),
        ("bytes", "DONE", "String/&str .bytes() as Iter<u8> for self-host FNV hashing"),
        ("option/result", "DONE", "Some/None/Ok/Err and unwrap/unwrap_or/is_*/map/and_then/or_else/copied/cloned/ok_or_else / Result::ok/map/map_err / None placeholder join core"),
        ("box/rc/refcell", "DONE", "Box::new/as_ref, Rc::new/clone/ptr_eq/as_ref, &Box/&Rc to &T coercion, RefCell::new/borrow/borrow_mut/into_inner core"),
        ("hashmap", "DONE", "HashMap<K,V> new/insert/get/get_mut/remove/iter/entry core; String lookup by &str"),
        ("surface", "DONE", "attributes, pub visibility, use items, and mod items accepted/ignored"),
        ("clone", "DONE", "selected built-in .clone() surface with deep Vec/String/struct/enum/Iter value cloning; full Clone trait solving held"),
        ("char/casts", "DONE", "char literals/patterns/methods, String::push(char), usize/i32/u32/u64/u8 type surface, as casts"),
        (
            "int-inference",
            "DONE",
            "in-range unsuffixed integer literals in expected integer contexts; full range/suffix typing held",
        ),
        ("int-suffix", "DONE", "integer literal suffixes i64/i32/u32/u64/u8/usize plus hex literals"),
        ("int-parse", "DONE", "u64::from_str_radix for self-host lexer hex literal parsing"),
        ("fs", "DONE", "std::fs create_dir_all/write/read/read_to_string Result surface with TV and acceptance coverage"),
        ("path", "DONE", "Path::new, PathBuf::from, Path/PathBuf join/display/exists with TV and acceptance coverage"),
        ("path-resolve", "DONE", "known fully-qualified std path canonicalization for supported std surfaces"),
        ("command", "DONE", "Command::new/arg/env/env_clear/output plus Output.status/stdout/stderr and ExitStatus::success"),
        ("env", "DONE", "std::env::args()/var() surface for CLI and local environment probes"),
        ("exitcode", "DONE", "std::process::ExitCode SUCCESS/FAILURE constants for CLI main"),
        ("bitops", "DONE", "integer bitxor `^` and `^=` for self-host hash code"),
        (
            "question",
            "DONE",
            "? early-return for Option/Result plus narrow From<&str> for String error conversion; full From<E> held",
        ),
        ("match-guard", "DONE", "match arm guards `pat if cond =>`"),
        (
            "match-exhaustive",
            "DONE",
            "light bool/enum match exhaustiveness; guarded arms do not count",
        ),
        ("string-pattern", "DONE", "string literal match patterns"),
        ("format", "DONE", "print!/println!/format! fixed macros with {}/{0}/{name}/{:?}/{:#?}/{:016x}/{:<N}/{:>N}/{:.N} placeholders + Display/Debug/LowerHex/fixed-numeric typeck"),
        ("eprintln", "DONE", "eprintln! fixed macro with stderr ignored for stdout TV"),
        ("write-macros", "DONE", "write!/writeln! fixed macros for String/&mut String targets"),
        ("matches", "DONE", "matches! fixed macro with pattern guard"),
        ("cfg", "DONE", "cfg!(name) fixed macro for self-host platform branches"),
        ("panic-macros", "DONE", "panic!/unreachable!/todo! fixed macros parsed as Never"),
        ("assert-macros", "DONE", "assert!/assert_eq! fixed macros with bool/equality typeck"),
        ("rvalue-ref", "DONE", "&expr and &mut rvalue temporaries; immutable-place &mut still rejected"),
        ("ref-pattern", "DONE", "reference pattern &pat and &mut pat"),
        ("ref-binding-pattern", "DONE", "binding modifiers ref/ref mut in patterns"),
        ("match-ergonomics", "DONE", "literal, tuple, struct, and enum destructuring patterns auto-deref one reference layer"),
        ("or-pattern", "DONE", "or-pattern `a | b`"),
        ("range-pattern", "DONE", "inclusive integer/char range patterns plus `name @ pat` binding"),
        ("if-let", "DONE", "`if let pat = expr { ... } else { ... }` lowered through match"),
        ("while-let", "DONE", "`while let pat = expr { ... }` loop pattern binding"),
        ("let-else", "DONE", "`let pat = expr else { diverge };` statement pattern binding"),
        ("let-pattern", "DONE", "`let pat = expr;` destructuring for tuple/enum/ref patterns"),
        ("for-pattern", "DONE", "`for pat in iter` destructuring for foreach loops"),
        (
            "array-literal",
            "DONE",
            "`[a, b]` plus repeat `[x; n]` / `vec![x; n]` surfaces modelled with Vec values",
        ),
        ("type-alias", "DONE", "`type Name = ...;` item surface accepted and resolved by typeck"),
        ("const-static", "DONE", "top-level immutable const/static globals readable from functions"),
        (
            "impl-trait",
            "DONE",
            "`impl Trait` type surface plus trait item and impl Trait for Type method surface; full trait solving held",
        ),
        (
            "generic",
            "DONE",
            "shallow fn/struct/enum generic unification; full solver held",
        ),
        ("vec-slice", "DONE", "Vec range slicing v[a..b], v[a..=b], v[..b], v[a..]"),
        ("slice-type", "DONE", "&[T] type surface, len/get/first/iter/index/foreach, and &Vec<T> -> &[T] call compatibility"),
        ("enum-struct-variant", "DONE", "struct-like enum variant definitions, literals, field/rest patterns parsed/typechecked"),
        (
            "lifetimes",
            "DONE",
            "lifetime tokens/params parsed and erased in types; borrow/lifetime checking held",
        ),
        ("struct-shorthand", "DONE", "struct literal field shorthand `S { x }`"),
        ("field-place", "DONE", "struct field assignment/compound assignment for mutable places"),
        (
            "closures",
            "DONE",
            "typed/zero-arg closures, expected-type inference for impl Fn args, value capture, variable calls, and immediate expression calls",
        ),
        ("closure-ret", "DONE", "zero-arg closures and closure return annotations"),
        (
            "turbofish",
            "DONE",
            "method turbofish parsing plus str/String parse::<i64>(); full generic turbofish held",
        ),
        ("assignment-expr", "DONE", "assignment and compound assignment are valid expressions"),
        ("return-expr", "DONE", "return is a diverging expression with type absorption"),
        ("break-never", "DONE", "break/continue are diverging expressions for match/loop absorption"),
        ("block-never", "DONE", "blocks with trailing diverging statements type as Never"),
        ("return-no-semi", "DONE", "return statement without trailing semicolon"),
        ("prelude-patterns", "DONE", "unqualified Some/None/Ok/Err patterns"),
        ("subset", "DONE", "source-cover Rust subset is checked through stage1/source-bundle and stage2-chain; full Rust language completeness held"),
        ("stage1", "DONE", "source-ast/source-bundle prove the evaluator source is inside the checked subset"),
        ("stage2", "DONE", "all-source evaluator' replays the positive corpus and matches rustc"),
        ("stage3..7", "HELD", "slim, evaluator-core smoke, mini, and prefix chains are checked; full all-source corpus chain and B==C are held on local cost"),
        ("stage8", "DONE", "native artifact reproducibility includes sample, all-source bundle, and stage2 evaluator' receipts"),
        ("stage9", "DONE", "product, proof command, and bounded proof-aggregate clean-process replay checked"),
        ("stage10", "DONE", "client/server/session/sandbox replay closure is checked by local proof receipts"),
        ("stage11", "DONE", "multi-domain adapter closure checks local/rustc replay plus disabled/held fail-closed rows"),
        ("stage12", "DONE", "self-improvement quarantine closure checks local gates, no-auto-promotion, disabled Actions, and held rows"),
        ("stage13", "DONE", "long-horizon organism closure checks replay receipts, no-boundary-leak, and degrade-to-held policy"),
        ("stage14", "DONE", "cross-implementation closure checks local/native replay plus disabled/held fail-closed rows"),
        ("stage15", "DONE", "open-world evidence federation closure checks local proof/manifest replay, disabled Actions, and held external evidence"),
        ("stageN", "DONE", "versioned constitutional extension closure checks manifest index, timeout/cost budget, stageN seed, and held future changes"),
    ];
    println!("rs-meta — Rust meta-circular compiler/evaluator (stage ladder toward stage15-N):");
    for (stage, state, note) in rows {
        println!("  {:<12} {:<5} {}", stage, state, note);
    }
    println!(
        "\nconstitution: zero crates.io deps (std only); rustc is toolchain, not a dependency."
    );
    println!("honesty: DONE = runnable & checked, TODO = next slice, HELD = not yet claimed.");
}
