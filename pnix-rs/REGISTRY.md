# REGISTRY — 능력·게이트·로드맵 통합 인덱스 (중복개발 방지)

> **새 기능을 만들기 전에 여기서 grep 하세요.** 이미 구현된 것과 앞으로 구현할
> 것을 한 곳에 모아, 누락·중복개발을 막습니다. 각 항목은 **게이트로 증명**되거나
> **proposal로 등록**되어 있습니다. (pnix-hy의 CAPABILITIES/proposals 방식을
> Rust 두 lane에 적용.)

두 lane 모두 crates.io 의존 0(std만). rs-meta는 네이티브 tier용으로 rustc만 호출.

---

## 1. 이미 구현됨 — 게이트 레지스트리 (truth = code)

### pnix-rs (Rust↔px 프론트엔드) — 18 게이트, `pnix-rs check` all_ready
생성 원천: `pnix-rs capabilities` → `pnix-rs/docs/CAPABILITIES.md`
(drift 게이트 `capabilities-check`, 정합 게이트 `registry-check`).

px · mirror · stage · ir · gate · interop · rust-mirror · specialize ·
incremental(demand-driven 변경 전파: 독립 변경→그것만, 피의존→전이적 의존자) · compartment · **tower**(reify/reflect + px 자기해석기 == 네이티브
+ **1·2차 Futamura 사영**) · **bta**(오프라인 BTA + mix 교차검증) ·
**jones**(Jones-optimality) · **certify**(proof-carrying residual) · **cogen**(손으로 쓴 generating extension) · **attest**(typed attestation) · **reflect-tower**(3-Lisp 유한 타워) · **verifying-cache**(캐시 무결성) · **phase**(phase 관측적 분리) · **assumption**(assumed specialization) · **ir-diff**(canonical IR 의미 diff) · **attenuate**(SES capability 생명주기: grant→감쇠→회수, 재확대 불가) · **welltyped**(px→Rust residual이 플로어 typeck로 well-typed — Rust 정적 강점) · action ·
cross-host · substrate(rs-meta interp==rustc==native 3-way) · capabilities ·
registry.
→ 상세: `pnix-rs/docs/CAPABILITIES.md` §게이트 레지스트리.

### rs-meta (Rust-in-Rust meta-circular) — 57 게이트, `bootstrap check` PASS
게이트 원천: `rs-meta/proofs/stage-manifest.tsv` (status 열). rs-meta는 pnix를
전혀 모른다(독립 Rust meta-circular engine). pnix-rs는 CLI 경계로만 호출.

self · tv(interp==rustc) · typeck · roundtrip · **emit-tv(310/310)** ·
**emit-self-host**(방출 번들이 corpus 재생) · **ast-canonical**(제네릭 faithful) ·
**ast-diff**(정본 AST 의미 diff) · **rust-ir**(content-addressed canonical Rust IR
+ format-invariant ir_hash) · **borrow-boundary**(ownership 경계: rustc reason
code 보존, interp≠borrow checker) · **trait-boundary**(supported vs held:
assoc-type/dyn/where/blanket) · **macro-boundary**(fixed vs macro_rules!/proc
held) · source-ast/bundle · stage2/stage3 mirror·fixedpoint·core · stage8~stageN
사다리 · **witness/hash** · **cap** · **trace** · **diag** · manifest · isolation ·
constitution.
→ Plan E1~E4 완주 + peer-engine 보조(rust-ir→verdict.ir_hash, boundary reports).
  상세: `rs-meta/proofs/stage-manifest.tsv`, `rs-meta/todo.md`.

### 배포 (실제 설치 작동)
`flake.nix`: packages(rs-meta/pnix-rs) · apps(pnix-rs/rs-meta/rs-meta-check/
pnix-rs-check/substrate-check) · devShell. `nix build`·`nix run` 검증됨
(래퍼가 rustc/RS_META_BOOTSTRAP 배선, substrate-check 3-way PASS).
예제: `pnix-rs/examples/` 12섹션(각 limit_rust.rs + pnix_rs_way.sh, 전량 실행/컴파일).

---

## 2. 새로 구현할 것 — 로드맵 (held/open, 순위·근거·proposal)

근거: `pnix-rs/docs/research/2026-07-03-metacircular-frontier.md`
(deep-research: 5각 검색 · 15소스 · 3표 적대검증 · 6 findings high-confidence).

| # | 능력 | 성격 | lane/모듈 | proposal |
|---|---|---|---|---|
| 1 | **full 3차 사영** — feature-rich specialiser 자기적용 (bounded cogen DONE; full은 연구 지평) | 연구 프론티어 | pnix-rs tower/bta | [0004] |
| 3 | **P6** — 트레이트 solving / 클로저 projection (수요 시) | 기계적 확장 | pnix-rs rust_mirror | [0001] |
| 4 | **runtime 표면(수요 시)** — int↔float 승격 / 중첩 보간 / string+ / bool / rec / with | 기계적 확장 | pnix-rs px | [0006] |
| 5 | **full S=L** + stage-polymorphic | 연구 지평 | pnix-rs tower | [0007] |
| 6 | research open — step-level bisimulation · N-레벨 collapsing tower [incremental·proof-carrying·finite reflective tower DONE] | 후속 리서치 | pnix-rs tower | [0007] |
| ext | 자매 lane TSV 파일-대-파일 비교 | external 대기 | pnix-rs cross-host | [0007] |

핵심 통찰(finding [5]): fv-제한 등 subject BTA는 **Jones-optimality를 못 올리는
강도 천장** — 다음은 "더 coarsen"이 아니라 위 게이트들. finding [6]: #1이 언어별
meta-circular 잠재력 차이의 정수(Rust만 싸게 얻는 정적 보증).

## 3. proposals (설계/경계 등록)
`pnix-rs/docs/proposals/`: 0001 rust-ast-projection(v1a~v8 DONE) · 0008 peer-engine-adapter(v0 DONE) · 0009 canonical-rust-ir(v0 DONE) · 0002 sorted-attrs
(DONE) · 0003 call-by-need(DONE) · **0004** cogen · **0005** well-typed-residual(DONE) ·
**0006** runtime-surface · **0007** research-frontier-index.
`pnix-rs/SCOPE_LOCK.md` = 경계 선언. `rs-meta/todo.md` = rs-meta 진행 로그.

## 4. 이 레지스트리를 어떻게 최신으로 유지하나 (gate 방식)
- pnix-rs 게이트 목록: `check_commands()`에서 자동 파생(`registry-check`가 누락 감지).
- 로드맵 proposal: `roadmap_items()`가 참조하는 파일 존재를 `registry-check`가 검증.
- docs/CAPABILITIES.md: `capabilities-check`가 코드-생성본과 drift 감지.
- 즉 **레지스트리는 손유지가 아니라 게이트-검증** — 거짓말할 수 없음.
