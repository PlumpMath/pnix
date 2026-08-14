# Host env P2 / P3 + next product tracks (planning)

**Status:** dual-axis + library import **closed enough** (2026-08-14).  
**Doctrine / Day-1:** [HOST_DEV_ENV.md](HOST_DEV_ENV.md) · [HOST_IMPORT.md](HOST_IMPORT.md)  
**Local regression:** `./bin/host-env-residual-smoke` (or PATH `./bin/host-import-smoke` after HM)

This file is the **owner-facing plan** for optional follow-ups. Hard items stay
planned until explicitly pulled. Easy items may land as small examples/CI.

---

## P2 — quality / convenience (host-import track)

### P2.1 Periodic smoke (ops)

| Field | Content |
|-------|---------|
| Goal | After every `dot-nix` rebuild that bumps pnix inputs, re-run import smoke |
| How | `~/pnix/bin/host-import-smoke` |
| Done when | Documented habit + optional CI (P2.3) |
| Effort | trivial |
| **State** | **landed** as script + docs; habit is operator-side |

### P2.2 Real mini projects (host-main demos)

| Host | Skeleton | Done when |
|------|----------|-----------|
| clj | `examples/host-import/clj/` + `deps.edn` local/root | `clojure -M -m smoke` prints 3 |
| cljs | `examples/host-import/cljs/smoke.mjs` | `node smoke.mjs` prints 3 (needs NODE_PATH / HM) |
| hy | `examples/host-import/hy/smoke.py` | `python smoke.py` prints 3 |
| rs | `examples/host-import/rs/README.md` path-dep or link flags | documented; optional tiny crate later |
| clr | point at `pnix-clr/csharp/examples/HelloPnix` + props sample | already exists |

| Field | Content |
|-------|---------|
| Effort | small |
| **State** | **landed** skeletons + **rs path-dep crate** `examples/host-import/rs/pnix-rs-smoke` |

### P2.3 CI for import regression

| Field | Content |
|-------|---------|
| Goal | PR cannot delete smoke/examples without notice |
| Phase A (easy) | layout + `bash -n` + **clj example** + **rs cargo path-dep** |
| Phase B (medium) | per-host flake jobs that print `pnix-*-library` / `*-refs` env |
| Phase C (hard) | full 5-host eval like local smoke (needs multi-toolchain matrix; do **not** start until A/B are green for weeks) |
| **State** | **Phase A+B landed** in `.github/workflows/hosts.yml` (B = clj/hy/rs; cljs/clr covered by host gates). ShellCheck SC2012 fixed for library printers. |

### P2.4 Public API polish (docs only unless churn)

| Host | Work | Priority | State |
|------|------|----------|-------|
| clj | Keep `docs/HOST_IMPORT.md` as source of truth; expand only when API grows | low | docs landed |
| hy | Freeze `__all__`; optional py.typed | low | **py.typed landed** (PEP 561 marker) |
| cljs | Scoped require verified; local export landed; npm publish **dropped** | — | **export+smoke** |
| rs | CARGO_HOST_IMPORT.md done; C ABI semver policy note in header | low | **ABI comment + version pin note landed**; full semver process still P3 |
| clr | pack local nupkg done; nuget.org **dropped** (local feed only) | — | — |

### P2.5 Dedicated host-import workflow on push

| Field | Content |
|-------|---------|
| Goal | Main-branch pushes that touch host-env files get import CI without full gate matrix |
| How | `.github/workflows/host-import.yml` (path-filtered push + PR) |
| **State** | **landed** — layout/examples + library-print (clj/hy/rs); hy example via flake package |

---

## P3 — distribution / large product (plan only)

Do **not** start without an explicit product decision.

### P3.1 Registry publish

| Registry | Package | Blockers |
|----------|---------|----------|
| Maven | `pnix-clj` jar | versioning, source jar, which namespaces public |
| npm | `@plumpmath/pnix-cljs` | already scoped name in store; need CI publish secrets |
| crates.io | `pnix-rs` | currently `publish = false`; zero-deps story must stay |
| nuget.org | `Pnix.Clr` | **won't do (owner 2026-08-14)** — personal/local feed only; pack+smoke stay |

### P3.2 Full ClojureCLR project story

