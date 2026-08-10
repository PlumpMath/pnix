# 0008 — peer-engine adapter (pnix-rs as a Rust-domain engine on a common .px plane)

상태: **v1 DONE (section 6 interop 완결: profile·verdict·TV→status·ir_hash·artifact·witness_id)** / v0 DONE (2026-07-03)** — engine-verdict-check 4/4, aggregate 30 reports.

## 동기 (사용자 분석 section 6)
pnix-rs가 rs-meta를 쓰는 이유는 "그냥 Rust compiler/evaluator"가 아니라
**meta-circular** Rust compiler/evaluator가 필요하기 때문. 그리고 rs-meta는
pnix를 모르는 독립 엔진으로 유지되어야 한다(헌법). 따라서 연결은 rs-meta에
pnix를 섞는 게 아니라, pnix-rs 안의 **adapter / interop / 공통 verdict
envelope**로 한다. `.px`는 공통 control plane, pnix-rs는 Rust domain engine.

## 구현 (src/engine.rs — pnix-rs 전용, rs-meta 무접촉)
rs-meta를 오직 bootstrap CLI(process 경계)로만 호출. rs-meta는 pnix를 전혀
모른다.

- **engine profile** (`pnix.engine.profile.v0`): supports(rust-parse/typeck/
  interp/rustc-native/translation-validation/native-artifact-receipt/stage-
  manifest/ast-canonical) + does_not_support(px-eval-direct/full-borrowck/
  macro-rules/full-trait-solver/cargo-crate-graph) — held 프론티어를 정직하게.
- **engine verdict** (`pnix.engine.verdict.v0`): Rust source를 rs-meta의
  run(interp)/native-run(rustc)/typecheck로 돌려 TV→status 분류. accept=exit0
  (Ok), reject=exit1(Err). 매핑(사용자 taxonomy 충실):
  - interp==native 실행 + typeck ok → accepted/ok (tv_equal=true)
  - 둘 다 거부 → accepted/negative-boundary-agrees (거부 경계 합치)
  - interp accept, rustc reject → rejected/divergent
  - interp reject, rustc accept → held/held-out-of-subset(선언 subset 갭) |
    rejected/incomplete-subset
  - rustc 없음 → held/held-rustc-unavailable
- verdict/profile은 **실제 .px 값**(attribute set) — control plane이 px 기계로
  eval/hash/route 가능. 게이트가 매 verdict를 px로 재파싱해 증명.

## 요청/응답 프로토콜 (section 6 A+B DONE)
engine-request -c|-f: pnix.engine.request.v0 .px 봉투를 phase로 디스패치
(eval-rust->verdict, artifact->artifact, profile->profile). 미지 phase 에러.
engine-request-check(4/4).

## 검증 가능 verdict (proof-carrying)
engine-verify: verdict의 witness_id를 증거 필드에서 재계산해 일치 확인 —
분산 제어 평면이 untrusted 엔진 verdict를 신뢰 없이 검증(변조 감지).
engine-verify-check(3/3). engine-attestation: TV 커버리지+substrate 3-way 신뢰 신호.

## surface 필드 (section 5 boundary ↔ verdict)
verdict.surface = rs-meta rust-surface 분류(held-macro-rules/held-assoc-type/
dyn 등) 또는 ok. rs-meta가 분류 소유, adapter가 CLI로 소비.

## CLI
- `engine-profile` — 프로필 .px 값 출력.
- `engine-verdict -c|-f <rust>` — Rust source의 공통 verdict .px 값 출력.

## 게이트 engine-verdict-check (4/4)
(1) profile이 유효 px + held 프론티어 정직, (2) good Rust → accepted/ok +
tv_equal=true + verdict가 유효 px, (3) ill-typed → accepted/negative-boundary-
agrees, (4) verdict 결정성. rustc 없으면 held-rustc-unavailable로 정직 skip.

## 남은 것 (v1+)
- Rust artifact receipt export: **DONE** — engine-artifact(pnix.engine.artifact.v0) .px 봉투, rs-meta rust-artifact 호출(stage8-repro per-program).
- ir_hash 필드: **DONE(0009)** — rs-meta rust-ir 호출로 채움(포맷 불변 canonical IR).
- witness_id 연결: **DONE** — verdict.witness_id = 증거 튜플(status/hashes) content-address(wit:...), 라우팅/dedup 가능.
