# 0007 — 연구 프론티어 인덱스 (deep-research open + 지평)

상태: 인덱스(다수 held/open). 근거: docs/research/2026-07-03-metacircular-frontier.md.

deep-research가 확증한 로드맵 중 이 저장소 스코프 밖이거나 연구 지평인 항목을
한 곳에 등록(중복·누락 방지). 개별 착수 시 별도 proposal로 승격.

## open (리서치가 다루지 못함 — 후속 리서치)
- (c) translation-validation / certified-compilation 게이트 — **[부분 DONE
  2026-07-03]** proof-carrying residual: certify-check(3/3) — 특화 residual이
  소스와 입력 배터리 전체 동등함을 재검증 가능 인증서로(증명기 없이, differential
  testing). 잔여 open: bisimulation(step-level), CompCert류 certified compilation
- (d) reflective tower / collapsing towers (Amin&Rompf, 3-Lisp/Black) —
  **[finite form DONE 2026-07-03]** reflect-tower-check(3/3): reify/reflect가
  인코딩을 다시 인코딩해도 2-레벨 coherent(P1 성질) + 메타-레벨 의미 투명.
  잔여 open: N-레벨 collapsing tower(self_interp 인코딩 필요 — 자기적용 무게 벽)
- (e) content-addressed incremental (Unison/salsa/adapton demand-driven) —
  **[DONE 2026-07-03]** 의존성-치환 해시에 내재된 최소-재계산을 게이트화:
  incremental::changed_between + incremental-check(8/8) — 독립 변경 → 그것만,
  피의존 변경 → 그것+전이적 의존자, 무변경 → 0. IR-content-hash + realisation-
  cutoff 위에서. 잔여 open: (c) TV/certified-compilation, (d) reflective towers

## 연구 지평 (알려진 어려움)
- full S=L (전 표면 poly)
- stage-polymorphic 전체
- 3차 사영 완주 (→ 0004 손으로 쓴 cogen이 가장 tractable한 경로)

## external 대기
- 자매 lane(pnix-clj/pnix-hy) TSV 파일-대-파일 비교 (그쪽 export 생기기 전까지)
