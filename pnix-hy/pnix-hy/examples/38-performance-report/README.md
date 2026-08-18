# 38. Performance report — 공유 런타임 레인별 벤치마크

## 무엇을
`performance_report`는 프로세스 시작 잡음을 뺀 채로 parse, 정본 emit,
컴파일러 emit, Python `compile()`, 인터프리터 eval, 컴파일러 compile+exec,
컴파일 1회+실행 반복(exec-many)까지 **런타임의 각 레인**을 따로 계측한다.
`19-compiled-runtime`이 "컴파일 런타임이 인터프리터보다 빠르다"는 결과를
보인다면, 이건 그 결과를 만드는 **파이프라인의 어느 단이 얼마나 걸리는지**
분해해서 보여준다.

## 왜
"느리다"는 감으로만 안다. plain Python 프로파일러는 함수 단위로 시간을
재지만, "이 언어 파이프라인의 parse/emit/compile/exec 각 레인"이라는
의미 단위로 쪼개서 반복 측정·비교하는 표준 도구는 없다.

## 무엇을 게이트하나
| 항목 | 값 |
|---|---|
| timings | parse/canonical-emit/compiler-emit/py-compile/interp-eval/compile+exec/exec-many 각각 total_ns·per_iter_ns |
| generated_python_sha256 | 생성된 Python 소스의 내용주소(결정성 확인용) |
| bytecode_op_count / bytecode_code_len | 컴파일된 바이트코드 규모 |

## 한 줄
> "컴파일된 실행이 빠르다"를 주장 대신, parse부터 exec-many까지 레인별
> 나노초 단위 수치로 쪼개 보여준다.

## 경계
- 벤치마크 수치이지 의미(semantic) 게이트가 아니다 — 결과가 맞는지는
  `19-compiled-runtime`/스테이지 tower 쪽이 증명한다. 여기는 **어디가
  느린가**를 보는 도구.
