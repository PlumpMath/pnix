# 2026-07-03 — meta-circular 프론티어 (deep-research)

언어별로 meta-circular의 가능성·잠재력이 다르다 — Rust(정적/소유권/스테이징/
translation-validation)와 동적 Lisp(pnix-hy/Hy, 코드=데이터)는 열 수 있는 문이
다르다. deep-research(5각 검색 · 15소스 · 3표 적대검증)로 "이 아키텍처에서
구현 가능하나 아직 안 된 것"을 순위화. 6 findings 전부 high-confidence(3-0).

## 순위 로드맵

1. **Jones-optimality 게이트 [DONE 2026-07-03]** — "specializer가 해석 계층을
   실제 제거했는가"의 측정 가능 속성(Glück; JGS strict form). 조작적 게이트:
   인터프리터 bloat에도 residual 불변. → jones-check 4/4.
2. **잘-타입된 residual 게이트 [DONE 2026-07-03]** — px→Rust residual이 rs-meta
   플로어 typeck로 구성상 타입-정합(Brown&Palsberg, POPL'18). Rust 정적 강점의
   정수 — 동적 Lisp이 싸게 못 얻음. → rs-meta `typecheck` + welltyped-check 5/5.
   모듈: typeck/rust_mirror. 게이트 = type-preservation / well-typed residual.
3. **손으로 쓴 cogen**(generating-extension) — 3차 사영을 *자기적용 없이* 우회
   (Leuschel et al., cs/0208009). **[bounded DONE 2026-07-03]** 산술 객체언어
   cogen(cogen_int.px) + cogen-check 3/3. full 3rd projection(feature-rich
   자기적용)은 연구 지평 held.
4. **종결성 = local + global 2 의무** — size-change/strong-termination(정적
   인자에 strictly-decreasing self-edge ⟹ 종결) + mgg generalization point로
   bounded polyvariance(0903.2202). 모듈: bta. 게이트 = 종결 인증서.
5. (제약) Jones-optimality는 subject BTA로 못 올리는 강도 천장(Glück) —
   m7 fv-제한이 3차 사영 폴리바리언스를 못 잡은 이유를 문헌이 확증. mix 자체가
   Jones-optimal이어야 함(비-BTI 경로: incremental/interpretive/BTI-universal).
6. (확증) px 온라인 specializer = self-interpreter의 기계적 변형(memoize on
   fn+static-args + recursion placeholder; Cook&Lämmel, 1109.0781) — **이미
   구현**(poly_mix pending-spec seeding).

## pnix-hy 예제 매핑 (part g) — 이식된 것
- 25-typed-attestation -> **[DONE 2026-07-03]** attest-check(4/4): witness에
  predicate 타입 URI + subject(content hash); 불일치 predicate 위조 거부.
- jones-optimality -> jones-check(DONE) · efficient-cogen -> cogen-check(bounded
  DONE) · incremental -> incremental-check demand-driven(DONE) ·
  meaning-preservation-roundtrip -> mirror-check(DONE) · 30-verifying-cache -> verifying-cache-check(DONE) · 22-phase-separation -> phase-check(DONE) · 32-assumed-specialization -> assumption-check(DONE) · 29-ir-diff -> ir-diff-check(DONE) · 23-capability-attenuation -> attenuate-check(DONE) · 17-unified-explain -> explain/explain-check(DONE) · 21-poly-optimizations(sharing/eta/let-insertion)는 부분 보유(etaBody/fv-제한/call-by-need), 명시 게이트 미구현.

## 리서치가 다루지 못한 부분(open)
(c) translation-validation/certified-compilation 게이트, (d) reflective tower/
collapsing towers(Amin&Rompf), (e) content-addressed incremental(Unison/salsa/
adapton), (g) 명시적 pnix-hy 예제 매핑 — 후속 리서치 대상.

## 1차 소스
- Cook & Lämmel, "Tutorial on Online Partial Evaluation" (arxiv 1109.0781)
- Brown & Palsberg, "Typed Self-Applicable Meta-Circular ..." (POPL'18)
- Leuschel et al., cogen without self-application (arxiv cs/0208009)
- Glück, Jones optimality & strength of specializers
- size-change/generalization (arxiv 0903.2202)
