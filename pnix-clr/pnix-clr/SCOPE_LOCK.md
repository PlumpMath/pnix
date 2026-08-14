# pnix-clr / clr-meta scope lock

`pnix-clr`는 ClojureCLR 호스팅 PNIX 메커니즘이다. `clr-meta`는 별도의,
PNIX-agnostic ClojureCLR meta/bootstrap lane이다.

## Bootstrap 범위

- 제품 소유 `runtime-artifact.edn` plan과 exact source closure를 소비하는
  PNIX-agnostic `clr-meta` artifact builder
- 별도 버전된 `clr-meta` selfhost compiler family: closed C0/C1
  source admission 및 exact low-level support ABI를 구현하고
  explicit pinned-host B0 trust root를 통해 canonical kernel에서
  executable Compiler Stage1 PE를 생성하는 소유자 승인 C2 slice;
  C2는 Stage2와 self-reproduction을 false로 유지하면서
  fresh-process unseen-target compilation과 semantic mutation
  propagation을 증명해야 함
- 그 plan이 선언한 정확히 아홉 DLL을 담은 `host-clojureclr-aot` manifest,
  plan/source/output hash와 explicit entrypoint 포함
- artifact-only PNIX product loading: live plan, source set,
  output set, exact manifest/tree shape, 기록된 모든 digest 검증;
  pinned-runtime 및 cwd namespace shadow 거부, cwd와 load path를
  artifact로 교체, product source를 compile하거나 load하는 대신 fail closed
- physical evaluator generation 2를 통한 `clr-meta` focused tool evaluation,
  non-evaluating, exact-one-form portable-domain reader, `load-string`
  tool path 없음
- CLR-native PNIX tokenization, parsing, evaluation mechanism
- `pnix.machine.host-outcome.v1`을 구현하는 nominal CLR
  `Done | Failed | Requested | Suspended` carrier와 observer;
  guest map으로 위조 불가
- `Done` 및 structured `Failed`용 production evaluator 통합, 그리고
  `Requested`와 `Suspended`에 대한 carrier/observer shape만
- 공통 11-case basic-outcome contract에 필요한 exact integer/string/
  `if`/checked-`+`/integer-`/` 메커니즘
- `../../pnix-meta`에서 canonical module의 relative, read-only,
  lexically confined loading (아직 symlink-safe security boundary 아님)
- deterministic seed JSON projection; `bool-01` bytes가 pinned common
  expected file과 일치
- dead `if` branch, unused argument, unselected attr field가 import
  expression을 resolve하거나 read하지 않는다는 common-corpus 증거
- null 및 bool/int/string scalar equality와 static identifier attr-path
  `?`, application binding이 `?`보다 더 tight; structural equality는
  여전히 제외
- source-originated `System.Int64` unary negation 및 checked add, subtract,
  multiply, truncating division, `production-checked-i64-01`이 요구하는
  structured overflow 및 lazy dead-overflow 동작 포함
- **README corpus language surface** (clj/hy/rs/cljs와의 peer parity 의도):
  builtins + `lib` (core/attrs/lists/strings/predicates/math/combinators/FS/
  best-effort fetch), nested attr path (`foo.bar = expr`), partial builtin
  application, `root-environment` frame. 기존 아홉 namespace 안에서 구현
  (`evaluator.clj` / `host.clj`); 새 artifact namespace 없음
- ClojureCLR/.NET host adapter
- JVM host로 fallback할 수 없는 focused net10 게이트

런타임은 그 surface 너머로 의도적으로 좁다. 추가 syntax 또는 ABI claim은
oracle 증거와 common-corpus 합의로만 admit된다. README surface 확장은
tri-host promotion을 **확립하지 않는다**.

Artifact dependency는 layer identity를 병합하지 않는다. `pnix-clr`는
namespace plan과 PNIX 메커니즘을 소유; `clr-meta`는 generic validation과 CLR
artifact production을 소유; `pnix-meta`는 portable meaning을 소유. pinned
ClojureCLR compiler/runtime은 explicit bootstrap 및 host-AOT trust root로
남는다.

Evaluator generation 번호와 compiler stage 번호는 분리된다. 현재 evaluator
generation 0, 1, 2는 focused nested interpreter를 증명한다. Compiler Stage1,
Stage2, 또는 Stage15/N을 증명하지 않는다. 그 nested interpreter를 15
self-extension으로 확장하는 것은 현재 CLR 스택을 소진하며, open host
resource limitation이지 `Held` 결과 또는 stage receipt가 아니다.
**clr-meta meta floor는 C3 Stage2로 남음; Stage3–15/N은 여전히 open.**

## 범위 밖

- JVM classfile, ASM, Java reflection, Maven/JAR execution, 또는 JVM fallback
- `../../pnix-meta`에서 portable PNIX semantics 복사
- basic execution에 대한 service admission, deployment policy, 또는 proof receipt
- Hangul/NL/dictionary/agent/domain 콘텐츠
- 완전한 mature JVM-host parity, IL fixed-point self-hosting, 또는
  게이트가 존재하기 전 established tri-host membership 주장
- Compiler Stage2--15/N, compiler self-reproduction, byte-identical raw AOT
  rebuild, 또는 CLR IL fixed point; 새로 admit된 compiler 성장은 위에서
  기술한 exact C2 selfhost-family Compiler Stage1 artifact뿐
- broad ClojureCLR language/command/runtime/ecosystem compatibility 또는
  교체; `bin/clojure-clr`는 현재 generation 2를 통한 focused `-e`와
  single-file profile만 admit하며 explicit bootstrap trust root가 host
- standalone source-free distribution; launch validation은 여전히 live
  plan과 source closure에 바인딩되고, AOT execution은 pinned runtime을 유지
- PNIX common compiler/PIR integration 또는 CLR host promotion
- BigInt arithmetic 또는 Int64 + finite Double을 넘는 full numeric promotion
- `pnix.primitive-abi.v1` manifest routing/enforcement, production-evaluator
  primitive-manifest enforcement, 또는 full-builtin manifest enforcement
- production effect request/resume, finite-fuel suspension, common-machine
  replacement, 또는 canonical-result/JCS completion
- Nix UTF-8 byte-string model, string-context propagation, pattern lambda,
  또는 derivation/store purity 게이트
  (float literal, `with`, list/attrset structural `==`, language
  `assert`, `inherit` / `inherit (expr)`는 admit됨)

## 규칙

먼저 `clr-meta`를 build하고 gate한다. aggregate 게이트는 그다음 exact AOT
artifact를 build하고 negative matrix를 검사하며, seed `pnix-clr` runtime을
그 artifact를 통해서만 admit한다. Missing 또는 stale artifact 상태는
infrastructure/configuration 실패이며, source 또는 bootstrap fallback을
허가하지 않는다. `pnix-clr`는 common `.px`를 load한다. Unsupported language
input은 nominal structured `Failed` outcome을 반환; `Held`로 안전하게 만들지 않는다.

목표 순서는 compiler Stage1, self-reproducing Stage2, 반복
Stage3--7 convergence, Stage8--15/N hardening, 개별 admit
ClojureCLR compatibility profile, 그다음에서야 bootstrap-hosted focused
facade에서 generated compiler tool로 더 넓은 compatibility command 이전이다.
PNIX common-compiler integration과 CLR host promotion은 그 이후 독립적으로
닫힌다. `../clr-meta/STAGE15_N_ROADMAP.md` 참조. 현재 CLR artifact/adoption
게이트 통과는 증거이지, established host로서의 자동 교체 또는 admission이 아니다.
`../../project-wiki/CONSTITUTION.md`의 공유 헌법이 권위 문서로 남는다.
