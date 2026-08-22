#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
pnix="$repo/bin/pnix-clr"
meta="$repo/bin/clr-meta"

done_value() {
  "$pnix" "$here/$1" | jq -e 'select(.outcome_kind == "done") | .value'
}

direct="$(done_value direct.px)"
consumer="$(done_value consumer.px)"
self_hosted="$(done_value self_interpreter.px)"

jq -e '.mode == "direct-runtime" and .value == 42' <<<"$direct" >/dev/null
jq -e '.answer == 42 and .mapped == [2,4,6] and .count == 4 and .total == 10' \
  <<<"$consumer" >/dev/null
jq -e '.mode == "pnix-in-pnix" and .value == 42' <<<"$self_hosted" >/dev/null

called="$($pnix --call-json "$here/library.px" double '[21]')"
jq -e '.outcome_kind == "done" and .value == 42' <<<"$called" >/dev/null
called_map="$($pnix --call-json "$here/library.px" mapDouble '[[1,2,3]]')"
jq -e '.outcome_kind == "done" and .value == [2,4,6]' <<<"$called_map" >/dev/null

# Exercise the CLR host-meta evaluator and require persisted compiler closure.
meta_output="$($meta -e '(+ 20 22)')"
[[ "$meta_output" == *':value 42}'* ]] || {
  echo "unexpected clr-meta result: $meta_output" >&2
  exit 1
}
jq -e '.ready == true and .claims.fixed_point == true' \
  "$repo/clr-meta/work/compiler-self-reproduction-check.receipt.json" >/dev/null
jq -e '.ready == true and .claims.stage15 == true' \
  "$repo/clr-meta/work/compiler-selfhost-stage15-gate.receipt.json" >/dev/null

echo "PASS pnix-clr production-readiness"
