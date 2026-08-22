# clr-meta TODO / 계속 노트

Full target 정의, stage 번호, promotion ordering은 `STAGE15_N_ROADMAP.md`에
있다 — 이 파일은 반복하지 않는다. 각 stage 번호가 *의미하는 것*은 그 문서를
먼저 읽고, 현재 closed로 검증된 것은 `STATUS.md`를 읽는다. 이 파일은
우선순위 "남은 것" 맵일 뿐이다.

## 현재 남은 작업 (verified 2026-08-11, updated 2026-08-12)

**2026-08-12 업데이트:** 아래 item 1, 2, 3, 5는 모두 DONE (mini backend
widening, Stage8, Stage9, Stage10–15/N 각각) — 각 item 자체 항목에서 무엇이
닫혔고 어떻게 검증됐는지 본다. item 4, 6, 7만 open으로 남았었으나 이후
대부분 closed; item 7만 실질 open.

이 pass 검증 방법 (2026-08-11 baseline, 다루는 범위에 대해 여전히 정확):
`STAGE15_N_ROADMAP.md` + `STATUS.md`를 전부 읽고, 당시 `todo.md`/`SCOPE_LOCK.md`가
아직 없었음을 확인 (`pnix-clr/` 재귀 검사), doc prose를 믿지 않고 real file에
대해 claim spot-check:

- `independent_mini_backend.clj` (177 lines)와 그 test (29 lines, 8
  fixture, `deftest independent-mini-backend-agrees-with-host-eval`)가 존재하고
  `STATUS.md` claim대로 `bootstrap_test.clj`에 정확히 연결됨 —
  done 확인, 아래에서 다시 open 플래그하지 않음.
- `work/compiler-selfhost-stage{3,4,5,6,7}-gate.receipt.json` 모두 존재,
  당일 15:14–15:18 timestamp, `ready: true`, stage7 receipt `claims` 블록이
  문서와 정확히 같음 (`compiler_stage7: true`,
  `same_source_recompile: true`, `stage7_fresh_target_replay: true`,
  `stage8: false`, `self_reproduction: false`, `fixed_point: false`,
  `stage15_n: false`, `clojureclr_replacement: false`) — Stage3–7 closure는
  real, stale doc text 아님.
- `STAGE8_DESIGN.md`도 `stage8` gate/builder script도 `scripts/` 아래
  어디에도 없었음; 모든 stage7 artifact (`stage7-contract.edn`,
  `clr-meta-build-compiler-selfhost-stage7`, stage7 게이트)가 명시적으로
  `stage8: false` stamp — Stage8은 당시 진정 시작되지 않음.
- Housekeeping nit (functional gap 아님): `STAGE15_N_ROADMAP.md` trailing
  "Open claims" 블록이 여전히 단일 `compiler_stage6_through_15_n =
  false` line으로 stage6/7을 시작 안 된 stage8–15/N range와 함께 묶음.
  같은 파일 첫 줄과 `STATUS.md` 둘 다 Stage3–7 closed라고 해도. 다음에 그
  doc을 건드릴 때 `compiler_stage6 = true` / `compiler_stage7 = true` /
  `compiler_stage8_through_15_n = false`로 나누면 좋음 — cosmetic, 게이트
  동작에 영향 없음.

### 우선순위 순 남은 item

