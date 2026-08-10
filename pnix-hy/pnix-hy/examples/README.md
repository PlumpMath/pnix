# examples — 왜 meta-circular인가 (plain Python/Hy/pnix의 한계 vs pnix-hy/hy-meta)

> **Foundation entry point:** [FOUNDATION_PATH.md](FOUNDATION_PATH.md) and
> [`00-foundation`](00-foundation/README.md). Basic PNIX evaluation and the
> meta-circular compiler/evaluator mechanism are available without importing
> proof, action, deployment, or admission policy. Those are separate explicit
> verification/research surfaces.

The numbered catalog below predates this sharper boundary. Its proof and
service-policy examples remain examples, but they do not define basic runtime
outcomes and cannot turn language failures into generic `Held` results.

이 폴더는 **사람이 직접 코드를 보고 "이 기능을 어디에 쓸지" 판단**하도록 만든 예제 모음입니다.
각 섹션은 한 가지 meta-circular 능력을 다루고, **두 파일을 나란히** 둡니다:

- `limit_*.py` / `limit_*.hy` / `limit_*.px` — **plain Python/Hy/pnix의 한계**: 그 언어를 "그냥"
  쓰면 왜 안 되는지 / 무엇이 불가능하거나 위험한지.
- `pnix_hy_way.py` — **pnix-hy / hy-meta로 같은 문제를 어떻게 해결**하는지.

모든 파일에 **한글 주석**이 있고, 각 섹션 `README.md`가 "무엇을 / 왜 / 어디에 쓰나 / 쉽게 말하면(비유)"을 정리합니다.

## 쉽게 말하면

```text
plain Python
= 실행은 되지만, 안전/증거/의미보존/재현성은 직접 다 챙겨야 함

pnix-hy / hy-meta
= 실행하면서 "왜 안전한가, 무엇을 했나, 같은 의미인가, 증거가 뭔가"를 같이 남김
```

Python 초급자에게 `eval("1+2")`는 결과 `3`만 준다. pnix-hy 방식은 결과에 더해:

```text
결과는 3이다 · 이 코드는 순수하다 · 파일/네트워크/서브프로세스를 안 썼다
AST/IR/hash는 이렇다 · 다시 돌려도 같다 · 번역해도 같은 의미다
문제가 있으면 어느 줄 어느 칸에서 막혔다
```
까지 같이 준다. → 이 폴더는 **"그냥 실행"과 "증거·안전·의미보존·재현성까지 포함해 다루기"의 차이**를 실행되는 70개 예제로 보여준다.

## 핵심 대비 (한 줄 요약)

