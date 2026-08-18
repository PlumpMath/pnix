# 37. Eval receipt — 재현성/감사 영수증 한 장

## 무엇을
`eval_receipt`는 한 계산에 대해 정본 emit + 소스 해시, 값 + 값 해시, 4-lane
수렴(host interp/compiler + stage7 runtime/compiler) 판정, 실행 흔적
(distinct 함수 수/총 호출/총 opcode), 순수성을 **한 장의 결정적 영수증**으로
묶는다. `05-witness-and-gate`(단일 witness)와 `11-self-hosting-convergence`
(4-substrate 수렴)를 하나의 재사용 가능한 문서로 합친 상위 뷰다.

## 왜
"이 계산을 재현할 수 있는가"를 감사하려면 소스 해시, 값 해시, 여러 실행
경로의 수렴 여부, 실행 흔적을 각각 따로 확인해야 한다. plain Python에는
이걸 하나의 재검토 가능한 문서로 묶는 개념이 없다 — 로그를 뒤져야 한다.

## 무엇을 게이트하나
| 항목 | 값 |
|---|---|
| source_sha256 / value_sha256 | 결정적(같은 소스 → 같은 두 해시) |
| convergence.converged | host_interp/host_compiler/stage7_runtime/stage7_compiler 4-lane 일치 |
| trace | runtime_functions_distinct/runtime_calls_total/runtime_opcodes_total(실행 흔적, 0 아님) |
| pure | 정적 순수성 판정 동봉 |

## 한 줄
> `let x = 1; y = 2; in x + y` 하나의 영수증에 "무엇을 계산했나(값=3),
> 왜 믿을 수 있나(4-lane 수렴), 무엇을 실행했나(흔적)"가 전부 담긴다 —
> 그리고 같은 소스는 항상 같은 두 해시를 낸다.

## 경계
- 영수증은 **기존 게이트들을 재사용해 조립**한 문서다(새 진리를 만들지
  않음). 개별 정합성은 `05`/`11`이 이미 증명한다.