1. **Independent mini-backend fixture set 확장 — DONE (2026-08-11, later pass).**
   상태였음: 8 fixture (checked `+`/`-`/`*`, comparison, `if`, 0–2 arg
   fns), full Compiler Stage1 `checked-i64-expression` profile 아님 —
   nested `if`, more arities, Stage1 게이트가 이미 exercise하는
   checked-overflow negative case 누락. 지금: 15 value-returning fixture
   (nested `if`, 3-arg, 4-arg function 추가) + 4 checked-overflow negative
   fixture (`Int64.MaxValue`/`Int64.MinValue` boundary case for `+`/`-`/`*`,
   real-host `eval`과 mini backend 모두 거부 확인 — mini backend의
   `Add_Ovf`/`Sub_Ovf`/`Mul_Ovf` IL opcode는 항상 checked였고 지금껏
   untested). 검증: `independent-mini-backend-test` namespace 38/38
   assertions, full `bin/clr-meta-gate --no-build` 209/209 assertions,
   `:ready true`, regression 없음. open으로 다시 플래그하지 말 것.

   **추가 확장, 2026-08-15: `let` 지원.** JVM host(clj-meta) U6 witness를
   이번 세션에 크게 넓힌 뒤, 나머지 4개 host 중 사용자가 지목한 clr-meta부터
   균형을 맞추려 착수. mini-backend는 여태 `fn`(단일 arity, closure 없음)/
   `if`/이항 산술·비교뿐 — `let`이 없었음(interpreter witness는 이미
   있었지만 완전히 다른 코드 경로). analyzer를 재설계: `env`가 심볼을 arg
   인덱스로 직접 매핑하던 것에서, analyze 시점엔 존재 여부만 추적하고
   (param/`let` 바인딩 둘 다 `:op :local`), 실제 저장(arg 슬롯 vs
   `ILGenerator/DeclareLocal`로 선언한 local)은 EMIT 시점에 결정하는
   방식으로 — JVM host `frontend_selfhost.clj`와 같은 analyze/emit 분리.
   `let` 바인딩은 진짜 `Int64` local(`DeclareLocal`+`Stloc`/`Ldloc`, boxing
   없음). Sequential binding, `let` 안 `if`, outer 이름 shadowing, nested
   `let` 4가지 조합 실제 host `eval` 대비 검증. fixture 5개 추가(15→20
   value fixture). 검증: `bootstrap-test` 20 tests/219 assertions(기존
   209에서 +10) 0 fail/0 error, `./bin/clr-meta-gate eval-only` PASS,
   회귀 없음. Stage1-N compiler-selfhost family는 이 파일과 코드 공유
   없어 전체 chain 재실행 안 함.

   **추가 확장, 같은 날: `loop`/`recur`.** nested `fn`/closure를 다시
   검토하니 `DynamicMethod` 기반(클래스 없음)이라 새 값 종류 +
   일반 apply 메커니즘까지 새로 설계해야 하는 큰 작업임을 확인 —
   대신 지금 아키텍처(전체 Int64) 안에서 자연스러운 `loop`/`recur`로
   진행(clj-meta 쪽도 `let` 다음이 이 순서였음). `analyze-loop`는
   `analyze-let`과 구조 같음 + `recur-arity-key`. `analyze-fn`도 같은
   키를 자기 params 개수로 심어서 **bare recur가 fn 자신을 타깃**하는
   것도 공짜로 지원(named self-recursion 별도 메커니즘 불필요).
   `emit-recur`의 핵심: 모든 새 값을 OLD 값으로부터 임시 local에 먼저
   계산한 뒤에야 실제 타깃(local `Stloc` / fn arg `Starg`)에 저장 —
   Clojure의 "동시 재바인딩" 의미 보존. swap 케이스
   (`(loop [i n a a b b] (recur (- i 1) b a))`)로 이게 실제로 필요함을
   실제 host 대비 검증(양쪽 2 — 순차 구현이면 다른 값). 합산/factorial
   loop, bare recur, swap 4가지 실제 host 일치. fixture 4개 추가
   (20→24). 검증: `bootstrap-test` 20 tests/227 assertions(기존
   219에서 +8) 0 fail/0 error, `./bin/clr-meta-gate eval-only` PASS,
   회귀 없음.

   **추가 확장, 같은 날: nested `fn` (closure, 런타임 값 없이).** 실제
   설계 정리 후 착수 — 핵심 통찰: fixture들의 "closure"는 전부
   즉시-적용(immediately-applied) 중첩 `fn`이라 표준 beta-reduction
   `((fn [p...] a...) args...)` ≡ `(let [p... args...] a...)` 그
   자체이고, 새 런타임 값이나 일반 apply 없이 이미 검증된 `let`
   메커니즘 재사용으로 처리 가능. `desugar` pass 추가: (1)
   beta-reduction, (2) `((let [b...] TAIL) a...)` → `(let [b...] (TAIL
   a...))` let-floating(자식부터 bottom-up + 고정점까지 재귀 적용해서
   임의 깊이/transitive capture 공짜로 처리), (3) named-local-fn
   단일-tail-호출 패턴도 같은 방식으로 인식. **의도적으로 좁힌 경계**:
   진짜 first-class 클로저(저장 후 나중에 호출 등)는 미지원, 시도하면
   "unsupported op fn"으로 명확히 에러(조용히 틀린 값 아님 확인).
   capture-avoiding substitution 아님(형제 인자와 파라미터 이름 충돌
   케이스 미지원, 이 repo 실제 fixture엔 없는 모양). 단순/4단계 중첩,
   `if` 안 중첩, named-local-fn 6가지 조합 실제 host 대비 검증 + 미지원
   형태의 명확한 에러도 확인. fixture 6개 추가(24→30). 검증:
   `bootstrap-test` 20 tests/239 assertions(기존 227에서 +12) 0 fail/0
   error, `./bin/clr-meta-gate eval-only` PASS, 회귀 없음.

   이걸로 clj-meta U6가 이번 세션 초반에 거친 것과 같은 순서(`let` →
   `loop`/`recur` → nested fn)를 clr-meta의 mini-backend도 따라잡음.

   **추가 확장, 2026-08-17: 진짜 first-class 클로저.** desugar가 지울 수
   없는 형태(여러 번 호출, non-tail 호출)를 실제로 채움. `.NET
   Reflection.Emit` 조사로 이 .NET 10.0.10 환경의 플랫폼 특성 확인:
   `DynamicMethod`의 IL generator는 `TypeBuilder`가 만든 생성자/메서드를
   참조 못 함("MethodInfo/ConstructorInfo must be a runtime ... object"),
   반대로 호출하는 쪽 메서드 자체가 TypeBuilder-hosted면 정상 동작(결과
   43로 end-to-end 확인). 그래서 `compile-source`가 AST에 클로저가
   있는지 먼저 검사해서 없으면 기존 `DynamicMethod` 경로 그대로(회귀
   위험 0), 있으면 전체 fn을 새 Run-only dynamic assembly 위
   TypeBuilder-hosted `public static` 메서드로 컴파일 — 그 안에서
   클로저 클래스(캡처마다 필드, 캡처를 저장하는 생성자, non-virtual
   `Invoke(long):long`)를 자유롭게 참조. **의도적으로 좁힌 경계**:
   단일 파라미터만, 캡처는 평범한 Int64만(클로저를 캡처하는 클로저
   불가), 클로저 본문 안에 또 다른 클로저 리터럴 중첩 불가(자유변수
   역산이 안쪽 파라미터를 바깥 캡처로 잘못 묶을 위험) — 셋 다 명확한
   구조적 에러로 거부 확인. 여러 번 호출 + non-tail 호출 + let-바인딩
   캡처까지 포함해 fixture 3개 추가(30→33), 실제 host 대비 검증. 검증:
   `bootstrap-test` 20 tests/245 assertions(기존 239에서 +6) 0 fail/0
   error, 회귀 없음. 상세는 `STATUS.md` 참조.

   **다음 후보:** `independent_mini_interpreter.clj` witness 쪽 fixture
   확장(이번 세션엔 mini-backend만 건드림, 9개 그대로), 사용자의 원래
   "나머지 host들도 clojure만큼" 지시대로 hy-meta/rs-meta/cljs-meta 중
   하나로 넘어가는 것, 또는 item 6/7.

