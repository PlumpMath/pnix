# 13 — peer engine (공통 .px 평면 위의의 Rust 도메인 엔진 rs-meta)

**왜 “그냥 Rust 컴파일러”가 아니라 rs-meta 인가:** rs-meta 는 *meta-circular*
Rust 컴파일러/평가기다 — Rust 안 인터프리터를 translation validation 으로
`rustc` 네이티브 티어와 같게 유지한다 (interp stdout == rustc stdout).
pnix-rs 는 이를 **peer 엔진**으로 다루고, Rust 쪽 TV 결과를 공통 `.px`
제어 평면에 올린다. pnix-hy / pnix-clj peer 엔진이 내는 것과 같은 모양.

- `limit_rust.rs` — plain rustc 의 한계: 바이너리는 나오지만, TV·내용주소·
  라우팅 가능한 verdict 는 없다.
- `pnix_rs_way.sh` — 제어 평면 툴킷 end-to-end:
  **attestation**(왜 믿을지) → **profile**(능력) → **verdict**
  (interp==rustc, .px) → **routing**(accept vs held → 다른 엔진) →
  **artifact**(빌드 영수증) → **batch**(프로젝트 단위 집계) → **verify**
  (tamper-evident, trust-free).

## 분리 (constitution)
- **rs-meta** 는 pnix 를 모른다 — 독립 Rust meta-circular 엔진. pnix-rs 는
  bootstrap CLI(프로세스 경계)로만 호출한다.
- **`.px` 가 제어 평면**이고, Rust 소스는 이 엔진의 도메인 payload.
- verdict/profile/artifact 는 실제 `.px` 값(attrset)이라, 제어 평면이 일반
  px 기계로 평가·해시·라우팅한다.

## 봉투 (envelope)
- `pnix.engine.profile.v0` — supports / does_not_support (정직한 held 프론티어:
  full-borrowck, macro-rules, full-trait-solver).
- `pnix.engine.verdict.v0` — status (accepted|held|rejected) + verdict_kind
  (ok | negative-boundary-agrees | divergent | incomplete-subset | held-*) +
  source_hash + ir_hash (형식 불변 정본 Rust IR) + interp/native 출력 해시 +
  tv_equal + witness_id.
- `pnix.engine.artifact.v0` — rust-native 빌드 영수증 (rustc 버전, artifact
  해시, receipt 해시).

## 제어 평면 툴킷
- `engine-request` — `.px` 요청 봉투 (pnix.engine.request.v0), phase 로 디스패치.
- `engine-verdict` — status/verdict_kind/reason_code(rustc E-code)/surface
  (held-*)/ir_hash/interp+native 해시/tv_equal/witness_id.
- `engine-artifact` — 재현 가능한 네이티브 빌드 영수증.
- `engine-profile` / `engine-attestation` — 능력 + 신뢰 신호
  (interp==rustc TV 커버: positive 310 + negative 257; substrate 3-way).
- `engine-verify` — verdict 자기 필드에서 witness_id 재계산으로 변조 탐지
  (proof-carrying verdict; 제어 평면이 미신뢰 엔진을 검증).
- `engine-batch` — 소스 목록 `.px` 를 verdict 매니페스트로.

## 게이트
`engine-verdict-check` · `engine-artifact-check` · `engine-request-check` ·
`engine-attestation-check` · `engine-verify-check` · `engine-batch-check`.
제안 0008 (peer-engine adapter), 0009 (canonical Rust IR) 참고.
