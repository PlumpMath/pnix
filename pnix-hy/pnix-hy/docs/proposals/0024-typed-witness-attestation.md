# 0024 — predicate-typed witness 증명 (0013 R4 승격; envelope 무변경 설계)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, `pnix_hy/gate.py` 한정.
- 근거: in-toto Attestation(버전드 predicate type URI; 스키마 진화는 URI 개명·deprecate) — 3-0 검증.
- 핵심 설계 결정: predicate type을 **witness payload 안에** 넣는다(`make_witness`의 payload는
  자유 필드) — 공유 §14 envelope 필드는 **불변**이므로 양-레인 조율/drift-guard 변경이 불필요.

## 딜리버러블
`PREDICATE_TYPES` 레지스트리(버전드 URI), `DEPRECATED_PREDICATES`(구 URI→현 URI 마이그레이션 맵),
`typed_witness(predicate_uri, payload, kind)`, `predicate_of(witness)`, `migrate_predicate(uri)`.

## 수용: 신규 `typed_witness_report` 등록(+1) — URI 임베드·결정성·deprecated 마이그레이션·미지 URI
검출 + **envelope 불변 검증**(`witness_schema_ok` 유지, witness 키 집합 기존과 동일).