| Goal | Drop-in `deps.edn` / project.clj style CLR host beyond `-e` / single file |
| Acceptance | Documented REPL + multi-file load + Reference assemblies without pnix-only CLI |
| Depends on | clr-meta substrate stability, not just pnix-clr guest AOT |
| **Plan detail** | see `pnix-clr/clr-meta/todo.md` § “P3 full ClojureCLR project” |
| **Step 1 inventory** | **landed** — `pnix-clr/docs/CLOJURE_CLR_ADMITTED_SURFACE.md` |
| **Step 2 TFM** | **landed** — `pnix-clr/docs/TFM_POLICY.md` |
| **Step 3 template+smoke** | **landed** — `pnix-clr/examples/clojure-clr-project/` (bootstrap multi-ns → 42) |
| **Profiles smoke** | **landed** — `bin/clojure-clr-profiles-smoke` (also in `pnix-clr-gate`) |
| **tool-eval-multi** | **landed** — `--multi-form FILE\|-`, `--multi-e FORM` + named gate |
| **Local nupkg smoke** | **landed** — `bin/pnix-clr-nupkg-smoke` (local feed only) |
| **nuget.org publish** | **dropped** — owner uses local feed only; no nuget.org track |
| **In-process C# eval** | **experimental** — net10 + parity gate; aggregate when substrate present |
| **clj local export** | **landed** — `export-pnix-clj-library` + library-smoke (local only) |
| **rs local export** | **landed** — `export-pnix-rs-library` + library-smoke (local only) |
| **hy local export** | **landed** — `export-pnix-hy-library` + library-smoke (local only) |
| **cljs local export** | **landed** — `export-pnix-cljs-library` + library-smoke (local only) |
| **tool-eval stdin** | **landed** — single-form `-` + multi-form `--multi-form -` |
| **clr host-import smoke** | **landed** — `examples/host-import/clr/smoke` (HelloPnix) |
| **clr library smoke** | **landed** — `pnix-clr-library-smoke` (export API + nupkg + HelloPnix) |
| **host-env residual cut** | **closed enough (2026-08-14)** — local feeds, examples, tool-eval surfaces, CI layout |
| **tool-eval surface gate** | **landed** — `clr-meta-tool-surface-gate` freezes admitted CLI |
| **Still open (product pillars, not host-env)** | machine/F-series if pillar; in-process ALC (blocked); new tool-eval only with named gate |

### P3.3 Common portable `.px` library (historical pnix-meta)

| Goal | One portable library corpus loadable by all five hosts with equal semantics |
| Status | **deferred** — do not block host-local import |
| Acceptance | packaging contract + five-host gate slice + no host-leak builtins |
| **Plan** | reopen only after host gates admit a shared corpus again |

### P3.4 ABI / typing contracts

| Item | Host | Plan |
|------|------|------|
| C ABI semver for `pnix_rs.h` | rs | version macro + changelog; bump on any struct/export change |
| `py.typed` + stub | hy | empty py.typed + export only `__all__` |
| MSBuild multi-TFM NuGet | clr | net8+net10 already multi-target managed DLL |

---

## Adjacent product tracks (plan only — not host-env)

These were listed as “next after host-env”. **No implementation in this cut.**

### A. clj residual / product residual

| Source of truth | `pnix-clj/pnix-clj/todo.md` § REMAINING WORK + `docs/REMAINING_DECISION.md` |
| Rule | Do not invent a new residual menu; pillar-driven (M-series) or oracle divergence only |
| Next candidates (owner picks) | machine fragment growth (if pillar); local clj export **landed**; Phase D **deferred** |
| Host-import interaction | none required — `eval-file` / classpath inject already green |
| **Detail** | `pnix-clj/pnix-clj/todo.md` § “Post host-env plan (2026-08-14)” |

### B. clr-meta residual

| Source of truth | `pnix-clr/clr-meta/todo.md` + `STATUS.md` + stage design docs |
| Rule | meta first; no Stage15/N promotion claims without receipts |
| Next candidates | widen admitted eval surface carefully; compiler stage ladder honesty; full CLR project story (P3.2) |
| Host-import interaction | export library already depends on artifact builder |
| **Detail** | `pnix-clr/clr-meta/todo.md` § “Post host-env plan (2026-08-14)” |

### C. Other hosts residual

| hy | `pnix-hy/pnix-hy/todo.md` — gate green; projection polish only |
| rs | `pnix-rs/pnix-rs/todo.md` — substrate-check / stage ladder in rs-meta |
| cljs | corpus admission still open per monorepo README |

---

## Decision log

