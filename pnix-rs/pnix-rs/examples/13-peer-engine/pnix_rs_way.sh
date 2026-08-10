#!/usr/bin/env bash
# The pnix-rs way: rs-meta as a PEER ENGINE on a common .px control plane.
#
# `.px` is the control plane; Rust source is this engine's domain payload.
# rs-meta stays pnix-ignorant (an independent Rust meta-circular engine); pnix-rs
# calls it only across the bootstrap CLI and maps its Rust translation-validation
# results into common `.px` envelopes.
set -euo pipefail
cd "$(dirname "$0")"
PNIX_RS="${PNIX_RS:-pnix-rs}"
RUST=limit_rust.rs

echo "== 0. engine attestation (why trust this engine's verdicts?) =="
$PNIX_RS engine-attestation

echo
echo "== 1. engine profile (this engine's capabilities as a .px value) =="
$PNIX_RS engine-profile

echo
echo "== 2. engine verdict for the Rust payload (rs-meta interp == rustc, .px) =="
$PNIX_RS engine-verdict -f "$RUST"

echo
echo "== 3. route on the verdict via the .px control plane =="
# The verdict IS a .px value; the control plane evaluates it (in px) to decide.
VERDICT="$($PNIX_RS engine-verdict -f "$RUST")"
# The routing DECISION is computed in px — the control plane, not the shell.
# (px uses nested if/then/else for conjunction.)
ACCEPT="$($PNIX_RS px-eval -c "let v = $VERDICT; in if v.status == \"accepted\" then (if v.verdict_kind == \"ok\" then v.tv_equal else false) else false")"
echo "control plane decision (computed in px): accept=$ACCEPT"
if [ "$ACCEPT" = "true" ]; then
  echo "=> ROUTE: accept (interp==rustc, well-typed) — safe to use the native artifact"
else
  echo "=> ROUTE: hold/reject"
fi

echo
echo "== 3b. a HELD payload: capability-aware routing (peer-engine value) =="
HELD=held_rust.rs
HV="$($PNIX_RS engine-verdict -f "$HELD")"
HSTATUS="$($PNIX_RS px-eval -c "let v = $HV; in v.status")"
HSURFACE="$($PNIX_RS px-eval -c "let v = $HV; in v.surface")"
echo "held payload (macro_rules!): status=$HSTATUS surface=$HSURFACE"
# The control plane decides IN px whether pnix-rs can serve this payload.
HELDSURF="$($PNIX_RS px-eval -c "let v = $HV; in v.status == \"held\"")"
if [ "$HELDSURF" = "true" ]; then
  echo "=> ROUTE: pnix-rs HOLDS this (surface=$HSURFACE) — control plane routes to another engine"
else
  echo "=> ROUTE: pnix-rs serves it ($HSTATUS)"
fi

echo
echo "== 4. native artifact receipt (build attestation as a .px value) =="
$PNIX_RS engine-artifact -f "$RUST"

echo
echo "== 5. batch orchestration (process a whole project at once) =="
# A .px list of Rust sources (quote-free here for shell clarity): one accepted,
# one held (macro_rules). The manifest carries every verdict + counts.
BATCH="$($PNIX_RS engine-batch -c '[ "fn main() { let _ = 6 * 7; }" "macro_rules! m { () => {}; } fn main() {}" ]')"
echo "$BATCH" | tr ';' '\n' | grep -E "total|accepted|held|rejected" | sed 's/^ */  /'

echo
echo "== 6. verify a verdict (tamper-evident — trust-free) =="
VER="$($PNIX_RS engine-verdict -f "$RUST")"
$PNIX_RS engine-verify -c "$VER"
# tamper it and show verification fails
TAMPERED="$(echo "$VER" | sed 's/status = \"accepted\"/status = \"rejected\"/')"
echo -n "tampered -> "; $PNIX_RS engine-verify -c "$TAMPERED" || true

echo
echo "== 7. the gates proving this adapter =="
for g in engine-verdict-check engine-artifact-check engine-request-check \
         engine-attestation-check engine-verify-check engine-batch-check; do
  printf '  %-26s ' "$g"; $PNIX_RS "$g" | tail -1
done