2. **Stage8 — reproducible assembly artifact closure — DONE (2026-08-12).**
   였음: 시작 안 됨 (design doc 없음, gate/builder script 없음, 어디에나
   `stage8: false` stamp). 지금: roadmap generic list를 가정하지 않고
   *실제* non-determinism을 측정으로 발견 (같은 frozen source 두 build,
   byte-diff) — 정확히 두 field만 변함: PE COFF `TimeDateStamp`와 module
   `Mvid`; 이 codegen path에 PDB/debug-info variance 없음 (checked, 가정
   아님). `PeSink.Finish()`가 이제 둘 다 canonicalize
   (`compiler-selfhost-runtime/PeSink.cs` `CanonicalizeForReproducibility`);
   새 `describe-determinism` verb가 finished artifact에서 두 field를 독립
   re-read. 새 게이트 `scripts/clr-meta-compiler-selfhost-stage8-gate`가
   같은 frozen Stage6에서 Stage7을 두 번 build하고 byte-identical output
   요구 — 첫 run PASS. Policy는 `compiler-selfhost/stage8-contract.edn`;
   design은 `STAGE8_DESIGN.md`; `scripts/clr-meta-gate`에 연결. 검증:
   full `bin/clr-meta-gate --no-build` 여전히 green (209/209 assertions,
   Stage1–8 게이트 모두 PASS) — regression 없음. Unplanned bonus live 관찰:
   Stage3–7 자체 compiler DLL이 이제 서로 모두 sha256-identical — structural
   equal만이 아님. 같은 frozen kernel의 그렇지 않으면 identical recompile
   사이에서 변하던 두 가지를 canonicalization이 제거. Stage8을 open 또는
   "not started"로 다시 플래그하지 말 것.

