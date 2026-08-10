# 2026-07-02 — 사다리 닫힘(P0~P13) adversarial 감사

주장: "pnix-rs는 선언된 Rust↔pnix projection scope(P0~P13 milestone-1)에 관해
complete" (SCOPE_LOCK §1). 이 감사는 그 주장을 공격했다.

## 방법
- 경계 grep: interop 밖 fs/subprocess, Python/Hy 오염, rs-meta 내 pnix 흔적,
  crates.io 의존.
- 의미론 edge 프로브: 중복 let 이름(A4 shadow), 중복 attrset 키(Nix 대조),
  sort 안정성(관측 불가 확인).
- 전 check 재실행(수정 전/후).

## 발견 및 조치 (4건 — 전부 수정됨)

**F1 (HIGH, 의미 버그)**: `let x = 1; x = 2; in x` → `1` 반환.
A4 확정 의미("뒤 바인딩이 앞을 shadow")는 `2`. 원인: `px_lookup`의 Rec 프레임
정방향 스캔(첫 매치 승). **수정**: 프레임 내 역방향 스캔. **회귀 가드**:
`runtime/corpus/seed_let_shadow.px`(기대 2) corpus 편입 — mirror/stage/ir/
gate/rust-mirror 전 레인이 자동으로 이 케이스를 물게 됨.

**F2 (MED, Nix 이탈 미기록)**: `{ a = 1; a = 2; }`를 조용히 수용(출력에 중복
키 노출). Nix는 duplicate attribute 에러. **수정**: 파서에서
`duplicate attrset key` 거부(let은 A4대로 중복 허용+shadow 유지). **가드**:
px-check 인라인 프로브(거부 확인).

**F3 (MED, 경계 위반)**: main.rs에 interop 밖 fs 접촉 4곳(`create_dir_all`
×3, `remove_file` ×1) — "모든 host 접촉은 interop 경유" 원칙 위반. **수정**:
`interop::host_ensure_dir`/`host_remove_file`(file-write capability) 추가,
4곳 전부 이관. grep 재확인: interop 밖 fs/Command 0건.

**F4 (LOW)**: `incremental::definition_hashes`가 중복 정의 이름에서 참조
치환이 모호해짐. **수정**: 중복 이름 명시 Err(shadowing let은 content-
addressable 아님 — 정직 거부).

## 발견되지 않은 것 (green 확인)
- Python/Hy 오염 0, rs-meta 내 pnix 0, crates.io 의존 0.
- witness 13필드 렌더 순서 동결(cross-host-check가 상시 가드).
- `builtins.sort` 비안정성: corpus 값 distinct + 동일 원소는 구별 불가라
  관측 불가(SCOPE_LOCK §2 기재 유지).
- tower self_interp: 순차 env prepend라 shadow 의미 이미 정합(later wins).

## 수정 후 재검증
`pnix-rs check` 15 reports **all_ready: true** (23s):
px 11/11 · mirror 10/10 · stage 10/10 · ir 12/12 · gate 16/16 · interop 4/4 ·
rust-mirror 11/11 · specialize 7/7 · incremental 5/5 · compartment 4/4 ·
tower 11/11 · action 4/4 · cross-host 3/3 · substrate 1/1 · capabilities 1/1.
oracles 재수출(10행). substrate-check 3-way 유지(px.rs 수정분 subset 통과).

## 판정
F1은 실제 의미 버그였다 — "닫힘 주장 전 adversarial 재검증" 문화가 실효를
증명. 수정 후 scope-relative 완성 주장은 유지된다:
**Complete w.r.t. the stated Rust↔pnix projection scope (P0~P13 milestone-1).**
