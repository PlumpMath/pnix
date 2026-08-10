# 0026 — 타워 사다리 마일스톤-1 (0013 T3+T2+T5+T6+T8 승격; 명시적 단계화)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, 신규 `pnix_hy/tower.py`(+T6은 기존 투영 재사용).
- 근거: Amin&Rompf POPL'18(λ↑↓ maybe-lift, Pink/Purple/EM), Jones/Gomard/Sestoft(S=L, cogen 자기생성),
  3-Lisp/Jefferson&Friedman(유한 타워 reify/reflect) — 전부 3-0 검증.
- **정직한 단계화**: 본 proposal의 SHIPPED 범위는 **마일스톤-1**(아래)이다. 전체 S=L(전 언어 자기
  적용 specializer), 실제 cogen, stage7 전면 stage-polymorphic 재작성은 **후속 마일스톤**으로 남는다
  — "완전 구현"이라 주장하지 않는다.

## 마일스톤-1 딜리버러블
1. **T5m stage-polymorphic 미니 평가기**: 코어 태그(int/bool/var/binary/if/let)에 대해 **한 코드
   경로**가 maybe-lift 매개로 인터프리터(값)이자 컴파일러(잔여 pnix 소스)로 동작. 1차 사영 검증:
   compile-then-eval == interpret (코퍼스), 잔여코드에 해석 오버헤드 0(평가기 구조물 부재).
2. **T3m mix-in-pnix**: 같은 코어 서브셋의 specializer를 **pnix 프로그램으로**(attrset-인코딩 AST
   위 재귀 함수) 표현 — S=L 전제의 서브셋 실증. host 참조 구현과 결과 동등 판정.
3. **T2h cogen 수용 하니스**: `self_generation_witness(cogen_src, mix_src, apply_fn)` — ⟦cogen⟧(mix)
   == cogen 을 ir-해시 동등 witness로 판정하는 **수용 장치**(실제 cogen은 후속; 하니스는 참/거짓
   토이 케이스로 정합성 검증).
4. **T6 reify/reflect v0**: `reify_computation(source)`(expr-IR + env 스냅샷 + defunctionalized
   continuation v0 + witness) / `reflect_to_stage7(reified, granted)` — `'reflect'` 권한 없으면
   held, 있으면 stage7 인터프리터 레인에서 실행 + host 값과 parity + checkpoint witness.
5. **T8 EM v0**: `em(source, transform, mode)` — **메타(host) 레벨** IR 변환을 interpreted(변환 후
   평가)/compiled(변환 후 잔여 방출) **이중 모드**로 적용(Pink EM+maybe-lift 유사). pnix 비동형
   불변(변환은 메타 레벨, IR은 데이터).

## 마일스톤-2 (SHIPPED 2026-07-02, "0026 마일스톤-2 시작" 지시)

M1 아티팩트를 **확장**(재구현 없음):
1. **T5 M2 — Jones 인터프리터 붕괴**: `_sp`를 재귀 let(공유 env 클로저)·lambda/apply **unfold**·
   정적 attrset/select·전체 이항연산(`rt.apply_binary` 재사용)으로 확장. `collapse_interpreter`:
   pnix로 쓴 미니 인터프리터를 정적 프로그램+동적 `input`에 특화 → 잔여 `((input * 3) + 4)` —
   **인터프리터 흔적 0**(tag 검사/디스패치 부재 단언) + 다입력 parity. unfold 한계로 동적 재귀는
   서브셋 밖 판정(건전).
2. **T3 M2 — mix 객체언어 성장**: `MIX_IN_PNIX` fold에 `==`/`!=`/`<`/`>`/`&&`/`||` + 타입-태그
   결과(`lit`). pnix-mix가 비교·불리언 프로그램을 특화.
3. **T6 M2 — CEK 스테퍼**: defunctionalized continuation 프레임(bin-l/bin-r/if/halt = 데이터)로
   소단계 실행 — **중단 → 상태 내용해시(witness) → 재개** 결정적(같은 지점 = 같은 해시), 최종값
   직접평가와 동등. (Wand&Friedman/Blond의 "meta-continuation을 해시 가능한 artifact로" 실현 v1)
4. **T8 M2 — 평가-중 EM**: `em_stepwise` — 매 스텝 **메타 레벨** 변환이 현재 control 식(IR 데이터)
   을 재작성(rewrite 카운트+witness). 객체 프로그램은 자신을 조작하지 않음(비동형 불변).