3. **Stage9 — clean-process compiler/runtime replay — DONE (2026-08-12,
   Stage8과 같은 날).** 였음: 시작 안 됨. Stage1-8이 다루지 *않는* 것을
   확인해 실제 gap 발견: 모두 `compiler-selfhost-runtime` support DLL을
   직접 호출하거나, calling shell environment를 상속한 채 in-process
   `bootstrap-test` 실행 — 사용자가 실제로 실행하는 `bin/clr-meta` 자체를
   fully cleared environment (`env -i`, 상속 없음) 아래에서 exercise하지
   않음. 새 게이트 `scripts/clr-meta-compiler-selfhost-stage9-gate`가
   4-case entrypoint matrix (`--gate`, `-e` eval, single-file, reader-
   conditional negative case)를 `env -i` 아래 `bin/clr-meta`로 실행, 각
   case를 독립적으로 *두 번* 실행하고 byte-identical stdout 요구 — 단순
   correctness가 아닌 replay property. 4 case 모두 첫 run에서 content
   검증 통과 (self-consistency만이 아님). Design: `STAGE9_DESIGN.md`;
   `scripts/clr-meta-gate`에 연결. 검증: full `bin/clr-meta-gate --no-build`
   여전히 green, Stage1–9 게이트 모두 PASS, regression 없음. Stage9를
   open 또는 "not started"로 다시 플래그하지 말 것.

