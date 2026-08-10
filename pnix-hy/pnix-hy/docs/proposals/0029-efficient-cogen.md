# 0029 — efficient cogen (3rd Futamura projection done RIGHT, the "cogen approach")

- 상태: **SHIPPED 2026-07-02** ("yes 다음~"). 근거: `docs/audits/2026-07-02-cogen-stagepoly-research.md`
  (딥리서치 24 confirmed). 0028 P2("cogen→풀 컴파일러")의 연구-블록을 해소.
- Scope: **추가 모듈** `pnix_hy/cogen.py`. host/pnix specializer lane만. `pnix_runtime`/stage7/
  4-lane **SACRED 무접촉**(새 아티팩트일 뿐 정본 평가기 미수정).

## 문제
0026 M5c/M6의 `build_cogen`/`run_cogen`는 **self-application**으로 cogen을 만든다. 4실험 + 딥리서치가
확정: self-applied cogen은 **병리적으로 비대**(specializer가 인터프리터+universal datatype+env/tag를
생성확장에 끌고 들어감) → num-only 인터프리터조차 실행 >150s. 런타임 문제가 아니라 **아티팩트 자체**.

## 해법 (리서치 만장일치: "cogen approach")
self-application을 **하지 말고**, 컴파일러 생성기를 **BTA/특화기의 얇은 층으로 직접 hand-write**한다
(인터프리터 없음, 구문트리만 조작). *Birkedal&Welinder'94, Thiemann'96, Glück&Jørgensen, Leuschel logen.*
**pnix는 그 hand-written 생성기를 이미 가짐**: native 폴리variant 특화기 `tower.poly_specialize`. 0029는
그것을 "cogen approach" API로 노출한다:

- `generating_extension(source, dynamic_vars)` → **프로그램별 생성확장** = 재사용 callable
  `gex(static_env) -> residual` (self-application 없이 native 특화기로 특화). 문헌의 `cog p`.
- `compiler_from_interpreter(interp)` → **인터프리터의 생성확장 = 컴파일러**: `compiler(program) -> target`.
- `cogen(source, dynamic_vars)` → 생성확장 + 메타. `tower.build_cogen`(self-application)은 이제
  docstring에 "비효율 경로"로 명시, 올바른 경로는 `pnix_hy.cogen`로 안내.

## 결과 (실측)
- **컴파일러를 인터프리터에서 생성 + 프로그램 2개 컴파일: 0.003s** (self-applied `run_cogen`은 >150s).
- `compiler(P1)`=`((input*3)+4)`류 잔여, `compiler(P2)` 전부 parity. 일반 생성확장도 직접 특화와 동일.
- `cogen_report`(`--check` +1): 일반 생성확장 == 직접특화 + 인터프리터→컴파일러가 **예산 내(<30s)** +
  전 입력 parity.

## 수용 기준 (충족)
- `generating_extension`/`compiler_from_interpreter`가 native 특화기와 동일 결과, self-application 미사용.
- 인터프리터→컴파일러가 실용 예산 내(측정 ~0.003s) → **0028 M6 성능 프론티어 해소**.
- `pnix_runtime`/stage7/4-lane 무변경, `--check`/`--gate` 회귀 0.

## P2 — SHIPPED (standalone pnix 컴파일러 소스)
- `compiler_source(interp)` → 인터프리터의 생성확장을 **이식 가능한 pnix 소스**(host 클로저 아님)로 방출
  (`tower.poly_mix_in_pnix`=M5b 재사용, 중복 없음). `compile_with(csrc, prog)` → 순수 pnix 평가로 target.
  실측: `compiler_source(INT)`=6.5KB pnix 컴파일러, `compile_with`(P1/P2) 전부 parity. `cogen_report`에
  `standalone_pnix_compiler` 추가.

## 남은 것 (선택)
- pure-lazy에서 생성확장 크기 특성(A7: thunk는 BTA-dynamic 잔여화) 실측 — 연구 성격.

## Forbidden (지킴)
- self-application을 "올바른 cogen"이라 주장하지 않음. `pnix_runtime`/stage7/4-lane 수정. 정본 평가기 대체.
