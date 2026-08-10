# 0013 — meta-circular / interop 확장 후보 카탈로그 (deep-research 2026-07-02)

- 상태: **candidate catalog** (후보 — 수락·구현 안 됨). SCOPE_LOCK §7: 사람이 고르면 개별 `NNNN-*.md`로 승격.
- 근거: 3중 파이프라인 — (a) 웹 딥리서치 107 agents(1차 출처 검증 23 claims, 3표 적대 투표),
  (b) 앵글 3–5 후속 검증 66 agents(21/22 확정), (c) 코드 감사 84 agents(체크리스트 24절 ↔ 저장소
  대조; 진짜 미구현 5, 금지 16 차단). 확정 버그 26건은 proposal이 아니라 **수정 대상** —
  `docs/audits/2026-07-02-deep-research-audit.md` 참조.
- 원칙 유지: pnix 비동형(동형화 금지), sacred 무접촉, 새 evaluator/mirror/gate 금지, additive only.
- 표기: ⟦p⟧ = 프로그램 p의 의미(실행). Futamura: 1차 target=⟦mix⟧(int,src) · 2차 compiler=⟦mix⟧(mix,int) · 3차 cogen=⟦mix⟧(mix,mix).

## T. 타워/Futamura 사다리 (딥리서치 앵글 1–2; 전부 1차 출처 3-0 검증)