4. **Compiler self-reproduction / B==C fixed point — DONE (2026-08-12,
   Stage10-15/N과 같은 날).** 였음: State false — Stage3–7이 same-source
   recompile + immediate parent에 대한 structural-description equality를
   증명했으나 stage가 자신을 byte-identically 재생산함은 아님. Stage8
   canonicalization의 unplanned 결과로 이미 TRUE였고, 형식적으로 검사·주장만
   안 됨: Stage8 자체 게이트 출력이 Stage3-7 공유 compiled-artifact sha256을
   bonus observation으로 이미 로깅. 이 pass 검증 (가정 아님): NEW 전용
   `scripts/clr-meta-compiler-self-reproduction-check`에서 Stage1부터
   Stage7까지 fresh rebuild — 일곱 stage 모두, adjacent pair만이 아니라
   Stage1 host-seeded build 포함, exact same sha256 공유
   (`19872f28ed3576cbdf50001649fb9fd773023778fa0bb5c3d7aee45a61baecb7`).
   모든 stage가 Stage8이 canonicalize한 같은 `PersistedAssemblyBuilder`
   codegen path로 같은 frozen `compiler_kernel.clj`를 compile하므로,
   non-deterministic PE field 두 개만 제거되면 generation 사이에 다를 것이
   없음 — hy-meta/rs-meta가 닫는 "kernelB compiles kernelC, B==C" 패턴의
   가장 강한 형태 (한 adjacent pair만이 아니라 모든 generation identical).
   shared bytes가 vacuously identical-but-broken이 아님 검증: 공유 Stage7
   artifact를 통한 unseen target live compile+execute가 여전히 올바른
   결과 반환. Design: `SELF_REPRODUCTION_DESIGN.md`;
   `scripts/clr-meta-gate`에 연결. full aggregate 게이트 여전히 green,
   regression 없음. open 또는 "state: false"로 다시 플래그하지 말 것 —
   Compiler Stage1-7 `PersistedAssemblyBuilder` artifact family 너머로
   일반화한다고 가정하기 전에 `SELF_REPRODUCTION_DESIGN.md` explicit scope
   note 확인 (general CLR IL fixed point는 더 넓은, 여전히 open claim).

5. **Stage10 (sandbox/session isolation) 및 Stage11–15/N (multi-domain
   adapters, self-improvement quarantine, long-horizon replay, cross-host
   law, open-world evidence, constitutional extension) — DONE (2026-08-12,
   Stage8/9와 같은 날).** 였음: 이 scaffolding 전무 — adapter matrix 없음,
   quarantine storage 없음, cross-host export/import command 없음, 아무것도
   없음. 다른 모든 host (`hy-meta`, `rs-meta`)는 이미 이 전체 range closed.
   그 host가 쓰는 같은 패턴으로 구축 (stage마다 `proofs/` 아래 policy TSV,
   관련 boundary마다 explicit DONE/GROW/HELD/DISABLED stance 선언 + DONE인
   것의 live replay), clr-meta 실제 surface에 맞게 적용:
   `proofs/session-sandbox.tsv` (Stage10, load-context shadow rejection +
   session replay), `adapter-schema.tsv` (Stage11), `quarantine-policy.tsv`
   (Stage12), `horizon-policy.tsv` (Stage13), `cross-impl-schema.tsv`
   (Stage14 — genuine Trusting-Trust bar까지 이미 closed된 한 row로
   `independent-mini-backend` 포함, local self-consistency만이 아님),
   `evidence-federation.tsv` (Stage15), `extension-policy.tsv` (StageN),
   그리고 공통 replay anchor로 재사용되는 새 `proofs/stage-manifest.tsv`와
   `scripts/clr-meta-manifest-check`.
   **구축 중 발견·수정한 cost-shape bug:** 초안이 모든 stage에서 predecessor
   *entire* 게이트를 두 번 replay하여 StageN까지 quadratic cost. Stage11
   이후 모든 stage가 predecessor를 정확히 한 번 호출하도록 수정, Stage11과
   Stage14의 expensive Stage8 rebuild 두 참조는 재실행 대신 latest checked
   receipt를 읽음.
   **live 수정:** `clr-meta-manifest-check` 초안이 `declare -A` (bash 4+
   associative arrays)를 써 macOS system `/bin/bash` (3.2)에서 즉시 실패 —
   이 환경 aggregate 게이트가 그 bash로 돌고, 첫 real aggregate run에서
   실패 표면. plain string matching으로 재작성.
   검증: full `bin/clr-meta-gate --no-build` end-to-end PASS, Stage1부터
   StageN 모두 green, regression 없음. Designs:
   `STAGE{10,11,12,13,14,15,N}_DESIGN.md`. 이 range를 open 또는
   "not started"로 다시 플래그하지 말 것.

