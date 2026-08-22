# 호스트 env P2 / P3 + 다음 제품 트랙 (계획)

**상태:** 이중 축 + 라이브러리 import **충분히 닫힘** (2026-08-14).  
**교리 / 1일차:** [HOST_DEV_ENV.md](HOST_DEV_ENV.md) · [HOST_IMPORT.md](HOST_IMPORT.md)  
**로컬 회귀:** `./bin/host-env-residual-smoke` (또는 HM 후 PATH `./bin/host-import-smoke`)

이 파일은 선택 후속 작업에 대한 **소유자용 계획**이다. 어려운 항목은
명시적으로 끌어올 때까지 계획 상태로 둔다. 쉬운 항목은 작은 예제/CI로 착지 가능.

---

## P2 — 품질 / 편의 (host-import 트랙)

### P2.1 주기 스모크 (ops)

| 필드 | 내용 |
|------|------|
| 목표 | pnix 입력을 올리는 모든 `dot-nix` rebuild 후 import 스모크 재실행 |
| 방법 | `~/pnix/bin/host-import-smoke` |
| 완료 기준 | 문서화된 습관 + 선택 CI (P2.3) |
| 노력 | 사소 |
| **상태** | 스크립트 + 문서로 **착지**; 습관은 운영자 측 |

### P2.2 실제 미니 프로젝트 (host-main 데모)

| 호스트 | 스켈레톤 | 완료 기준 |
|--------|----------|-----------|
| clj | `examples/host-import/clj/` + `deps.edn` local/root | `clojure -M -m smoke` → 3 출력 |
| cljs | `examples/host-import/cljs/smoke.mjs` | `node smoke.mjs` → 3 (NODE_PATH / HM 필요) |
| hy | `examples/host-import/hy/smoke.py` | `python smoke.py` → 3 |
| rs | `examples/host-import/rs/README.md` path-dep 또는 link flags | 문서화; 선택 tiny crate 이후 |
| clr | `pnix-clr/csharp/examples/HelloPnix` + props sample 가리킴 | 이미 존재 |

| 필드 | 내용 |
|------|------|
| 노력 | 작음 |
| **상태** | 스켈레톤 + **rs path-dep crate** `examples/host-import/rs/pnix-rs-smoke` **착지** |

### P2.3 import 회귀 CI

| 필드 | 내용 |
|------|------|
| 목표 | PR이 스모크/예제를 모르게 삭제하지 못하게 |
| Phase A (쉬움) | 레이아웃 + `bash -n` + **clj 예제** + **rs cargo path-dep** |
| Phase B (중간) | `pnix-*-library` / `*-refs` env를 출력하는 호스트별 flake 잡 |
| Phase C (어려움) | 로컬 스모크처럼 5호스트 full eval (멀티 툴체인 매트릭스 필요; A/B가 수주 green 전까지 **시작 금지**) |
| **상태** | `.github/workflows/hosts.yml`에 **Phase A+B 착지** (B = clj/hy/rs; cljs/clr는 호스트 게이트). library printer용 ShellCheck SC2012 수정됨. |

### P2.4 공개 API 다듬기 (문서 위주, churn 없으면)

| 호스트 | 작업 | 우선순위 | 상태 |
|--------|------|----------|------|
| clj | `IMPLEMENTATION.md` §11을 정본으로 유지; API 성장 시에만 확장 | low | 문서 착지 (옛 `docs/HOST_IMPORT.md` 흡수) |
| hy | `__all__` 고정; 선택 py.typed | low | **py.typed 착지** (PEP 561 마커) |
| cljs | scoped require 검증; 로컬 export 착지; npm 게시 **폐기** | — | **export+smoke** |
| rs | IMPLEMENTATION.md §7 (옛 CARGO_HOST_IMPORT.md) 완료; 헤더 C ABI semver 정책 주석 | low | **ABI 주석 + version pin 노트 착지**; full semver 프로세스는 아직 P3 |
| clr | 로컬 nupkg pack 완료; nuget.org **폐기** (로컬 피드만) | — | — |

### P2.5 push 시 전용 host-import 워크플로

| 필드 | 내용 |
|------|------|
| 목표 | host-env 파일을 건드리는 main 브랜치 push가 full gate 매트릭스 없이 import CI를 받음 |
| 방법 | `.github/workflows/host-import.yml` (path-filtered push + PR) |
| **상태** | **착지** — 레이아웃/예제 + library-print (clj/hy/rs); hy 예제는 flake 패키지 경유 |

---

## P3 — 배포 / 대형 제품 (계획만)

명시적 제품 결정 없이 **시작하지 말 것**.

### P3.1 레지스트리 게시