| # | 후보 | 레인 | 비용 | 내용 |
|---|---|---|---|---|
| T1 | **Jones-optimality 수용 게이트** | pnix+host | 중 | Pink(Amin&Rompf POPL'18)의 검사 가능한 성질: compile(source(p)) == canonical(p) **해시 동등**을 545 코퍼스 전체에 게이트로 + n-단 자기해석 붕괴 검사. 기존 B==C 고정점 증명의 직접 확장(모든 프로그램·모든 타워 높이로 정량). |
| T2 | **3차 사영 고정점 witness** | pnix | 중 | cogen 자기생성 ⟦cogen⟧(mix)=cogen (Jones/Gomard/Sestoft §1.5.3, Glück PEPM'09) — 3차 사영 시도 시 수용 기준을 **내용해시 동등 witness**로. 스키마는 기존 §14 그대로. |
| T3 | **2차 사영 전제: S=L** | pnix | 대 | specialize_pnix를 **pnix 서브셋으로 재표현**(자기적용 가능해야 compiler=⟦mix⟧(mix,int)). 문헌: 고전 offline PE는 단당 ~9-10×, TDPE는 34-200%뿐+3차 비실용 → **canonical IR 위 offline 스타일**로 갈 것. (주의: 순수·지연 언어에서의 자기적용은 미답 — open question) |
| T4 | **Truffle식 PE 어노테이션** | host | 중 | 무유도 전체 PE는 폭발/저성능(PLDI'17 검증) — stage7 인터프리터에 PEBoundary/PEFinal/Assumption 아날로그 마커를 달아 specialize_pnix를 선택적·가정부 특화로 강화. |
| T5 | **stage-polymorphic 평가기(타워 붕괴)** | pnix | 특대 | maybe-lift 매개 하나로 인터프리터/컴파일러가 **한 아티팩트**가 되는 λ↑↓ 스타일 재작성 — 4-lane 미러의 쌍들을 통합하고 one-pass 붕괴. 검증됐지만 이상화 세팅(재작성 필요, retrofit 불가) — 장기 연구 트랙. |
| T6 | **reify/reflect 레벨 이동** | 경계 | 대 | pnix-lane 계산을 (expr, env, kont) 삼중으로 물화해 stage7 인터프리터 레벨로 승격(3-Lisp/Jefferson&Friedman 유한 타워) — capability 게이트 + checkpoint witness 필수. meta-continuation은 **defunctionalize해서 해시 가능한 witness**로(Wand&Friedman/Blond). |
| T7 | **의미 변경 시 재특화** | pnix | 중 | Purple: 수정된 의미 아래 재컴파일 — drift 분류기/action 층이 메타 변경을 감지하면 영향받은 잔여코드를 **재특화 트리거**(영구 해석 폴백 대신). |
| T8 | **EM(execute-at-meta) 메타레벨 매크로** | 경계 | 대 | 매크로류 능력을 **pnix가 아닌 메타(Hy) 레벨에 상주**시키는 Pink의 EM+maybe-lift — 비동형 원칙 보존. 단 Pink는 동형 언어라 외삽(미증명) 주의. |

## I. interop 강화 (앵글 4; 10/10 확정 — 사용자 요청 핵심)

| # | 후보 | 대응 기존 심볼 | 내용 |
|---|---|---|---|
| I1 | **런타임 회수 가능 capability** | gate.gate_check, interop.check_capability | SES: 기본 권한 0 + 명시 endow + **attenuate/revoke/suspend를 런타임에** — 현재 정적 grant에 없는 능력. opaque-ref/host-call 권한을 회수 가능 핸들로. |
| I2 | **own/borrow 핸들 + lend 카운터** | opaque_lifecycle | Wasm Canonical ABI: 핸들에 own/borrow 비트 + num_lends; **대여 미반환 상태의 drop은 trap**; borrow는 call-scoped. 현재 카운터(total/released)의 정확한 상위 모델. |
| I3 | **Context-수명 opaque ref** | _OPAQUE 전역 레지스트리 | GraalVM polyglot: 값은 생성 Context에 수명 결박 — close 후 접근은 명확한 오류. eval-컨텍스트 단위 opaque 스코프(현재는 프로세스 전역 영생). |
| I4 | **blame 방향 판정** | InteropError, try_call_host | blame calculus(Wadler&Findler): 경계 캐스트에 blame 라벨 — 실패 시 **호스트/게스트 어느 쪽 책임인지**(positive/negative) 판정해 InteropError에 blame 필드. "정밀→비정밀 캐스트는 positive blame 불가" 정리 = 검사 가능한 불변식. |
| I5 | **harden 표면-witness** | make_opaque_ref | SES harden(): 노출 host 객체의 표면(속성/프로토타입 전이폐포)을 동결+**표면 해시 witness** — 호출 시마다 재검증해 변조 감지(순수성 경계 강화). |
| I6 | **신뢰 프록시 불변식** | opaque_allowed_methods | Trustworthy Proxies(ECOOP'13): 불변식을 래퍼 신뢰가 아니라 **기질(런타임)이 강제** — opaque-ref 래퍼가 위반 불가능하게 invariant 검사를 호출 경로에 내장. |
| I7 | **fitsIn* 무손실 술어** | to_host/from_host loss 마킹 | GraalVM: 변환 전 fitsInByte/Int/BigInteger 술어 검사 → 수치 경계 변환의 손실을 **사전 판정**(현재 big-int→float 등 암묵). A-계열 loss 마킹의 자연 확장. |
| I8 | **compartment식 게스트 네임스페이스** | repl ctx, eval env | SES Compartment: eval 단위로 own globalThis+훅 로더, **동결 intrinsic(builtins)은 공유** — 다중 세션/모듈 격리를 복제 없이. |

## P. phase/hygiene/패스 물화 (앵글 3; 5/5 확정 — 비동형 원칙 하 toolkit 레벨)

| # | 후보 | 내용 |
|---|---|---|
| P1 | **sets-of-scopes hygiene 검사** | Flatt POPL'16: 위생 해석을 **집합 연산으로 검사** — hy_macro_over_pnix 브리지에 의도치 않은 포획 검출 리포트(감사가 찾은 진짜 gap "hygiene self-check 부재"를 정확히 메움). |
| P2 | **phase 정수 산술 추적** | Racket: for-syntax +1 / for-template −1, 합성·상쇄 — 투영 단계(read/expand/eval/collapse)에 정수 phase 라벨 + 합성 법칙 검사. |
| P3 | **nanopass식 IR 패스 물화** | define-language + extends 델타(ICFP'13, Chez 상용 검증 15-27% 개선): lower_to_ir 파이프라인을 **언어 델타 시퀀스로 물화** + 패스별 불변식 검사. 감사가 찾은 pnix-ir-diff와 결합 시 시너지. |
| P4 | **컴파일-실행 관측 무관성 게이트** | Flatt macromod: 컴파일이 빈 store에서 시작(컴파일타임 상태가 런타임에 누출 불가) — 4-lane parity에 **분리 증명 축** 추가(hy-meta stage 격리 검사의 pnix-hy 리포트화). |

## R. 재현성/증명 (앵글 5; 6/7 확정)

| # | 후보 | 내용 |
|---|---|---|
| R1 | **정의-단위 내용주소** | Unison: 정의별 구문트리 해시(의존성은 해시로 치환), **이름은 메타데이터**(rename≠재컴파일) — cached_eval을 전체-식에서 정의 단위로 세분화. |
| R2 | **해시-키 검사 캐시** | Unison: 결정적·무I/O 테스트는 의존성 해시 불변이면 재실행 불필요 — `--check`/`--gate` 리포트를 입력 내용해시로 키잉해 무변경 시 스킵(로컬 CI 대폭 단축). |
| R3 | **resolved-derivation 조기중단** | Nix CA: 입력 해시로 동일성 증명 시 리빌드 중단 + **Realisation 매핑**(drv output id→산출 경로) — witness 저장을 ir_hash→value_hash 매핑 스토어로 승격해 조기중단. |
| R4 | **predicate-typed witness 증명** | in-toto/SLSA: witness에 **버전드 predicate type URI**(스키마 진화는 URI 개명·deprecate로) — §14 envelope과 정합하는 확장(경계 영향: proposal+drift-guard 필수). |
| R5 | **scheduler×rebuilder 분류 적용** | Build Systems à la Carte: 캐시를 verifying/constructive trace로 분류하고 suspending scheduler+constructive trace 지점("Cloud Shake")을 아티팩트 캐시 설계 기준으로. |

## G. 저장소 감사가 찾은 진짜 미구현 (반박 검증 통과 5건)

| # | 항목 | 가치 |
|---|---|---|
| G1 | **pnix-ir-diff** — 두 IR/AST의 노드 수준 구조 diff(현재 해시 boolean 동등뿐) | med — P3와 결합 권장 |
| G2 | form_sha256 — read-단계(Hy form/model) 해시가 artifact 레코드에 부재 | low |
| G3 | cache_key에 entrypoint/dependency-hash 필드 부재 | low — R1/R3와 결합 |
| G4 | pnix-hy 툴킷에 hygiene/symbol-capture self-check 리포트 부재 | low — P1이 정확히 해결 |
| G5 | per-변수 env-snapshot diff(현재 해시 단위 감지뿐) | low |

## 기각/금지 (다시 열지 말 것)

- **기각(검증 실패)**: diffoscope 포맷 주장(과잉주장 0/3), "principal architecture=무한타워" 서술(0-3),
  "offline BTA 필수" 강주장(0-3 — online 자기적용 PE·수제 cogen 존재; offline은 증명된 길일 뿐).
- **금지(SCOPE_LOCK §3/§4, 감사 재확인 16건)**: pnix-side quasiquote/unquote/splice/defmacro/reader-macro
  (비동형 의도 — T8 EM은 **메타레벨**이라 별개), derivation store-hashing, #_pnix-gap 채우기, 공유 witness
  스키마 무단 확장(R4는 proposal+양레인+drift-guard 경로로만), stage16 fail-closed 재해석 등 —
  전체 목록은 감사 원장 §C.

## 우선순위 제안 (사람 결정용)

1. **버그 26건 수정이 최우선** (proposal 불요; 특히 high 6: specialize_pnix 의미 오류 2, stage7 worker desync, deployment 거짓 ready, host_callable arity, ci-local 오거부).
2. 싸고 확실: **T1**(Jones 게이트) · **I7**(fitsIn 술어) · **I2**(own/borrow) · **P1+G4**(hygiene 리포트) · **G1+P3**(ir-diff+패스 물화) · **R2**(해시-키 검사 캐시).
3. 중기: I1(회수 가능 capability) · I4(blame) · I3(context-수명 opaque) · T4(PE 어노테이션) · T7(재특화) · R3(조기중단).
4. 장기 연구: T3(S=L)→T2(3차 고정점) · T5(stage-polymorphic 붕괴) · T6(reify/reflect) · T8(EM).

출처(전부 1차): Amin&Rompf POPL'18 + artifact repo · Jones/Gomard/Sestoft 1993 · Grobauer&Yang BRICS-RS-99-40 ·
Würthinger+ PLDI'17 · Wand&Friedman LFP'86 · Danvy&Malmkjær(Blond) LFP'88 · Jefferson&Friedman LASC'96 ·
endojs/endo SES README · Van Cutsem&Miller ECOOP'13 · Wadler&Findler(blame) · WebAssembly CanonicalABI.md ·
GraalVM polyglot Value API · Flatt POPL'16(scope sets)/macromod/GPCE'13 · Keep&Dybvig ICFP'13(nanopass) ·
unison-lang.org · tweag Nix CA · in-toto/SLSA provenance · Mokhov+ (Build Systems à la Carte).
