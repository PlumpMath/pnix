# 33. Futamura 사다리 — 하나의 인터프리터에서 셋 (proposal 0026 M7 / 0029)

## 무엇을
하나의 인터프리터에서 세 Futamura 사영을 전부 파생해 보여주는 `futamura_ladder()`(CLI `--futamura`):
1차=해석 붕괴, 2차=컴파일러, 3차=cogen(생성기). 그리고 `compiler_from_interpreter`(효율적 3차, 0029).

## 왜
plain하게는 인터프리터를 짜면 그걸로 끝 — 컴파일러도 cogen도 따로 손으로 짜야 한다. Futamura 사다리는
**같은 인터프리터에서** 컴파일러와 컴파일러-생성기가 파생됨을 보인다(pnix-hy의 중심 결과).

## 세 사영
| 사영 | 무엇 | 결과 |
|---|---|---|
| **1차** | 인터프리터를 프로그램에 특화 | `((input * 3) + 4)` — interpreter-free |
| **2차** | 특화기를 인터프리터에 특화 | 독립 컴파일러, `compiler(prog) = target` |
| **3차** | 특화기를 자기적용(cogen) | 생성기, specializer로 실행: `cogen(a*b, a=6) = (6*b)` |

## 한 줄
> 인터프리터·컴파일러·컴파일러생성기는 별개가 아니라, **하나의 인터프리터에서 특화로 파생**된다 —
> `futamura_ladder()`가 셋을 한 산출물로 보여준다.

## 경계
- 효율적 3차(풀 컴파일러 파생)는 hand-written cogen approach(0029)로; self-application cogen은 검증된
  반례로 남김(examples/20). 정본 평가기·4-lane 미러 무접촉.