6. **Independent-interpreter DDC track (이미 closed된 compiler-backend DDC
   작업과 구별) — DONE (2026-08-12, item 1-5와 같은 날).**
   였음: 전혀 시작 안 됨.
   `src/pnix/clr_meta/independent_mini_interpreter.clj` 구축: from-scratch
   tokenizer/reader + tree-walking interpreter for the small,
   environment-driven Lisp subset `bootstrap.clj` 자체 9-case
   `conformance-cases` corpus가 증명 (`quote`/`if`/`let`/`fn` including named
   recursion and `&` variadic rest), `pnix.clr-meta.main` reader나
   `pnix.clr-meta.bootstrap/evaluate`와 코드 공유 없음.
   *real, textual* `bin/clr-meta -e` evaluator-generation-2 tool-eval path에
   대해 cross-validate (pre-parsed data 아님 — ordinary
   arithmetic/comparison/vector symbol이 injected environment 없이 이미
   resolve됨 live 확인, case마다 placeholder name inject하는
   `conformance-cases` 자체 test harness와 다름) via new
   `scripts/clr-meta-independent-mini-interpreter-gate`. 검증: 9/9 fixture
   첫 run accept, full aggregate 게이트 여전히 green, regression 없음.
   Design: `INDEPENDENT_MINI_INTERPRETER_DESIGN.md`. "not started"로 다시
   플래그하지 말 것 — interpreter alone은 여전히 full Wheeler bar를 자체로
   넘지 않음 (mini-backend 자체 scope note와 같은 honest bar), 그러나 이
   track 자체는 이제 구축·게이트됨.

7. **Broad ClojureCLR compatibility/replacement, `pnix_common_compiler_
   integration`, `cross_host_canonical_equivalence`, `clr_host_promotion`.**
   상태: false, roadmap 자체 ordering이 명시적으로 연기
   (steps 6–9: exact `-e`/file/REPL/compile/AOT/namespace/tooling profile을
   개별 admit, 그다음에야 `bin/clojure-clr`를 현재 facade 너머로 확장,
   그다음에야 common PNIX compiler/machine model에 연결).
   **Size: large / long-horizon** — 위 모든 것 뒤에 올바르게 게이트됨;
   Stage8–15/N이 닫히기 전까지 actionable하지 않았음 (현재 그 범위는 closed).

### 명시적으로 다시 플래그하지 않음 (이미 done, 이 pass 검증)

- Stage1–7 same-source recompile ladder (C0–C3 checkpoints, Stage3–7 게이트).
- generic `clr-meta` CLR artifact builder / `host-clojureclr-aot` /
  `pnix-clr` 8-namespace-DLL manifest binding.
- Trusting-Trust independent mini backend (compiler-side DDC), 8 fixture,
  `bootstrap-test`에 연결.
- `gen0→1→2` evaluator-generation self-interpretation agreement.

## 검증 명령

```sh
# From pnix-clr/clr-meta/
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
./bin/clr-meta-gate                # full family, --no-build default
./scripts/clr-meta-compiler-selfhost-stage7-gate   # newest closed stage
```

full script chain은 `STATUS.md` "Primary 게이트" 섹션, stage 정의와 이
list가 따르는 promotion ordering은 `STAGE15_N_ROADMAP.md` 참조.

## Host toolchain / library export (dot-nix integration, 2026-08-13)

dot-nix는 이미 **CLI runner** (`pnix-clr`, `pnix-clr-pnix`, `clojure-clr`
alias, `pnix-clr-refs` helper)를 노출할 수 있다. 다음은 home-manager만으로는
**불가능** — 이 tree의 product 작업 필요.
**packaging wrapper로 closed라고 주장하지 말 것.**

### 빠진 product surface

