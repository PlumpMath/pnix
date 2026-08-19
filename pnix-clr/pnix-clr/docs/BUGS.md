# pnix-clr 알려진 버그 / 제한사항 / 의도된 제외

목적: "이거 고쳐야 하나?"라는 질문에 빨리 답하기 위한 문서. 여기 적힌
항목은 두 종류로 나뉜다 — **의도적으로 admit 안 한 것**(버그 아님, 실수로
"고치지" 말 것)과 **실제로 알려진 제한/버그**(외부 의존성 등으로 아직
못 풀었지만, 진짜 제한). 각 항목이 어느 쪽인지 제목에 명시했다.

관련: [`IMPLEMENTATION.md`](IMPLEMENTATION.md)(뭐가 built됐는지),
[`TODO.md`](TODO.md)(지금 집을 수 있는 작업), [`PLANS.md`](PLANS.md)(아직
방향 미정인 것).

## 공통 규칙 — Stage15/N·Trusting-Trust 주장 금지 (의도적, 버그 아님)

여러 원본 문서(`SCOPE_LOCK.md`, `CLOJURE_CLR_ADMITTED_SURFACE.md`,
`IN_PROCESS_EVAL.md`, `IMPLEMENTATION.md` §3)에 반복해서 나오던 규칙을
여기 한 번만 정리한다: 이 호스트의 어떤 조각도 — `clr-meta` tool-eval,
`bin/clojure-clr` facade, in-process C# 평가기 스파이크, evaluator
generation 0/1/2 등 — **그 자체만으로** Compiler Stage15/N,
self-reproduction, IL fixed point, 또는 Trusting-Trust를 증명한 것으로
주장하지 않는다. 이건 각 기능의 "여전히 진행 중"인 상태를 정직하게
유지하기 위한 전역 가드레일이다. `clr-meta` 쪽 정확한 Stage 진행 상태는
`clr-meta/STATUS.md` / `clr-meta/STAGE15_N_ROADMAP.md` 참고(현재
meta-floor: C3 Stage2 닫힘, Stage3–15/N은 open).

## 1. 의도적으로 범위 밖(버그 아님) — `SCOPE_LOCK.md` "범위 밖" 원문 기반

아래는 전부 **이건 버그 아니라 의도된 제한**이다. 필요해지면 나중에
`SCOPE_LOCK.md`(→ 이제 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §5)의
admit 절차를 따라 새로 admit해야 하는 것들이지, 지금 코드에서 놓친 게
아니다.

- JVM classfile, ASM, Java reflection, Maven/JAR execution, 또는 JVM
  fallback 없음
- 어떤 sibling corpus 트리에서든 portable PNIX semantics를 복사해오는 것
  없음
- basic execution에 대한 service admission, deployment policy, 또는
  proof receipt 없음
- Hangul/NL/dictionary/agent/domain 콘텐츠 없음
- 완전한 mature JVM-host parity, IL fixed-point self-hosting, 게이트가
  존재하기 전 established tri-host membership 주장 없음
- Compiler Stage2–15/N, compiler self-reproduction, byte-identical raw
  AOT rebuild, 또는 CLR IL fixed point 없음(새로 admit된 compiler 성장은
  exact C2 selfhost-family Compiler Stage1 artifact뿐)
- broad ClojureCLR language/command/runtime/ecosystem compatibility 또는
  교체 없음; `bin/clojure-clr`는 현재 generation 2를 통한 focused `-e`와
  single-file profile만 admit하며 explicit bootstrap trust root가 host
- standalone source-free distribution 없음; launch validation은 여전히
  live plan과 source closure에 바인딩되고, AOT execution은 pinned
  runtime을 유지
- PNIX common compiler/PIR integration 또는 CLR host promotion 없음
- BigInt arithmetic 또는 Int64 + finite Double을 넘는 full numeric
  promotion 없음
- `pnix.primitive-abi.v1` manifest routing/enforcement, production-evaluator
  primitive-manifest enforcement, 또는 full-builtin manifest enforcement
  없음
- production effect request/resume, finite-fuel suspension,
  common-machine replacement, 또는 canonical-result/JCS completion 없음