| 섹션 | plain의 한계 | pnix-hy / hy-meta |
|---|---|---|
| `01-pure-sandbox` | `eval()`은 부작용·무한루프·자원소모를 막지 못함 | 순수성 정적판정 + 자원한계 + 게이트로 **신뢰 가능한 샌드박스** |
| `02-determinism-and-drift` | `hash()`는 실행마다 바뀌고, 코드의 정본 해시가 없음 | 내용주소 해시 + **결정성/drift 분류** |
| `03-specialization-futamura` | 부분입력에 특화된 잔여 프로그램을 만들 수 없음 | `specialize_pnix` = **Futamura 1차 사영(잔여 코드 생성)** |
| `04-host-interop-loss-effect` | 언어 간 값 변환의 손실/부작용/권한이 **무표시** | `to_host`/`from_host`가 loss/effect/capability를 **명시 기록** |
| `05-witness-and-gate` | eval은 증거를 남기지 않고 권한 제어가 없음 | 내용해시 **witness** + effect **capability gate** |
| `06-ir-and-roundtrip` | Python AST는 정본 아님·안정 해시 없음 | 정본 **IR**(해시 안정, IR로 직접 평가) + 의미보존 roundtrip |
| `07-hy-macro-over-pnix` † | Python엔 매크로 없음·pnix는 비동형 | Hy 매크로/quasiquote를 **pnix 코드/값 위에** 적용 |
| `08-hy-reader-embed-pnix` † | 다른 언어는 read-time에 그냥 문자열 | Hy `#px` reader macro로 pnix를 **read-time 임베드** |
| `09-mirror-and-reify` | ast/dis/inspect를 따로 꿰매야 함 | `reify_pnix` = 한 폼을 **모든 단면 통일 물화** |
| `10-repl-context` | 반복 CLI는 stateless·재기동 | **warm REPL**로 컨텍스트 유지(누적 env) |
| `11-self-hosting-convergence` † | 자기 언어 구현과의 수렴 증명 없음 | 한 폼을 **4 substrate에서 평가·수렴**(자기호스팅) |
| `12-content-addressed-cache` | `lru_cache`는 표현 기반(포맷 다르면 미스) | `cached_eval` = **정본 내용주소** 캐시 |
| `13-structured-diagnostics` | 오류가 traceback(직접 파싱) | `diagnose` = line/column/phase/**캐럿** 구조화 진단 |
| `14-pnix-import-hook` † | `.px` import 불가·sys.modules 롤백 없음 | `.px` 모듈 로딩 + **스냅샷/롤백 트랜잭션** |
| `15-host-introspection-mirror` † | 내성이 호스트 관점 하나뿐 | host-direct vs **stage7 내성 일치(parity)** |
| `16-meaning-preservation-roundtrip` † | 번역의 의미보존 증명·상태어휘 없음 | `meaning_preserved` + 상태어휘(lossless/lossy-ok/held/rejected) |
| `17-unified-explain` | 값·순수성·안전성·진단을 따로 조립 | `explain_pnix` = 한 번에 통합 설명 |
| `18-action-checkpoint` | 값만 있고 action 승인 verdict·증거·rollback ref가 없음 | `check_action`/`verify_action` = accepted/held/rejected 판정표 |
| `19-compiled-runtime` | tree-walker는 매 노드 재-dispatch(hot 재귀 느림)+호스트 재귀한계 | `compiled_eval` = core-subset를 Python 클로저로 컴파일(정본 동등·fib ~8x) |
| `20-efficient-cogen` | naive cogen(self-application)은 비대해 인터프리터→풀 컴파일러 >150초 | `cogen` = hand-written 생성기(cogen approach), 컴파일러 ~0.003초 + 이식 pnix 소스 |
| `21-specializer-optimizations` | naive 부분평가는 sharing 손실+문맥 갇힌 정적계산으로 잔여 부풀림 | `poly_specialize` BTI 계열(sharing/eta/let-insertion/commuting/bounded) |
| `22-incremental-evaluation` | 이름 기반 캐시는 alpha-rename에 무효화+부분 재사용 없음 | `incremental_eval` = 의존성-치환 content hash(정의별 재사용·rename 면역)+realisation cutoff |
| `23-capability-attenuation` | Python 객체를 넘기면 전권—감쇠도 회수도 불가 | `CapabilityHandle` = grant→attenuate→suspend/resume→revoke(SES 최소권한) |
| `24-phase-separation` | plain eval엔 컴파일/실행 단계 구분·순수성 보장 없음 | `phase_of`/`phase_separation_report` = phase 대수(±1)+lowering 관측적 분리 |
| `25-typed-attestation` | 형식 없는 witness는 무엇에 대한·유효한지 판별 불가 | `typed_witness` = predicate 타입 URI 부여+유효성/이관(in-toto/SLSA) |
| `26-jones-optimality` | 특화기가 해석 계층을 정말 없앴는지 검증할 척도 없음 | `jones_optimality_report` = 533-코퍼스 특화-왕복 IR 불변 |
| `27-macro-hygiene` | 순진한 매크로/치환은 변수를 실수로 포획 | `hygiene_report` = 심어둔 충돌에서 capture 탐지+fresh binder 청결 |
| `28-numeric-boundary` | Python은 숫자 경계 정밀도 손실을 조용히 넘김 | `numeric_fits(value, kind)` = 변환 전에 무손실 판정(GraalVM fitsIn*) |
| `29-ir-diff` | 텍스트 diff는 포맷 잡음에 속고 변화 위치 못 짚음 | `ir_diff` = 정규화 IR 구조 비교, 발산을 AST 경로로 지목 |
| `30-verifying-cache` | 검사를 매번 전부 재계산, 상태 증거 없음 | `cached_run`/`package_state_hash` = 상태-해시 키 replay+자동 무효화 |
| `31-compartment-isolation` | Python eval은 전역 공유—컨텍스트 격리 없음 | `Compartment` = 이름·모듈 격리(SES), back-leak 없음 |
| `32-assumed-specialization` | 순진한 특화는 가정 기록·drift 재특화 불가 | `specialize_pnix(assumptions=)`+`respecialize_if_drifted`(투기적+guard) |
| `33-futamura-ladder` | 인터프리터 하나로 컴파일러·cogen 파생 불가 | `futamura_ladder`/`--futamura` = 1·2·3차 사영을 한 산출물로 |
| `34-module-distribution` | 평범한 패키지는 자기 위치·증명 레인 못 찾음 | `deployment_info` = 자기 발견 레이어링+능력 티어(core/projection/full) |
| `35-staging-tower-internals` | plain eval은 통째로—멈춤·reify·재개·정적/동적 분류 없음 | CEK 기계(pause/reify/resume)+stage-poly(interpret/compile)+offline BTA |

† = Hy 1.3.0 proof Python 필요 (`nix develop` 또는 `PNIX_HY_PYTHON` 설정 후 실행).
그 외 섹션(01–06, 09, 10, 12, 13, 17, 18)은 의존성 없이 바로 실행됩니다.

## 실행법

순수(Python) 예제는 의존성 없이 바로:

```sh
# 저장소 어디서나 (스크립트가 pnix-hy/를 sys.path에 넣음)
python pnix-hy/examples/01-pure-sandbox/pnix_hy_way.py
python pnix-hy/examples/01-pure-sandbox/limit_python.py
```

또는 flake 개발셸에서:

```sh
nix develop          # python + hy + pnix-hy-project on PATH
python pnix-hy/examples/03-specialization-futamura/pnix_hy_way.py
```

Hy가 필요한(프로젝션/mirror/hy-meta) 예제는 저장소 루트에서 `PNIX_HY_PYTHON`(=Hy 1.3.0)이
설정된 상태로 실행합니다 — `nix develop` 안이면 자동입니다.

## 한 줄 결론

pnix-hy/hy-meta가 정말 meta-circular 기능을 갖는지 **말이 아니라 실행되는 70개 예제로** 보여주는
데모 세트다 — "코드를 실행하는 것"을 넘어 "증거·안전·의미보존·재현성까지 포함해 다루는 것".

> 상위 개요/CLI/REPL은 저장소 루트 `README.md`, 능력 전체 목록은 `../docs/` 와 `../todo.md` 참고.