1. **Shareable library package (NuGet 및/또는 `lib/` layout)** — **landed 2026-08-13**  
   - `bin/export-pnix-clr-library` → `lib/{net8,net10}/Pnix.Clr.dll` + guest AOT
     + `build/Pnix.Clr.props|.targets` + `share/pnix-clr/refs.env`.  
   - Flake: `packages/apps.pnix-clr-library`, `pnix-clr-refs`, `clojure-clr`.  
   - C# API: `Pnix.Clr.Eval.Source` / `Eval.File` (process→CLI JSON).  
   - Optional `dotnet pack` / multi-machine nupkg: nuget.org와 함께 **dropped**
     (소유자 local feed only, 2026-08-14).

2. **`clojure-clr`를 이름 alias만이 아닌 real host substrate로** — **partial**  
   - Flake/dot-nix가 `clojure-clr` → `bin/clojure-clr` (clr-meta `-e`/file) 노출.  
   - 여전히 누락: 임의 `.clj` 프로젝트를 위한 full “Clojure on CLR”
     (deps.edn / project.clj on CLR), focused facade 너머.

3. **Stable `DOTNET_*` / Reference env contract** — **landed 2026-08-13**  
   - `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (+ legacy
     `PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`.  
   - Manifest는 여전히 `bin/pnix-clr` 안에서 AOT integrity 게이트.  
   - Optional later: export layout drift 시 실패하는 전용 게이트.

4. **“stock CLR tools”의 developer identity**  
   - 명확한 분리 필요: Rhino/net8 plugin SDK vs pnix-clr net10 host SDK —
     overlay가 TFM을 조용히 혼합하지 않도록. (`Pnix.Clr`가 net8+net10을
     multi-target하여 Rhino-side C#이 net8에서 managed Eval API를 reference
     가능.)


## Host-language import of pnix product library (user intent, 2026-08-13)

**Canonical doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`  
**C# surface:** [`../csharp/Pnix.Clr/README.md`](../csharp/Pnix.Clr/README.md)

home-manager (`dot-nix`) integration 컨텍스트:

- `pnix-<host>-pnix` = 이 host 위 pnix-language surface (`.px` REPL/eval).
- `pnix-<host>-<lang>` = day-to-day host development용 host-language
  interpreter/compiler.
- 이 host의 **pnix product half**가 만든 library는 **host-language
  library**다: *이* host language에서 load되어야 한다. 다른 host용 portable
  common bytecode로 가정하지 **않음**.
- 미래 **common portable `.px` library** track (historical pnix-meta style)은
  연기; host-local import 작업을 그것으로 막지 말 것.

dot-nix는 PATH/env (classpath, PYTHONPATH, link paths, NODE_PATH, DLL
HintPath)만 set할 수 있다. real packaging format이 필요한 것은 아래 product
작업.


### clr — 상태 (2026-08-14)

1. **Library package** — **done**: `export-pnix-clr-library` + flake
   `pnix-clr-library` / `pnix-clr-refs` + C# `Pnix.Clr.Eval` + MSBuild props.
2. **ClojureCLR host library story** — **partial**: `clojure-clr` facade +
   props 경유 guest AOT Reference; full arbitrary-.clj project story는 여전히
   open.
3. Versioned env contract — **done**: `PNIX_CLR_*` (+ library path).
4. Dual-axis docs — **done**: monorepo `HOST_DEV_ENV.md` + host `CLAUDE.md` /
   `README.md` + HM matrix.
5. Optional local NuGet — **landed enough**: `bin/pack-pnix-clr-nupkg` +
   `bin/pnix-clr-nupkg-smoke` + `csharp/Directory.Build.props.sample`
   (local feed only; not nuget.org).
6. Explicit note: runtime-artifact `.clj.dll`은 **host-bound** (CLR)이며
   common multi-host .px package가 아님. (여전히 참; 문서화, 반대로 주장하지
   말 것.)

## Post host-env plan (2026-08-14) — 소유자가 끌지 않으면 plan only