| 레지스트리 | 패키지 | 블로커 |
|------------|--------|--------|
| Maven | `pnix-clj` jar | 버저닝, source jar, 공개 네임스페이스 범위 |
| npm | `@plumpmath/pnix-cljs` | 스토어에 scoped 이름 있음; CI publish 시크릿 필요 |
| crates.io | `pnix-rs` | 현재 `publish = false`; zero-deps 스토리 유지 필수 |
| nuget.org | `Pnix.Clr` | **안 함 (소유자 2026-08-14)** — 개인/로컬 피드만; pack+smoke 유지 |

### P3.2 완전한 ClojureCLR 프로젝트 스토리

| 목표 | `-e` / 단일 파일을 넘는 드롭인 `deps.edn` / project.clj 스타일 CLR 호스트 |
| 수락 | pnix 전용 CLI 없이 문서화된 REPL + 멀티파일 로드 + Reference 어셈블리 |
| 의존 | clr-meta substrate 안정성, pnix-clr guest AOT만이 아님 |
| **계획 상세** | `pnix-clr/clr-meta/todo.md` § “P3 full ClojureCLR project” |
| **Step 1 inventory** | **착지** — `pnix-clr/docs/CLOJURE_CLR_ADMITTED_SURFACE.md` |
| **Step 2 TFM** | **착지** — `pnix-clr/docs/TFM_POLICY.md` |
| **Step 3 template+smoke** | **착지** — `pnix-clr/examples/clojure-clr-project/` (bootstrap multi-ns → 42) |
| **Profiles smoke** | **착지** — `bin/clojure-clr-profiles-smoke` (`pnix-clr-gate`에도 포함) |
| **tool-eval-multi** | **착지** — `--multi-form FILE\|-`, `--multi-e FORM` + named gate |
| **Local nupkg smoke** | **착지** — `bin/pnix-clr-nupkg-smoke` (로컬 피드만) |
| **nuget.org publish** | **폐기** — 소유자는 로컬 피드만; nuget.org 트랙 없음 |
| **In-process C# eval** | **실험** — net10 + parity gate; substrate 있을 때 aggregate |
| **clj local export** | **착지** — `export-pnix-clj-library` + library-smoke (로컬만) |
| **rs local export** | **착지** — `export-pnix-rs-library` + library-smoke (로컬만) |
| **hy local export** | **착지** — `export-pnix-hy-library` + library-smoke (로컬만) |
| **cljs local export** | **착지** — `export-pnix-cljs-library` + library-smoke (로컬만) |
| **tool-eval stdin** | **착지** — single-form `-` + multi-form `--multi-form -` |
| **clr host-import smoke** | **착지** — `examples/host-import/clr/smoke` (HelloPnix) |
| **clr library smoke** | **착지** — `pnix-clr-library-smoke` (export API + nupkg + HelloPnix) |
| **host-env residual cut** | **충분히 닫힘 (2026-08-14)** — 로컬 피드, 예제, tool-eval 표면, CI 레이아웃 |
| **tool-eval surface gate** | **착지** — `clr-meta-tool-surface-gate`가 허용 CLI 고정 |
| **아직 열림 (제품 필라, host-env 아님)** | machine/F-series if pillar; in-process ALC (blocked); new tool-eval은 named gate와 함께만 |

### P3.3 공통 이식 가능 `.px` 라이브러리 (역사적 pnix-meta)

| 목표 | 동일 의미로 다섯 호스트 모두 로드 가능한 하나의 이식 가능 라이브러리 코퍼스 |
| 상태 | **연기** — 호스트 로컬 import를 막지 말 것 |
| 수락 | 패키징 계약 + 5호스트 게이트 슬라이스 + host-leak builtin 없음 |
| **계획** | 호스트 게이트가 다시 공유 코퍼스를 허용한 뒤에만 재개 |

### P3.4 ABI / 타이핑 계약

| 항목 | 호스트 | 계획 |
|------|--------|------|
| `pnix_rs.h` C ABI semver | rs | version macro + changelog; struct/export 변경 시 bump |
| `py.typed` + stub | hy | 빈 py.typed + `__all__`만 export |
| MSBuild multi-TFM NuGet | clr | net8+net10 이미 multi-target managed DLL |

---

## 인접 제품 트랙 (계획만 — host-env 아님)

“host-env 다음”으로 나열됐던 항목. **이번 컷에서 구현 없음.**

### A. clj residual / 제품 residual

| 정본 | `pnix-clj/pnix-clj/docs/TODO.md` + `docs/PLANS.md` |
| 규칙 | 새 residual 메뉴 발명 금지; pillar-driven (M-series) 또는 oracle 분기만 |
| 다음 후보 (소유자 선택) | machine fragment 성장 (pillar일 때); Phase D **연기** |
| Host-import 상호작용 | 불필요 — `eval-file` / classpath inject 이미 green |
| **상세** | `pnix-clj/pnix-clj/docs/TODO.md` § “언어 정합성 / 레인 커버리지”·“Machine fragment”, `docs/PLANS.md` § “Conformance Phase D” |

