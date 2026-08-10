// Substrate harness: appended after src/px.rs when ../rs-meta's bootstrap runs
//   bootstrap run        -f src/px.rs -f harness/substrate_harness.rs
//   bootstrap native-run -f src/px.rs -f harness/substrate_harness.rs
// The .px probes are embedded (not read from disk) so the transcript is
// deterministic across working directories. Keep the probes byte-identical to
// the runtime/corpus/*.px files, in px_corpus() order (seed_* excluded).
fn main() {
    let c02 = "let name = \"pnix\"; n = 7; in { hello = \"hello, ${name}!\"; len = builtins.stringLength name; joined = builtins.concatStringsSep \"-\" [ \"a\" \"b\" \"c\" name ]; sub = builtins.substring 0 3 \"abcdefgh\"; interp = \"n=${builtins.toString n} sq=${builtins.toString (n * n)}\"; }";
    let c03 = "let range = n: if n == 0 then [] else (range (n - 1)) ++ [ n ]; xs = range 20; in { len = builtins.length xs; mapped = builtins.map (x: x * x) xs; filtered = builtins.filter (x: x > 10) xs; total = builtins.foldl' (a: x: a + x) 0 xs; }";
    let c04 = "let base = { a = 1; b = 2; }; merged = base // { c = 3; b = 20; }; in { m = merged; pick = merged.c; names = builtins.attrNames merged; has = builtins.hasAttr \"a\" merged; }";
    let c05 = "let go = acc: n: if n == 0 then acc else go (acc + n) (n - 1); fib = n: if n < 2 then n else fib (n - 1) + fib (n - 2); in { sum = go 0 500; fib = fib 20; }";
    let c07 = "let xs = [ 5 2 8 1 9 3 ]; in { sorted = builtins.sort (a: b: a < b) xs; head = builtins.head xs; tail = builtins.tail xs; at = builtins.elemAt xs 2; member = builtins.elem 8 xs; }";
    let c08 = "let classify = n: if n < 0 then \"neg\" else if n == 0 then \"zero\" else if n < 10 then \"small\" else \"big\"; in builtins.map classify [ (0 - 5) 0 7 42 ]";
    let c09 = "let compose = f: g: x: f (g x); inc = x: x + 1; dbl = x: x * 2; apply3 = f: x: f (f (f x)); in { c = compose inc dbl 10; a = apply3 inc 0; curry = (a: b: c: a + b + c) 1 2 3; }";
    let c06 = "let mk = i: { id = i; sq = i * i; tags = [ \"t${builtins.toString i}\" \"x\" ]; }; range = n: if n == 0 then [] else (range (n - 1)) ++ [ (mk n) ]; in builtins.toJSON (range 12)";
    let c10 = "let range = n: if n == 0 then [] else (range (n - 1)) ++ [ n ]; xs = range 30; evens = builtins.filter (x: x - (x / 2) * 2 == 0) xs; rec = builtins.foldl' (acc: x: acc // { \"k${builtins.toString x}\" = x * x; }) {} evens; in { count = builtins.length evens; squares = rec; total = builtins.foldl' (a: x: a + x) 0 evens; label = \"evens-${builtins.toString (builtins.length evens)}\"; }";
    let c01 = "let a = 21; b = 2; f = 3.5; in { sum = a + b; prod = a * b; mixed = a * 2 + b; flt = f * 2.0; div = a / b; modv = a - (b * 10); }";
    match px_run(c02) {
        Ok(out) => println!("c02_strings {}", out),
        Err(e) => println!("c02_strings ERR {}", e),
    }
    match px_run(c03) {
        Ok(out) => println!("c03_list {}", out),
        Err(e) => println!("c03_list ERR {}", e),
    }
    match px_run(c04) {
        Ok(out) => println!("c04_attr {}", out),
        Err(e) => println!("c04_attr ERR {}", e),
    }
    match px_run(c05) {
        Ok(out) => println!("c05_recurse {}", out),
        Err(e) => println!("c05_recurse ERR {}", e),
    }
    match px_run(c07) {
        Ok(out) => println!("c07_builtins {}", out),
        Err(e) => println!("c07_builtins ERR {}", e),
    }
    match px_run(c08) {
        Ok(out) => println!("c08_bool {}", out),
        Err(e) => println!("c08_bool ERR {}", e),
    }
    match px_run(c09) {
        Ok(out) => println!("c09_lambda {}", out),
        Err(e) => println!("c09_lambda ERR {}", e),
    }
    match px_run(c01) {
        Ok(out) => println!("c01_arith {}", out),
        Err(e) => println!("c01_arith ERR {}", e),
    }
    match px_run(c06) {
        Ok(out) => println!("c06_nested {}", out),
        Err(e) => println!("c06_nested ERR {}", e),
    }
    match px_run(c10) {
        Ok(out) => println!("c10_mixed {}", out),
        Err(e) => println!("c10_mixed ERR {}", e),
    }
    let phase2 = "let x = 100000000000000000000.0; y = x * x; z = y * y; w = z * z; i = w * w; n = i - i; f = v: v; in { hashes = map (a: builtins.hashString a \"abc\") [ \"md5\" \"sha1\" \"sha256\" \"sha512\" ]; mixed = [ (1 + 1.5) (builtins.add 1 1.5) (builtins.lessThan 1 1.5) (1 == 1.0) ([ 1 ] == [ 1.0 ]) (builtins.elem f [ f ]) ]; strings = [ (builtins.toString 1.5) (builtins.toString 1.25e-3) (builtins.toString .5e2) (builtins.toString 0.0e-400) (builtins.toString (-0.0)) (builtins.toString (0.0 / (-1.0))) (builtins.toString ((-1.0) * 0.0)) (builtins.toString i) (builtins.toString (0.0 - i)) (builtins.toString n) ]; compares = [ (n < n) (n <= n) (n > n) (n >= n) ([ n 0 ] < [ n 1 ]) ]; identity = [ (let l = [ (builtins.throw \"forced\") ]; in (builtins.tryEval (l == l)).success) (let g = h: [ h ]; in (g f) == (g f)) ]; round = [ (builtins.ceil (-1.8)) (builtins.floor (-1.2)) ]; }";
    match px_run(phase2) {
        Ok(out) => println!("phase2_numeric_hash {}", out),
        Err(e) => println!("phase2_numeric_hash ERR {}", e),
    }
    match px_run("9223372036854775807 + 1") {
        Ok(out) => println!("phase2_overflow UNEXPECTED {}", out),
        Err(e) => println!("phase2_overflow {}", e),
    }
    match px_run("builtins.hashString \"sha3\" (builtins.throw \"payload\")") {
        Ok(out) => println!("phase2_hash_order UNEXPECTED {}", out),
        Err(e) => println!("phase2_hash_order {}", e),
    }
    let hash_edges = "let p56 = builtins.concatStringsSep \"\" (builtins.genList (_: \"a\") 56); p112 = builtins.concatStringsSep \"\" (builtins.genList (_: \"a\") 112); raw = builtins.substring 0 1 \"가\"; in { boundary = [ (builtins.hashString \"md5\" p56) (builtins.hashString \"sha1\" p56) (builtins.hashString \"sha256\" p56) (builtins.hashString \"sha512\" p112) ]; raw = builtins.hashString \"sha256\" raw; unicode = builtins.hashString \"sha256\" \"가🙂\"; }";
    match px_run(hash_edges) {
        Ok(out) => println!("phase2_hash_edges {}", out),
        Err(e) => println!("phase2_hash_edges ERR {}", e),
    }
    match px_run("1.0e-308") {
        Ok(out) => println!("phase2_float_literal UNEXPECTED {}", out),
        Err(e) => println!("phase2_float_literal {}", e),
    }
    let uri_literals = "[ x:x let:x a:b==c a:%/?::@&=+$,-_.!~*' (builtins.typeOf (x: x)) (builtins.typeOf (_x:_x)) (a:b + \"c\") ]";
    match px_run(uri_literals) {
        Ok(out) => println!("phase3_uri_literals {}", out),
        Err(e) => println!("phase3_uri_literals ERR {}", e),
    }
    let posix_classes = "let m = p: s: builtins.match p s != null; in [ (m \"[[:alnum:]]+\" \"Az09\") (m \"[[:blank:]]+\" \" \\t\") (m \"[[:cntrl:]]+\" \"\\t\\n\") (m \"[[:graph:]]+\" \"Az!9\") (m \"[[:print:]]+\" \" Az!9\") (m \"[[:punct:]]+\" \"!?\") (m \"[[:space:]]+\" \" \\t\\n\") (m \"[[:xdigit:]]+\" \"aF09\") (builtins.match \"[[:space:]]*(.*[^[:space:]])[[:space:]]*\" \" ?x \" == [ \"?x\" ]) (builtins.split \"[[:space:]]+\" \"a \\tb\\nc\" == [ \"a\" [ ] \"b\" [ ] \"c\" ]) ]";
    match px_run(posix_classes) {
        Ok(out) => println!("phase3_posix_classes {}", out),
        Err(e) => println!("phase3_posix_classes ERR {}", e),
    }
    let mirror_ast = px_parse(c05).unwrap();
    let emitted = px_emit(&mirror_ast);
    let reparsed = px_parse(&emitted).unwrap();
    let env = Vec::new();
    let v = px_eval(&reparsed, &env).unwrap();
    println!("mirror_c05 {}", px_print(&v));
}