| Date | Decision |
|------|----------|
| 2026-08-14 | Host dual-axis + library import declared **closed** for day-to-day dev env |
| 2026-08-14 | P2.2 skeletons + P2.3 Phase A CI **started**; P3 registry/full CLR/common-.px **plan only** |
| 2026-08-14 | clj residual / clr-meta / full examples / heavy CI = **todo detail only** until owner pull |
| 2026-08-14 | P2.2 rs mini crate + P2.3 Phase B library-print matrix **started** |
| 2026-08-14 | P2.4 py.typed + rs ABI header note; P2.5 `host-import.yml` on push |
| 2026-08-14 | P3.2 step1 clojure-clr inventory; clj multi-module import example |
| 2026-08-14 | P3.2 step2 TFM + step3 bootstrap multi-ns project smoke |
| 2026-08-14 | P3.2 named profiles + clojure-clr-profiles-smoke (4/4) |
| 2026-08-14 | tool-eval-multi --multi-form + clr-meta-tool-eval-multi-gate |
| 2026-08-14 | profiles-smoke wired into pnix-clr-gate (~17s) |
| 2026-08-14 | local nupkg pack smoke (`pnix-clr-nupkg-smoke`); nuget.org still owner-gated |
| 2026-08-14 | M1 per-call `:fold-fuel`; nuget publish fail-closed; in-process eval design |
| 2026-08-14 | in-process eval spike (net10 ALC) + parity gate (opt-in, not aggregate) |
| 2026-08-14 | in-process corpus 17-pass; isolated ALC held (CLR Default load); host-artifact API |
| 2026-08-14 | host-artifact report rows; nupkg-smoke in gate if export; INPROCESS opt-in gate |
| 2026-08-14 | nuget.org publish **dropped** (local-only); inprocess gate auto when substrate+artifact |
| 2026-08-14 | clj local library export + smoke; inprocess reentrancy = serialized lock |
| 2026-08-14 | tool-eval-multi: --multi-e + --multi-form - (stdin); default -e stays single-form |
| 2026-08-14 | rs/hy local library export + smoke (personal feed; not crates.io/PyPI) |
| 2026-08-14 | cljs local library export + smoke; bin/host-library-smokes aggregator |
| 2026-08-14 | tool-eval single-form stdin `-`; HOST_IMPORT local-export table for all hosts |
| 2026-08-14 | CI: local export script layout + clj/hy(/cljs) library smokes on host-import |
| 2026-08-14 | clr host-import ./smoke (HelloPnix); machine unsupported-node docstring honesty |
| 2026-08-14 | export-pnix-clr-library per-TFM build fix; pnix-clr-library-smoke; HelloPnix project-ref first |
| 2026-08-14 | host-env residual cut closed enough; tool-eval failures carry :profile; examples smoke aggregator |
| 2026-08-14 | clr-meta-tool-surface-gate: full admitted CLI matrix in clr-meta-gate |
| 2026-08-14 | machine differential +4 dotted-let rows; F2 Jones measured witness todo closed |
| 2026-08-14 | oracle D: `++` requires list operands (was wrong VALUE via Clojure concat nil) |
| 2026-08-14 | oracle D: `//` requires attrset operands (was wrong VALUE via Clojure merge nil) |
| 2026-08-14 | host-import CI: clr examples smoke SKIPs without substrate (not FAIL) |
| 2026-08-14 | oracle D: attrNames/attrValues/elem/genList reject null/bad length (wrong VALUE) |
| 2026-08-14 | oracle D: fromJSON/compareVersions/dirOf/baseNameOf/toJSON-fn; with non-attrset no-op |
| 2026-08-14 | Day-1 checklist in HOST_DEV_ENV; host-import-smoke python3; tool-eval result keys doc |
| 2026-08-14 | oracle D: select-or no longer swallows intermediate missing-attr; catAttrs/listToAttrs types |
| 2026-08-14 | oracle D: hasAttr/intersectAttrs/mapAttrs/groupBy reject null (was false/{}) |
| 2026-08-14 | oracle D: zipAttrsWith null, genericClosure non-set, elemAt float, replaceStrings len |
| 2026-08-14 | oracle D: catAttrs name must be string; getAttr requires attrset |
| 2026-08-14 | oracle D: baseNameOf \"/\" is \"\" not null (Clojure split edge) |
| 2026-08-14 | oracle D: match/split reject empty regex (Java would match every pos) |
| 2026-08-14 | oracle D: path + string/path concatenation (was over-strict held) |
| 2026-08-14 | oracle D: elemAt OOB/negative → :elem-at-index-out-of-bounds (not throwable) |
| 2026-08-14 | oracle D: path `<`/`lessThan` (stored path-text order; was incomparable) |