#### Oracle D-type 표면 (2026-08-14) — 충분히 닫힘

실제 Nix builtins/operators에 대한 반복 `nix-instantiate` 스윕
(wrong-VALUE, over-strict, both-ok)은 일상용으로 **충분히 green**. 당일 착지:
`++`/`//` operands, attr/list null guards, string/version types,
`toJSON` functions, `with` non-attrset no-op, select-`or` continuous
attrPath (or는 어떤 세그먼트 miss도 catch; 괄호 중간은 여전히 hard-fail),
empty regex, path `+` and path `<`, `elemAt` OOB,
`baseNameOf "/"`, 등.
Machine differential은 같은 값 대수를 추적 (**~220 rows, 0 diverge**).

**의도적 / 비버그 (제품 결정 없이 “고치지” 말 것):**

| 주제 | 입장 |
|------|------|
| Path absolute vs relative | 순수 relative path 텍스트 (`./a`); Nix는 FS로 abs resolve |
| `toString ./a` | relative 텍스트, abs path 아님 |
| Host-only builtins (`mod`, `hasPrefix`, `take`, …) | pnix에 있을 수 있음; stock Nix builtins에는 없을 수 있음 |
| `tryEval` type errors | 전파 (throw/assert false만 catch) — Nix 정렬 |
| int/bool 문자열 보간 | `toString` 없이 에러 — Nix 정렬 (2.34) |
| `builtins.fromTOML` | 미허용 (`builtins ? fromTOML` is false); pure TOML 슬라이스와 함께만 추가 |

**다음 oracle 작업**은 새 nix-instantiate 분기 또는 pillar 필요 시에만 —
체크리스트 갈아 없애기 아님.

### B. clr-meta residual

| 정본 | `pnix-clr/clr-meta/todo.md` + `STATUS.md` + stage design docs |
| 규칙 | meta 우선; receipt 없이 Stage15/N promotion 주장 금지 |
| 다음 후보 | admitted eval 표면 신중 확대; compiler stage ladder 정직성; full CLR project story (P3.2) |
| Host-import 상호작용 | export library가 이미 artifact builder에 의존 |
| **상세** | `pnix-clr/clr-meta/todo.md` § “Post host-env plan (2026-08-14)” |

### C. 기타 호스트 residual

| hy | `pnix-hy/pnix-hy/docs/TODO.md`·`docs/PLANS.md` — gate green; 미확정 연구 후보는 PLANS |
| rs | `pnix-rs/pnix-rs/docs/TODO.md`·`docs/PLANS.md` — 제품 TODO는 비어 있고 substrate/stage 후속은 PLANS·rs-meta |
| cljs | `pnix-cljs/pnix-cljs/docs/TODO.md`·`docs/PLANS.md` — 제품 TODO는 비어 있고 미확정 방향은 PLANS |

---

## 결정 로그

| 날짜 | 결정 |
|------|------|
| 2026-08-14 | Host dual-axis + library import를 일상 dev env에 **닫힘**으로 선언 |
| 2026-08-14 | P2.2 스켈레톤 + P2.3 Phase A CI **시작**; P3 registry/full CLR/common-.px **계획만** |
| 2026-08-14 | clj residual / clr-meta / full examples / heavy CI = 소유자 pull 전까지 **todo 상세만** |
| 2026-08-14 | P2.2 rs mini crate + P2.3 Phase B library-print matrix **시작** |
| 2026-08-14 | P2.4 py.typed + rs ABI header note; P2.5 push 시 `host-import.yml` |
| 2026-08-14 | P3.2 step1 clojure-clr inventory; clj multi-module import example |
| 2026-08-14 | P3.2 step2 TFM + step3 bootstrap multi-ns project smoke |
| 2026-08-14 | P3.2 named profiles + clojure-clr-profiles-smoke (4/4) |
| 2026-08-14 | tool-eval-multi --multi-form + clr-meta-tool-eval-multi-gate |
| 2026-08-14 | profiles-smoke를 pnix-clr-gate에 연결 (~17s) |
| 2026-08-14 | local nupkg pack smoke (`pnix-clr-nupkg-smoke`); nuget.org는 여전히 owner-gated |
| 2026-08-14 | M1 per-call `:fold-fuel`; nuget publish fail-closed; in-process eval design |
| 2026-08-14 | in-process eval spike (net10 ALC) + parity gate (opt-in, not aggregate) |
| 2026-08-14 | in-process corpus 17-pass; isolated ALC held (CLR Default load); host-artifact API |
| 2026-08-14 | host-artifact report rows; nupkg-smoke in gate if export; INPROCESS opt-in gate |
| 2026-08-14 | nuget.org publish **폐기** (local-only); inprocess gate auto when substrate+artifact |
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
| 2026-08-14 | oracle D-type surface declared closed enough; intentional pure-path notes |
