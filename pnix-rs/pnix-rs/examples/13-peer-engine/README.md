# 13 — peer engine (rs-meta as a Rust-domain engine on a common .px plane)

**Why pnix-rs uses rs-meta and not "just a Rust compiler":** rs-meta is a
*meta-circular* Rust compiler/evaluator — an in-Rust interpreter kept equal to
the `rustc` native tier by translation validation (interp stdout == rustc
stdout). pnix-rs treats it as a **peer engine** and exposes its Rust
translation-validation results on a common `.px` control plane, the same shape a
pnix-hy / pnix-clj peer engine would emit.

- `limit_rust.rs` — the limit of plain rustc: you get a binary, but no
  translation-validated / content-addressed / routable verdict.
- `pnix_rs_way.sh` — the full control-plane toolkit end-to-end:
  **attestation** (why trust) → **profile** (capabilities) → **verdict**
  (interp==rustc, .px) → **routing** (accept vs held → another engine) →
  **artifact** (build receipt) → **batch** (whole project, counts) → **verify**
  (tamper-evident, trust-free).

## The separation (constitution)
- **rs-meta** knows nothing about pnix — an independent Rust meta-circular
  engine. pnix-rs calls it only across the bootstrap CLI (a process boundary).
- **`.px` is the control plane**; Rust source is this engine's domain payload.
- The verdict/profile/artifact are real `.px` values (attribute sets), so the
  control plane evaluates/hashes/routes them with ordinary px machinery.

## Envelopes
- `pnix.engine.profile.v0` — supports / does_not_support (honest held frontier:
  full-borrowck, macro-rules, full-trait-solver).
- `pnix.engine.verdict.v0` — status (accepted|held|rejected) + verdict_kind
  (ok | negative-boundary-agrees | divergent | incomplete-subset | held-*) +
  source_hash + ir_hash (format-invariant canonical Rust IR) + interp/native
  output hashes + tv_equal + witness_id.
- `pnix.engine.artifact.v0` — rust-native build receipt (rustc version, artifact
  hash, receipt hash).

## Control-plane toolkit
- `engine-request` — `.px` request envelope (pnix.engine.request.v0) dispatched by phase.
- `engine-verdict` — status/verdict_kind/reason_code(rustc E-code)/surface
  (held-*)/ir_hash/interp+native hash/tv_equal/witness_id.
- `engine-artifact` — reproducible native build receipt.
- `engine-profile` / `engine-attestation` — capabilities + trust signal
  (interp==rustc TV coverage: 310 positive + 257 negative; substrate 3-way).
- `engine-verify` — recompute the witness_id from the verdict's own fields to
  detect tampering (proof-carrying verdict; a control plane verifies untrusted
  engines instead of trusting them).
- `engine-batch` — process a `.px` list of sources into a verdict manifest.

## Gates
`engine-verdict-check` · `engine-artifact-check` · `engine-request-check` ·
`engine-attestation-check` · `engine-verify-check` · `engine-batch-check`.
See proposals 0008 (peer-engine adapter) and 0009 (canonical Rust IR).
