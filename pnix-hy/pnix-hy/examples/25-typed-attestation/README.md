# 25. 타입 있는 attestation — predicate가 붙은 증거 (proposal 0024)

## 무엇을
증거(witness)에 **predicate 타입 URI**를 부여하고 payload를 그 타입에 묶는 `typed_witness`
(in-toto/SLSA 스타일). `is_known_predicate`(유효성), `predicate_of`(역조회), `migrate_predicate`
(deprecated→최신 이관).

## 왜
형식 없는 witness/로그는 (1) 무엇에 대한 증거인지(eval? compile? interop?) 판별할 수 없고, (2) 필수
필드 스키마가 없어 오타·누락이 조용히 통과하며, (3) 예전 형식인지 알 수 없다. predicate 타입은 증거에
**"이것은 무엇에 대한 주장인가"**를 명시한다.

## predicate 종류 (URI)
`action` · `realisation` · `interop` · `eval` … (`PREDICATE_TYPES`), deprecated는
`DEPRECATED_PREDICATES`로 최신에 매핑.

```
w = typed_witness(PREDICATE_TYPES["eval"], {"in_hash":..,"out_hash":..,"status":"lossless"})
predicate_of(w)                        == PREDICATE_TYPES["eval"]   # 역조회
is_known_predicate("https://…/made-up") == False
migrate_predicate(old_uri)             == 최신 uri
```

## 한 줄
> 증거에 **predicate 타입**을 붙이면 — 무엇에 대한 주장인지 명시되고, 알려진/최신 형식인지 판별되며,
> payload가 타입에 묶인다. 공유 witness envelope는 그대로.

## 경계
- predicate/payload는 **payload 레벨**에 얹혀 공유 witness 불변식(in/out/env hash·status)을 유지한다.
  4-lane 미러·정본 평가기 무접촉.