**마일스톤-3 (SHIPPED 2026-07-02, "시작~" 지시)**:
1. **M3a — MIX 객체언어의 S=L 폐포 방향 성장**: `MIX_IN_PNIX`가 `let`(재귀; env2 자기참조를 pnix
   laziness로)·`lambda/closure`·`apply`(maybe-lift 언폴딩)·`select`·`attrset`·`const`를 객체언어로
   처리 — **specializer가 자기 자신이 쓰는 구조 대부분을 다룸**. senv는 folded-노드 사상으로 승격
   (M1/M2 하위호환은 `_wrap_node` 래퍼로 유지, 회귀 0).
2. **M3b — pnix 안에서의 1차 Futamura 사영**: pnix-표현 mix가 pnix-표현 인터프리터를 정적
   프로그램에 특화 → 잔여 `((input * 3) + 4)`, 해석 계층 완전 소거, 다입력 parity. (M2에선 host가
   수행 — 이제 **pnix가 pnix를 접는다**.)
3. **M3c — offline BTA v1** (`binding_time_analysis`): 노드별 S/D 분할(monovariant, 적용-지점
   memo+fixpoint, 재귀는 in-progress 가드) — **if-조건 전부 S ⟺ 잔여에 디스패치 0**을 실제 붕괴
   결과와 교차검증. **연구 기록**: naive 2차 사영은 "재귀 클로저가 동적 env를 포획 → 자기참조
   코드 무한 방출"로 막힘 — 정확히 1985 Mix가 offline BTA+polyvariant specialization으로 푼 지점.
   BTA v1이 그 전제를 실장한 것.

**마일스톤-4 (SHIPPED 2026-07-02, "next~" 지시)**:
1. **M4a — polyvariant specialization** (`poly_specialize`/`_ps`): M3의 벽을 1985-Mix 기법으로 돌파 —
   (함수 본문, 정적-시그니처)마다 **이름 붙은 specialization point**(잔여 함수 정의)를 만들고 재귀
   호출은 그 이름 호출로 방출; 자기참조 동적 let은 **재귀 잔여 let**으로 방출(pnix let은 재귀라 합법);
   커리드 함수는 eta-확장; `&&`/`||` 단락 특례; builtins(head/tail/listToAttrs/hasAttr/getAttr/isX)
   정적 폴딩/동적 방출. 데모: 동적 인자 재귀 `f x` → `let __s1 = __a2: ...(__s1 (__a2 - 1)); in (__s1 x)`
   (naive는 unfold 한계 초과), 정적 인자는 여전히 `4`로 완전 폴딩.
2. **M4b — 실제 2차 Futamura 사영**: `compiler = ⟦S_host⟧(MIX_pnix, INT)` — 호스트 polyvariant
   특화기가 **pnix-표현 specializer를 인터프리터에 특화** → 16 spec-point, ~16KB 잔여 = stand-alone
   **컴파일러**. 검증: compiler(P1) → `((input * 3) + 4)`, compiler(P2) → `((input + 1) * 10)` —
   프로그램 인코딩→완전 접힌 target 번역, 전 입력 parity. (외부=host, 내부=pnix인 2-특화기 변형임을
   명시 — 고전 정식화의 인정된 변형.)

**마일스톤-5a (SHIPPED 2026-07-02, "다음~" 지시) — 외부 특화기의 S=L(core)**:
`POLY_MIX_IN_PNIX` — **polyvariant specializer를 pnix로 표현**. pnix는 순수라 spec-point memo를
**state-passing**으로 스레딩(모든 재귀 호출이 `{ n = 노드; st = { specs; ctr; }; }` 반환); spec 검색은
구조 동등(`==`) 키 — 클로저는 **body만** 시그니처에 기여(자기참조 env가 비교에 못 들어와 무한 회피);
pending-seed 후 결과로 patch(1985 Mix의 pending list를 순수식으로). 검증: 동적 재귀
`let f = n: ...; in f x` → pnix가 `let __s0 = __a1: (if (__a1 == 0) then 0 else (1 + (__s0 (__a1 - 1))));
in (__s0 x)` 방출 — host `_ps`와 의미 동일(다입력 parity), 정적 재귀는 `4`로 완전 폴딩.
`_decode_full`(lambda/apply/let/select/attrset 디코딩) 추가.

**마일스톤-5b (SHIPPED 2026-07-02) — pnix 단독 2차 Futamura 사영**:
`POLY_MIX_IN_PNIX`의 객체언어를 자기 자신이 쓰는 구조까지 성장: list/`++`/`//`, builtins(head/tail/
listToAttrs/hasAttr/getAttr/isX/attrNames/toString/length/seq/map — 정적 폴딩·동적 방출, 동적 name은
guard), attrset-투과 select, 자기참조 잔여 `let`, 커리드 spec의 eta-확장. state는 `builtins.seq`로
strict 스레딩, 깊은 force 체인은 512MB 스택 워커 스레드(`_eval_deep`)에서. 검증: **컴파일러를 pnix가
단독 생성** — `poly_mix_in_pnix(MIX ⊕ INT)` → 17 spec-point 잔여, compiler(P1)=`((input*3)+4)`,
compiler(P2)=`((input+1)*10)`, 전 입력 parity. (M4b는 외부 host 특화기, M5b는 **외부·내부 모두 pnix**.)

