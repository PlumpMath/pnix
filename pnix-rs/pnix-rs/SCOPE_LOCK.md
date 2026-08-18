# SCOPE_LOCK — pnix-rs

> 권위 있는 경계 선언. 무엇을 구현하기 전에 먼저 읽을 것. 2026-07-02 수립.
> (형식은 pnix-hy SCOPE_LOCK을 따르되, 이 lane의 호스트는 Rust/rs-meta뿐이다.)

## 0. Source of truth

`~/pnix-rs` 저장소 `main` 브랜치가 권위 상태다. 완성/닫힘 주장은 이 브랜치의
커밋과 `pnix-rs check`(all_ready) receipt를 기준으로만 한다.

## 1. Status — scope-relative (이 정확한 표현을 쓸 것)

**맞음:**
> pnix-rs는 현재 선언된 Rust↔pnix meta-circular projection scope(P0~P13
> milestone-1) 안에서 open todo 0으로 수렴했다.
> Complete **with respect to the stated Rust↔pnix projection scope.**

**틀림(쓰지 말 것):** "전체 완성" / "Complete overall" / "the project is
finished." 미완 0은 **항상** scope-relative다.

## 2. 대원칙

> **의도적 placeholder를 미구현으로 재해석해서 구현하지 말 것.**

이 lane의 의도적 held/미주장 경계(각각 todo.md 해당 P의 "명시 미주장" 블록과
docs/CAPABILITIES.md "명시 미지원"에 기록):

- px runtime: path literal/string-context/store 값, URI literal, 중첩 동적
  attr 경로, POSIX ERE 전체 정합, JSON float exponent canonicalization은 held.
  string `+`, bool `&& || !`, `?`, `rec`, `with`, checked int 연산과
  int↔float 승격은 2026-07-10 proposal 0010 tranche에서 개방됐다.
  thunk-memo laziness는 proposal 0003으로 2026-07-03 개방됐고 재귀 let/rec가
  call-by-need로 동작한다. 등록된 public `builtins` 표면은 함수·값 상수·재귀
  self를 합쳐 91종이다. 이는 presence 수이며 `sin/cos/tan/sqrt/exp/ln/log/abs/
  pow/max/min/mod` 12개 extension 이름은 호출 시 fail-closed HELD다. 전체 Nix
  118종과 string-context 의존 동작은 아직 미주장이다.
  비유한 float은 관찰·비교할 수 있지만 유효 px source로 roundtrip되는
  canonical print는 held다.
- P6: v1a~v7 DONE — mirror_probe.rs 전량 + **제네릭 함수**(fn<T>/G() 타입,
  rs-meta sig faithfulness 전제). held: v8(제네릭 struct/impl<T>) + 트레이트
  solving/클로저 + 비균형-브래킷 char 리터럴 소스(proposals/0001).
- P11: **3차 사영 완주 = BTA-driven generalization**(m9+ — BTA 분석 facet은
  m8로 존재[예측+mix 교차검증, 상한 관계 명시], 온라인 배선은 large rewrite +
  종결성 미보장 연구 지평. m5 수용 기준이 판정자)·full S=L·stage-polymorphic
  전체. DONE: m2~m6f + m7 fv-제한 spec 키 + m8 오프라인 BTA(bta-check 6/6).
- P9: SCC 사이클 내부 이름이 그룹 해시에 포함되는 v0 경계.
- P5: OS 수준 샌드박싱(capability는 lane 내부 admission 규율).
- P13: 자매 lane 파일 대 파일 자동 비교(그쪽 TSV export 생기기 전까지).
- `builtins.sort` 비안정(corpus 값 distinct라 관측 불가; 요구 발생 시 명시 수정).
- 비유한 float(inf/NaN): 값 canonical print("inf")가 유효 px 소스가 아님 —
  P1 print-is-source 성질의 예외(pnix-hy repr도 동일). toJSON/rust-mirror는
  비유한에서 명시 에러/held(2026-07-03 감사 #2).

## 3. 절차

- 새 기능/경계 이동은 `docs/proposals/NNNN-*.md`로 시작한다.
- 스키마 동결: witness 13필드(이름·순서), roundtrip 어휘(lossless/lossy-ok/
  held/rejected), effect 어휘(file-read/file-write/host-call/import/network),
  `pnix-rs.*.v0` receipt들. 변경은 vN+1 + 마이그레이션 명시.
- 두 번째 평가기/mirror/gate 금지. 모든 평가는 `src/px.rs` sacred runtime 경유.
- zero crates.io dependency. Python/Hy 불가촉(pnix-hy는 구조 모범일 뿐).
- rs-meta에 pnix 코드 금지(필요 기능은 pnix 무관 범용으로 제안).
- px.rs는 rs-meta evaluated subset 안에 유지(substrate-check가 게이트).

## 4. 걷지 않는 길

todo.md §4.0 그대로: 에이전트/coding-agent 런타임 ❌, task routing/plan
synthesis ❌, MSV/gate-graph ❌, corpus 표면 갈기 자체가 목적 ❌.


---

## OWNER AMENDMENT 2026-07-08 — import/module system + shared-core admitted IN scope (B6)

Owner-authorized. Previously `import` was a reserved/held effect word and the
runtime had no import/module system. Now IN scope for this repo:

- an **import / module / resource resolver + export ABI** for `.px`
  (blocker B2 — the current hard blocker); B1–B3 originally tracked in the
  `pnix-zero` sibling repo's project-wiki, which this self-contained tree
  does not have;
- loading common `.px` from `../pnix-meta` and running it;
- a px-level canonical result + held reason (B1) and the effect/capability
  bridge (B3).

Bound by the constitution (`../CLAUDE.md`):

1. **Meta-first** — the `rs-meta` interpretable-subset constraint still governs
   runtime growth; every new `px.rs` surface must stay `substrate-check`-clean.
   Grow the substrate first, then the surface (no cram).
2. **Non-regression** — `check` / `substrate-check` / `cross-host-check` stay
   green; the shared-core track is additive.
3. Documented value divergences (no int↔float promotion, dynamic-attr
   first-wins, non-stable sort) are B4 convergence work, tracked separately.