Host library export (`export-pnix-clr-library`, `Pnix.Clr.Eval`, MSBuild props,
local nupkg pack)는 C# day-to-day에 **충분히 closed**. 모노레포
`HOST_ENV_P2_P3.md` 참조.

### P3 full ClojureCLR project (상세)
**목표:** focused `clojure-clr -e` / single-file facade 너머 — stable substrate
Reference를 가진 multi-file `.clj` project.

**Acceptance sketch:**
1. `pnix-clr` guest eval과 분리된 문서화된 "plain ClojureCLR REPL" entry.
2. Project template: Clojure NuGet pin + optional `PNIX_CLR_ARTIFACT` guest
   AOT path를 Reference하는 deps 또는 .csproj.
3. 게이트: pnix product CLI 없이 disk에서 2 namespace load하는 smoke.
4. Honest claim만 — Stage15/N 없음, "clojure-clr replaces ClojureCLR" 없음.

**순서:**
1. ~~인벤토리~~ → `docs/CLOJURE_CLR_ADMITTED_SURFACE.md` (2026-08-14).
2. ~~TFM story~~ → `docs/TFM_POLICY.md` (2026-08-14).
3. ~~Template + smoke (bootstrap multi-ns)~~ →
   `examples/clojure-clr-project/` (`./run` / `./smoke` → 42) via
   **clojure-clr-bootstrap**, facade 아님 (2026-08-14).
4. ~~Named profiles + dual smoke~~ → `bin/clojure-clr --help`,
   `bin/clojure-clr-profiles-smoke` (2026-08-14).
5. ~~tool-eval-multi~~ → `--multi-form FILE` +
   `scripts/clr-meta-tool-eval-multi-gate` in `clr-meta-gate` (2026-08-14).
6. ~~Wire profiles-smoke into aggregate~~ → `bin/pnix-clr-gate` runs
   `clojure-clr-profiles-smoke` after `clr-meta-gate` (~17s, 2026-08-14).
7. ~~Local nupkg pack smoke~~ → `bin/pnix-clr-nupkg-smoke` (export layout +
   pack dual-TFM; local feed only, 2026-08-14).
8. ~~nuget.org~~ → **dropped** (owner: personal/local feed only, 2026-08-14).
9. ~~tool-eval-multi-e / stdin~~ → `--multi-e FORM`, `--multi-form -`
   in multi-gate (2026-08-14); default `-e` still single-form.
10. ~~tool-eval fail results carry :profile~~ (2026-08-14).
11. ~~tool-eval surface inventory gate~~ → `clr-meta-tool-surface-gate`
    (full CLI matrix; in clr-meta-gate, 2026-08-14).
12. **Next (product, not packaging):** 새 named 게이트로만 further tool-eval;
    isolated ALC 여전히 held; Stage ladder honesty는 STATUS.md 경유.

### clr-meta residual (product, not packaging)
- STATUS.md + design docs로 stage ladder honesty 계속.
- artifact plan + hash 게이트로만 guest eval 확장.
- packaging 작업으로 stage를 renumber하지 말 것.

### Host-import hard
- [x] In-process C# evaluator **design** — `docs/IN_PROCESS_EVAL.md` (2026-08-14).
- [x] In-process **spike** — `InProcessEval.cs` + `SourceInProcess` (net10),
  parity gate `bin/pnix-clr-inprocess-eval-gate` (17-pass). Process-spawn이
  supported API default로 남음; substrate 있을 때 게이트 auto.
- [x] Local NuGet pack path — `pack-pnix-clr-nupkg` + `pnix-clr-nupkg-smoke`.
  nuget.org publish **dropped** (owner local-only, 2026-08-14).
- [x] In-process broader corpus (14 sources + file + negatives) — gate 17-pass
- [x] substrate+artifact 있을 때 `pnix-clr-gate`에 in-process
  (`PNIX_CLR_INPROCESS_GATE=0` to skip). Isolated ALC 여전히 held.
