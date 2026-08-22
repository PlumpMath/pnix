#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
binary="${PNIX_RS:-$repo/target/release/pnix-rs}"
local_meta="$repo/../rs-meta/target/release/bootstrap"

if [[ ! -x "$binary" ]]; then
  cargo build --release --manifest-path "$repo/Cargo.toml" --bin pnix-rs
fi
if [[ -z "${RS_META_BOOTSTRAP:-}" && -x "$local_meta" ]]; then
  export RS_META_BOOTSTRAP="$local_meta"
fi

assert_output() {
  local file="$1"
  local expected="$2"
  local output
  output="$($binary px-eval -f "$here/$file")"
  [[ "$output" == *"$expected"* ]] || {
    echo "unexpected $file result: $output" >&2
    exit 1
  }
}

assert_output direct.px 'value = 42'
assert_output consumer.px 'answer = 42'
assert_output consumer.px 'mapped = [ 2 4 6 ]'
assert_output self_interpreter.px 'mode = "pnix-in-pnix"; value = 42'

# Live rs-meta substrate composition and the established PNIX tower check.
"$binary" substrate-check >/dev/null
"$binary" tower-check >/dev/null
cargo test --quiet --release --manifest-path "$repo/Cargo.toml" --lib \
  tests::calls_exported_library_functions_with_json_data

echo "PASS pnix-rs production-readiness"
