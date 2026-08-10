#!/usr/bin/env bash
# Every flake runner, doing its role. rs-meta = RUST engine (interp + rustc);
# pnix-rs = PNIX(px) engine + the interactive REPL driver (rs-meta stays pure).
set -u
cd "$(dirname "$0")"
PNIX_RS="${PNIX_RS:-pnix-rs}"
RS_META="${RS_META_BOOTSTRAP:-bootstrap}"
RUST='fn main() { let a = 6; let b = 7; println!("{}", a * b); }'

echo "== RUST engine (rs-meta): interpreter vs rustc, kept equal by TV =="
echo -n "  run        (interp) -> "; $RS_META run -c "$RUST"
echo -n "  native-run (rustc)  -> "; $RS_META native-run -c "$RUST"

echo
echo "== RUST REPL (repl-pnix-rs-rust): pnix-rs drives the rs-meta interpreter =="
printf 'fn twice(x: i64) -> i64 { x * 2 }\nlet base = 21;\ntwice(base)\n:quit\n' \
  | $PNIX_RS rust-repl 2>/dev/null | sed 's/^/  rust> => /'

echo
echo "== PNIX (px) compiler (pnix-rs-pnix): evaluate a .px file =="
echo -n "  -f default.px -> "; $PNIX_RS px-eval -f default.px

echo
echo "== PNIX (px) REPL (repl-pnix-rs-pnix): name = expr binds; else evaluates =="
printf 'x = 21\nx + x\ny = x * 2\n[ x y ]\n:quit\n' \
  | $PNIX_RS px-repl 2>/dev/null | sed 's/^/  px> => /'