**마일스톤-5c (SHIPPED 2026-07-02) — cogen(3차 사영) self-application이 이제 완주**:
초기엔 self-application이 "cannot residualize a closure"(고차 특화기의 고전 난제)로 막혔다. **closure
conversion(lambda lifting)** 을 `_ps`에 추가(`_force_code`: 잔여로 필요한 클로저를 파라미터 symbolic로
특화해 `(param: body)`로 방출) + `null`/`builtin`/`None` literal 지원. 결과: `poly_specialize`가 **자기
자신(POLY_MIX)을 특화**해 완주 → **21 spec-point, 22.5KB의 cogen(polyvariant specializer의 생성확장)
아티팩트를 생성**. `tower_ladder_report.m5c_cogen_produced` = cogen이 종료+well-formed pnix+T2h 하니스
수용. 벽은 "생성 불가"→"**생성됨, 실행 비현실적**"으로 이동: 생성된 cogen을 인터프리터에 **실행**해
컴파일러를 뽑는 단계는 호스트 tree-walker의 깊이/성능으로 >10분(비현실적). unblock = trampolined/
iterative force 또는 stage7-레인 실행. (closure conversion은 M4/M5b의 잔여 능력도 강화 — 커리드 잔여가
이제 정식으로 방출됨.)

**마일스톤-6 (SHIPPED 2026-07-02) — cogen 아티팩트가 실행됨(specializer로 동작 검증)**:
`build_cogen`(자기적용으로 cogen 생성, 캐시) + `run_cogen(prog, static_env)`(cogen 아티팩트를 **실행**).
검증: 생성된 cogen이 **poly_mix_in_pnix와 동일하게** 동작 — `2*3+4`→`10`(정적 폴딩), `a+1`(a=41)→`42`,
`a*b`(a=6)→`(6 * b)`(동적 잔여), 그리고 `run_cogen(...) == poly_mix_in_pnix(...)`. 즉 M5c의 "cogen이
생성됨"을 넘어 **"cogen이 실행되어 specializer로서 정확히 계산함"**(core 서브셋)까지 검증. 남은 성능 벽:
cogen을 인터프리터에 실행해 **풀 컴파일러**를 재도출하는 큰 과제는 호스트 tree-walker 성능 한계(>10분)
— compiled runtime이나 stage7-레인 실행이면 사라짐(개념 아닌 성능).

**마일스톤-7 (SHIPPED 2026-07-02) — 사다리 통합 capstone `futamura_ladder`/`--futamura`**:
기존 rung만 재사용(신규 기계 없음)해 1·2·3차 사영을 **하나의 검사 가능한 산출물**로: 1차=인터프리터
붕괴(`((input*3)+4)`, interpreter-free), 2차=specializer를 인터프리터에 특화(17 spec-point 컴파일러,
compiler(prog)=target), 3차=자기적용 cogen(22.5KB) **실행**(`cogen(a*b|a=6)=(6*b)`). CLI `--futamura`.

**원칙적 종점 — 남은 둘은 정확성이 아님**:
- **cogen으로 풀 컴파일러 재도출**: 순수 성능. 22.5KB cogen 잔여를 큰 객체(인터프리터, 77KB AST)에
  tree-walker로 돌리면 native `_ps`(1.8s)의 ~50배(축소판도 >90s). **compiled runtime**이면 사라짐 —
  개념 완료, 성능만 남음.
- **stage7 evaluator의 stage-polymorphic 재작성**: `pnix_runtime`/4-lane **SACRED**. SCOPE_LOCK §4상
  명시적 human 경계 결정 없이는 **진행 금지** — "다음~"으로 자동 진행하지 않음.

0026 타워 사다리는 이로써 **허용 scope 내 종결**(M1–M7). 위 둘은 별도 결정(compiled-runtime 프로젝트
또는 sacred 경계 승인)이 있어야 진행.

## 수용: 신규 `tower_ladder_report` 등록(+1) — 5개 마일스톤 각각 판정(T6는 Hy 필요, 부재 시
available:False). SCOPE_LOCK: pnix-side 매크로 금지 유지(T8은 메타 레벨).
