# pnix-clj PLANS

목적: 아직 확정 안 된 미래 설계 방향 — 방향 제시용 메모이지 착수가 결정된
작업이 아니다. 지금 당장 집어서 진행할 수 있는 작업은
[`TODO.md`](TODO.md)에 둔다.

## F7b — pnix self-applicable specializer (HELD, open research)

**call-by-need self-applicable partial evaluator 선례가 문헌에 없다.**
고전적 self-applicable PE(flow-chart mix, Scheme0, lambda-mix, Logimix,
Similix)는 전부 STRICT 언어 대상이다. Similix가 non-strict *인터프리터*를
specialize하긴 하지만 specializer 자체는 strict이고 laziness는 object-level
suspension(공유 없는 call-by-name)이라 call-by-need가 아니다. Nix-like
언어용 F7b는 엔지니어링이 아니라 NEW research다.

검증된 lazy-언어 경로는 이미 갖고 있다: Bondorf 1990의 `delay` memoization
문제를 통과하는 길은 lazy-언어 인터프리터 위 STRICT-host specializer —
정확히 pnix-clj의 Clojure specializer 형태다(Jørgensen POPL'92,
commercial-compiler-speed lazy-language compilers).

성공해도 한정된 이득이다(Glück PEPM'09 §5.2): `spec(spec,spec)` mismatch는
오류를 증명하지만 동의는 아무것도 증명하지 않는다 — 기계 검사 가능한
**부분** 정확성 테스트일 뿐, 결코 증명이 아니다.

소유자가 언젠가 green-light하면: Jones/Gomard/Sestoft §7.4의 11-step
레시피 — pnix를 bare-bones core로 자르기(with/assert/contexts 없음), clean
self-interpreter, hand annotations, specializer를 Clojure로 FIRST, pnix로
LAST 재작성; go/no-go 게이트는 self-interpreter specialization의
Jones-optimality.

## D1c — explicit-stack non-tail eval (DEFERRED)

지금의 graceful structured `:stack-overflow` bound가 **이미 Nix-parity
답**이다 — 실제 Nix도 deep non-tail stack-safety를 보장하지 않는다(native
C++ 스택 + `max-call-depth`는 함수 호출만 세고 데이터 중첩은 안 셈; 우리와
같은 nested-list/left-spine 형태에서 Nix 자체도 머신 의존 segfault, 미해결
upstream issue #9627). conformance 가치는 사실상 0.

건드릴 유일한 이유는 **functional correspondence**(closure-conversion +
CPS + defunctionalization → CEK/Krivine machine; Ager-Biernacki-Danvy-
Midtgaard PPDP'03)로서, 그 자체가 메타순환/투영 아티팩트 — M-series pillar
작업이지 conformance 작업이 아니다. ★함정: **trampoline**과
**"store-allocated continuations"** 정당화는 0-3으로 반박됐다(clojure.core/
trampoline; arXiv 1007.4446) — 그 위에 짓지 말 것. call-by-need는
Krivine + memoizing-store(CESK) 정제가 필요하다.

## Conformance Phase D — impurity/store 순수 부분집합 (DEFERRED)

conformance MATERIAL이지, pillar가 필요로 할 때만, 그것도 PURE 부분집합만
짓는다. Tvix가 미러할 정확한 아키텍처를 보여준다: 교체 가능한 `EvalIO`
trait(기본 `DummyIO`), `builder_pure`/`builder_impure`, `pure_builtins`
모듈 vs cargo feature 뒤 **7개**뿐인 impure 모듈(`getEnv`/`hashFile`/
`pathExists`/`readDir`/`readFile`/`readFileType`/`currentTime`). 최고 가치
pure/hermetic 부분집합 = `hashString`, `fromTOML`, `toXML`, `toFile`→
content-addressed path, 결정적 path realization — daemon/network 없음.
Fetchers/`currentTime`/`currentSystem`/flake-refs/`findFile`은 진정
impure이므로 같은 seam 뒤에서 시뮬레이션만. Full Tvix도 store를 evaluator
밖에 유지한다(`tvix-store`/`castore` crates).

## self-* generator 후속 순서 (§10에서 이어짐)

[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §10의 observational-equivalence
bottom-up enumerator(`pnix-clj.generate`)와 CEGIS refinement
(`pnix-clj.cegis`, WIKI `generator-cegis`)는 이미 랜딩했다. `/deep-research`
결정이 제시한 **나머지** 순서는 아직 안 지어졌다:

1. **Canonical equivalence-reduction pruning**(Knuth-Bendix) — 평가 전
   후보를 구문적으로 가지치기, 기존 정규 형태(α-canonical + arith-proof
   polynomial + bool-proof truth-table)를 정규형 oracle로(~80% 가지치기
   목표).
2. **Synquid refinement-type 합성** — 증명 가능하지만 수동 논리 명세가
   필요해 self-improve 루프에 자율 공급 불가. 명세 소스에서
   proven-by-construction 후보를 원할 때 재검토.
3. **Library-learning / LLM**(DreamCoder/babble/LILO) — 휴리스틱, corpus
   또는 모델이 필요. 이후 배율기이지 첫 벽돌이 아니다.

## F7 — self-generating cogen 증명 앵커 (랜딩됨, 여기 두지 말 것)

3차 Futamura 투영은 cogen-free curried 경로로 이미 있다
(`pnix-clj.futamura`, WIKI `f7-cogen-collapse`). Glück PEPM'09 collapse는
그 구성에 대해 by-construction으로 기계화됐고, 고전적 self-application
정리는 아니다. F7b(call-by-need self-applicable specializer)만 위 절의
open research로 남는다.

## 증거-저장소 spine의 `origin/main` 포트 — 모호(moot)

`clj-meta-separation.md`의 리팩터 Phase F는 `origin/main`(`cas.clj`/
`store.clj`/`term.clj`/`resolve.clj`)에서 CAS/event-store를 포트하는
계획이었다. 그런데 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §8에서 보듯
증거-저장소 spine 전체가 이미 **독자적으로 clean-rewrite**(옵션 C)돼서
네이티브로 존재한다 — 그래서 이 포트 계획은 더 이상 실행할 필요가 없다.
기록만 남겨둔다.

## 공통 이식 가능 `.px` 라이브러리 트랙 (아직 확정 안 됨)

이 호스트가 만드는 라이브러리는 지금은 **호스트 바인딩 JVM 라이브러리**다
— 이 호스트 언어(Clojure)에서만 로드되며, 다른 호스트가 그대로 쓸 수 있는
이식 가능 공용 바이트코드로 가정하지 않는다. 예전 `pnix-meta` 스타일의
공통 이식 가능 `.px` 라이브러리 트랙은 미룬 상태다 — 호스트-로컬 임포트
작업을 그것 때문에 막지 않는다.

## 미래 아이디어 — pnixMounts / unsafeGetAttrPos 모양 통일 (아직 예정 없음,
2026-08-20)

지금은 만들지 않는다. `pnixMounts`는 Nix 빌트인이 아니고 이 제품에 발명하지
않는다. `unsafeGetAttrPos` 자체는 5개 호스트에 이미 있다 — 남은 건 clj의
`{start; end; span;}`을 hy/cljs/clr/rs의 `{file; line; column;}`로 맞추는
일인데, 착수 확정 전이다.

### unsafeGetAttrPos 모양

- Nix 실제 스펙: 속성이 정의된 위치를 `{ file; line; column; }` 모양으로
  돌려준다.
- 2026-08-20 5개 호스트 상태:
  - hy / cljs / clr / rs: `{file; line; column;}` (인라인 파일 라벨
    `"<pnix-px>"`, 생성 attrset은 null).
  - **clj (여기)**: `{start; end; span;}`(바이트 오프셋) — 파서가 아직
    line/column을 안 들고 있다. 인프라가 생기면 hy 모양으로 바꿀 수 있다.
- 방향 아이디어(확정 아님): line/column 추적은 이 빌트인 하나만을 위한 게
  아니라 에러 메시지 품질 전반에 같이 쓸 수 있는 인프라다 — 파싱/평가 에러가
  지금은 대부분 바이트 오프셋만 주는데, 실제 Nix처럼 "파일:줄:컬럼"으로
  보여주면 디버깅이 훨씬 편해진다. 이 하나만 따로 만들기보다 에러 위치 표시
  개선 작업과 묶는 게 나을 수 있다. 여러 파일을 넘나드는 `import`가 이제
  실제 파일시스템으로 동작하니(2026-08-19 filesystem-import-resolver),
  "어느 파일인지" 추적하는 것도 이제 실제로 의미가 생겼다.

### pnixMounts

- Nix 실제 빌트인 아님 — [`IMPLEMENTATION.md`](IMPLEMENTATION.md)에 이미
  적어뒀듯 `:nix-builtin? false`, `:policy
  :non-faithful-extension-not-nix-coverage`로 명시돼있다. Nix 호환 주장에서
  의도적으로 제외된 pnix 자체 아이디어다.
- 이름과 프로젝트 전체 설계 방향(순수 평가기는 기본적으로 실제 OS
  파일시스템/store에 손을 못 댐 — `storePath`도, 원래 `import`도 전부 이
  원칙 때문에 막혀있다가 2026-08-19에 `import`만 실제 파일 읽기로 확장됨)으로
  미루어 짐작하면(확정 아님, 순전히 추측): "순수 평가기에게 실제 OS
  파일시스템 대신 미리 정해둔 가상 경로 목록(mount)만 제한적으로 보여주는
  기능"일 가능성이 있다.
- 이미 이 저장소의 `import`가 `*import-modules*`(경로 문자열 -> pnix 소스
  텍스트로 된 순수 인메모리 맵, `eval-source-with-imports`)로 정확히 이
  패턴을 증명해뒀다 — 2026-08-19에 실제 파일 읽기(`filesystem-import-resolver`)
  도 추가했지만, 그 인메모리 방식 자체는 여전히 살아있고 재사용 가능하다.
- 방향 아이디어(확정 아님): 나중에 필요해지면 `import`뿐 아니라
  `pathExists`/`readFile`/`readDir` 같은 다른 파일시스템 관련 빌트인들도
  똑같은 "인메모리 mount 맵" 패턴으로 확장하고, `pnixMounts`는 그 맵을
  읽기 전용으로 들여다보는 조회용 빌트인으로 만드는 게 자연스러워 보인다.
  정확한 시그니처/의미는 아직 미정 — 실제로 필요한 상황(재현 가능한 테스트,
  hermetic 빌드 등)이 생겼을 때 다시 설계해야 한다.

**중요**: 위 두 항목은 전부 방향 제시용 메모다. 5개 호스트를 실제로
통일시키는 작업은 기본 언어 기능이 production 수준으로 완전히 갖춰진 다음,
필요에 의해 결정한다.
