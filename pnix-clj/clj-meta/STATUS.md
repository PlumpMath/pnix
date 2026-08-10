# clj-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**clj-meta** is the JVM/Clojure host-meta substrate for `pnix-clj`. Practical
peer floor relative to other metas:

| Peer | Peer floor | clj-meta counterpart |
|---|---|---|
| hy-meta | stage ladder / fixed-point checks | `:gate` bytecode selfhost + stock stage7 rebuild |
| rs-meta | TV + multi-stage selfhost | compiler conformance + self-emit fixed point |
| cljs-meta | fixed-point compiler (stage2==stage3) | backend self-emit determinism / stage1→7 selfhost chain |
| clr-meta | eval gen0–2 + C0–C3 Stage1/2 | kernel + full-eval tower + compiler lane |

Two **separate** lanes (do not conflate):

1. **Bytecode meta compiler** (`src/pnix/clj_meta/{compiler,selfhost,gate}.clj`) —
   analyzer/ASM emit + deterministic self-host checks. Primary product floor.
2. **Stock stage7 rebuild** (`stage7-gate.sh`) — hosted deterministic rebuild of
   Clojure 1.12.5 via Maven/Ant. Reproducible-build evidence, not the
   meta-circular compiler proof.

Neither lane claims JVM-free Clojure self-hosting. Reader, `clojure.core`, and
the JVM remain permanent substrate.

## Closed claims

Live-verified this session (2026-08-07):

```text
./bin/clj-meta-gate selfhost              PASS  ready=true
./bin/clj-meta-gate stage7                PASS  stage7-check OK (Maven 3.9.12)
./bin/clj-meta-gate primary (:gate)       READY ✅
  stage11 multisurface                    OK
  stage12 quarantine                      OK
  stage13 long-horizon                    OK
  stage14 crosshost                       OK  (missing external transcripts held)
  stage15 openworld                       OK
  stageN recursive                        OK
  full-source stage1                      OK  (M12 fallback-free accepted)
  lowering admission                      OK
  diverse double compile                  OK
  reproducible DDC lane                   OK
  (plus prior OK witnesses: bytecode/TV/verifier/language-surface/…)
```

### Fixes that closed stage11–N (this wave)

1. **stage9/10 child classpath** — children invoked `clojure -M:audit-self-source`
   from pnix-clj root (no root `deps.edn`). Now use `-Sdeps` with absolute
   `clj-meta/src` (same shape as primary gate).
2. **stage10 sandbox cwd** — `source-path` now resolves under `CLJ_META_ROOT`
   (`clj-meta/src/...` preferred), so sandbox relocation finds compiler.clj.
3. **lowering-admission** — m6aj `checked-fallback` accepted rows
   (`promotion/allowed?=false`) map to held boundary, not raw-bytecode admission.
4. **stage14** — missing external transcripts and synthetic drift sentinel use
   `:held` (aligned with docstring + invariants), not `:unavailable`/`:rejected`.

Documented closed by design:

```text
boundary policy: direct emit uses host Compiler 0 times; fallback explicit
M12 fallback-free genuine stage1 boundary: ACCEPTED (host-Compiler-fallback-forms=0)
```

## Open / not claimed

```text
full-language-correctness                 false
trusting-trust / Wheeler independent DDC  false
JVM-free self-hosting                     false
external stage14 transcripts (hy/pnix-hy/pnix-clj files)  optional held evidence
```

Stage11–N are **local product/organism closure seeds** with honest held
boundaries (missing cross-host transcripts, checked-fallback lowering), not
Clojure language-runtime replacement.

## Primary gate

```sh
# From pnix-clj/clj-meta/
./bin/clj-meta-gate              # full :gate integrated receipt
./bin/clj-meta-gate selfhost     # practical peer floor (bytecode selfhost)
./bin/clj-meta-gate stage7       # stock rebuild (needs mvn on PATH)
```

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `./bin/clj-meta-gate selfhost` | **PASS** | ready=true |
| `./bin/clj-meta-gate stage7` | **PASS** | Maven 3.9.12 |
| `./bin/clj-meta-gate primary` | **READY ✅** | stage11–N + DDC + full-source closed |
| env | JDK 21, Clojure 1.12.5 CLI, Maven 3.9.12 | OK |