- Nix UTF-8 byte-string model, string-context propagation, pattern
  lambda, 또는 derivation/store purity 게이트 없음 — 단, **float literal,
  `with`, list/attrset structural `==`, language `assert`,
  `inherit`/`inherit (expr)`는 이후 admit되어 이미 동작한다**(위 목록에
  넣지 말 것 — `IMPLEMENTATION.md` §5 admit된 것 목록 참고)

## 2. CLI 허용 표면 — 금지된 지름길(의도적, 버그 아님)

`bin/clojure-clr` / `bin/clr-meta` 관련해서 하지 않기로 한 것들
(`CLOJURE_CLR_ADMITTED_SURFACE.md` 원문). 자세한 admitted 표면은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §6 참고.

- 전체 ClojureCLR를 암시하도록 `clojure-clr` 이름을 바꾸지 않는다.
- facade `-e`만으로 Stage15/N 또는 Trusting-Trust를 주장하지 않는다
  (위 "공통 규칙" 참고).
- 명시되지 않은 profile에서 Rhino sdk_8과 pnix-clr net10을 혼합하지
  않는다(§7 TFM 정책 참고).
- `bin/clojure-clr`는 REPL, `-i`, `-M`, deps.edn, clojure CLI 패리티를
  admit하지 않는다 — 요청하면 stderr + exit 2로 fail closed(우회하려고
  하지 말 것; `clojure-clr-bootstrap`가 그 용도의 별도 entrypoint다).

## 3. 프로세스 내 평가기 스파이크 — 비목표(의도적, 버그 아님)

`IN_PROCESS_EVAL.md` 원문. 자세한 내용은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §8 참고.

- 첫 스파이크에 nuget.org 요구하지 않는다(로컬 export 레이아웃으로
  충분).
- `clojure-clr` facade 또는 bootstrap multi-ns 스토리를 교체하지 않는다.
- 임의 사용자 Clojure 프로젝트를 프로세스 내에 로드하지 않는다.
- (옵션 C) ClojureCLR 없이 CLR에서 pnix를 pure managed로 재구현하지
  않는다 — 두 번째 의미 소스가 되고 host-bound 제품 교리에 위배되기
  때문에 **현재 거부**.

## 4. 프로세스 내 평가기 스파이크 — 알려진 제한(진짜 제한, blocked)

이 항목은 "의도된 제외"가 아니라 **외부 의존성 때문에 지금은 못 푸는
진짜 제한**이다. 나중에 조건이 바뀌면(아래 참고) 다시 볼 것.

- **Collectible isolated ALC 언로드 안 됨 — 현재 blocked.** ClojureCLR
  guest AOT가 `Assembly.Load`로 **기본** 컨텍스트에 초기화된다;
  collectible ALC는 이미 로드된 substrate 타입을 dual Resolving 없이 볼
  수 없고, dual Resolving은 Default로 붕괴한다. 문서화된 tradeoff다 —
  **ALC-aware load를 지원하는 substrate가 나오기 전까지는 재검토하지
  않는다.** (`csharp/Pnix.Clr/InProcessEval.cs`)
- **net8 호스트에서는 프로세스 스폰만 유지된다** — in-process API는
  net10.0+ 전용이라 net8 전용 host-main C#은 `Eval.Source`/`Eval.File`
  (process-spawn)만 쓸 수 있다. 이건 TFM 정책(§7)상 당연한 결과이자 현재
  제한이다.
- **Reentrancy: 직렬화, multi-threaded 아님** — `eval-source` 주변에
  global lock이 걸려 있다(ClojureCLR RT가 process-wide이기 때문). 동시
  호출자는 대기하며 `*Async` 헬퍼도 같은 lock을 공유한다. 병렬 처리량이
  필요한 워크로드에는 안 맞는다 — 의도된 설계(ClojureCLR RT 특성)이지만,
  "여러 스레드에서 in-process eval을 동시에 빠르게 돌릴 수 있다"고
  기대하면 안 된다는 뜻에서 여기 기록해둔다.

## 5. 알려진 코드 차이점(설계 특성, 버그 아님)

다른 4개 호스트와 다르게 동작하는 부분들은 이미
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §3에 정리돼 있다 — path 값
타입, `import`/`scopedImport` 예약 키워드, 아티팩트 기반 배포, evaluator
generation vs compiler stage 축 분리. "clr에서만 다르게 동작하네?"
싶으면 여기부터 확인할 것 — 대부분 버그가 아니라 이 호스트의 설계
선택이다.
