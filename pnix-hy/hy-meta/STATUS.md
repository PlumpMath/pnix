# hy-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**hy-meta** is the Hy/Python host-meta bootstrap for `pnix-hy`. It owns the
deepest explicit stage ladder among host metas (stage1 → stage7 compiler chain,
self-host kernel fixed point, stages 8–15/N product/organism seeds).

| Peer | Peer floor | hy-meta counterpart |
|---|---|---|
| clj-meta | stage7 stock + bytecode selfhost | stage7-check + bootstrap-fixedpoint-check |
| rs-meta | TV + stage chain toward 15-N | stage ladder + parity ledger (kernel vs native) |
| cljs-meta | fixed-point compiler | bootstrap-fixedpoint-check (B==C kernel artifacts) |
| clr-meta | eval gen0–2 + C0–C3 | stage chain + evaluator/kernel path |

**Honest classification:** self-hosting **back-end** (direct kernel → Python AST),
not full meta-circular ownership of the reader. `hy.reader` and name mangling
remain delegated host substrate. Full upstream `hy.compiler` parity is a
post-stage7 track, not a closed claim.

Python proof targets: **3.11** and Homebrew **3.14** only (3.12/3.13 rejected).

## Closed claims

Live-verified this session (2026-08-07) via `./hy-meta/bin/hy-meta-gate primary`:

```text
self-check                         PASS (stage1=6, stage2=42, stage2_self_check=True)
stage7-check                       PASS
  stage_count=7, all_stage_self_checks=True
  compiler/kernel AST+Python+value stage7 mirrors=True
  isolation (modules/macros/globals) ok
  kernel_factorial=120, kernel_loop=120, kernel_features=449.0
```

Documented closed by bootstrap commands (not re-run this session):

```text
chain-check / kernel-check / prime-check / stage3-check / mirror-check
self-host-check / bootstrap-fixedpoint-check / no-fallback-check
parity-ledger-check / stage8..stage15 / stagen seeds
reader-boundary-check / kernel-import-check / native-subset-test
```

## Open claims (do not claim)

```text
full_reader_ownership = false
complete_upstream_hy_compiler_parity = false
full_REPL/hyc/hy2py/zipimport_product_surface = false
Python_3.12_or_3.13_support = false
trusting-trust_defense = false
pnix_language_semantics_ownership = false
```

Stage15/N checks are **local product/organism seeds**, not Hy/CPython replacement.

## Primary gate

```sh
# From pnix-hy/
./hy-meta/bin/hy-meta-gate              # self-check + stage7-check
./hy-meta/bin/hy-meta-gate self-check
./hy-meta/bin/hy-meta-gate full         # + self-host + fixedpoint subset
```

Env used this session:

```sh
/usr/local/bin/python3.11 -m venv /tmp/pnix-hy-py311-venv
/tmp/pnix-hy-py311-venv/bin/python -m pip install 'funcparserlib ~= 1.0'
export HY_META_PYTHON=/tmp/pnix-hy-py311-venv/bin/python
# HY_META_PYTHON has hy installed (pip/nix); no vendored PYTHONPATH needed
```

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `hy-meta-gate primary` | **PASS** | Python 3.11.15 + hy 1.3.0 + funcparserlib |
| full ladder stage8–stagen | not default-run | available via bootstrap.py |
