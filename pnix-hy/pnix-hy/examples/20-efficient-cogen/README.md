# 20. efficient cogen — 3차 Futamura 사영을 제대로 (proposal 0029)

## 무엇을
컴파일러 생성기(cogen)를 **self-application 없이** 만드는 "cogen approach". 같은 3차 사영을
naive 방식(비대·>150초)이 아니라 hand-written 생성기(0.003초)로.

## 왜 (딥리서치가 확정한 것)
cogen을 "특화기를 자기 자신에 특화"(mix³)로 만들면 **아티팩트가 병리적으로 비대**하다: 자기적용된
특화기가 인터프리터 + universal value 자료형 + 환경/태그 조작을 생성확장마다 끌고 들어간다. 그래서
그 cogen으로 풀 컴파일러를 재도출하면 초소형 인터프리터조차 **>150초**(런타임/규모 무관).
정답(Birkedal&Welinder'94, Thiemann "Cogen in Six Lines"'96, Glück&Jørgensen, Leuschel logen):
**컴파일러 생성기를 BTA/특화기 위의 얇은 층으로 직접 hand-write**한다 — 인터프리터를 안 끌고 오므로
작고 빠르다. pnix는 그 생성기를 이미 가짐(native `poly_specialize`); `pnix_hy.cogen`이 API로 노출.

## 쉽게 말하면 (비유)
```
naive cogen (self-application) = 요리사에게 "요리사 만드는 법을 스스로에게 적용"시켜 만든 매뉴얼 →
                                 매뉴얼마다 요리사 전체(주방·재료·도구)가 통째로 복사됨 → 못 씀
cogen approach (hand-written)  = "레시피 생성 규칙"만 직접 적어둔 얇은 문서 → 작고 빠름
두 방법의 결과 요리(target)는 같다.
```

## 어디에 쓰나
- 인터프리터를 **컴파일러로** 바꾸는 3차 사영을 실용 속도로.
- host 생성확장(`compiler_from_interpreter`) 또는 **이식 가능한 pnix 컴파일러 소스**(`compiler_source`).

## 실측 (이 저장소)
| 경로 | 인터프리터→컴파일러 | 상태 |
|---|---|---|
| naive self-application (`run_cogen`) | **>150초** (초소형조차) | 확장 불가 |
| **cogen approach** (`compiler_from_interpreter`) | **~0.003초** | 실용 |
| standalone pnix 컴파일러 (`compiler_source`) | 6.5KB pnix 소스, 순수 pnix 평가 | 이식 가능 |

## 코드 발췌
```python
import pnix_hy as ph, pnix_hy.pnix_runtime as rt
INT = "let int = prog: env: if prog.tag == \"num\" then prog.value ... ; in int prog input"
compiler = ph.compiler_from_interpreter(INT)          # 인터프리터의 생성확장 = 컴파일러
target   = compiler({"tag":"add","l":{"tag":"arg"},"r":{"tag":"num","value":5}})
assert rt.eval_source("let input = 9; in " + target) == 14
csrc = ph.compiler_source(INT)                        # 이식 가능한 pnix 컴파일러 소스
```

## 한 줄
> 같은 3차 Futamura 사영이라도 **self-application(비대)** 이 아니라 **hand-written 생성기(cogen
> approach)** 로 만들면 실용 속도가 된다 — 4실험 + PE 문헌이 일치.

## 경계 (정직)
- self-application cogen(`run_cogen`/`build_cogen`)은 남겨둠 — 3차 사영의 "생성·실행" 검증용(0026 M5c/M6)
  이자 반례 교보재. 실용 경로는 `pnix_hy.cogen`.
- 근거: `docs/audits/2026-07-02-cogen-stagepoly-research.md`, `docs/proposals/0029-efficient-cogen.md`.
- 정본 평가기(`pnix_runtime`)·4-lane 미러는 무관(추가 아티팩트일 뿐).
