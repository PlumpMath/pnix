# pnix-clr — pnix의 ClojureCLR/.NET 호스트

이 identity들을 분리 유지:

- `clr-meta`는 ClojureCLR host-language bootstrap, focused
  evaluator-generation lane, generic CLR artifact builder, 미래 native CLR
  acceleration을 소유한다. PNIX-agnostic이다: product-owned artifact plan은
  input이지 `clr-meta`에 컴파일된 knowledge가 아니다.
- `pnix-clr`는 .NET 위 PNIX runtime 메커니즘을 소유한다: PNIX parse/evaluate,
  CLR host adapter 제공, runtime-artifact plan의 exact namespace 선언.

pnix-clr는 self-contained다: sibling repository에 의존하지 않고 다른 host와
corpus를 공유하지 않는다.

product dependency는 이제 gate ordering만이 아니라 operational이다.
`clr-meta`는 `pnix-clr/runtime-artifact.edn`을 소비하고, declared source set이
exact임을 require하며, 정확히 eight namespace DLL을 담은 hash-bound
`host-clojureclr-aot` artifact를 emit한다. `bin/pnix-clr`는 plan, source
closure, output closure, exact manifest/tree shape, entrypoint, 모든 recorded
digest를 검증한다. ClojureCLR pinned runtime lookup root의 product namespace
shadow를 거부하고, cwd를 verified artifact로 바꾸며, `CLOJURE_LOAD_PATH`를
그 directory로 교체하고, product code를 artifact에서만 load한다. Missing 또는
stale evidence는 fail closed; product source 또는 build fallback 없음. pinned
ClojureCLR runtime은 explicit substrate로 남는다.

`clr-meta -e`와 file mode는 reader evaluation과 data reader를 비활성으로
정확히 하나의 form을 읽고, admitted portable form domain 밖 value를 거부하며,
physical evaluator generation 2를 사용하고, `load-string` path를 포함하지
않는다. Generation 0, 1, 2는 nested evaluator generation이다; compiler
Stage1, Stage2, Stage15/N이 아니다. 이 nested interpreter를 15
self-extension으로 확장하려는 live 시도는 CLR 스택을 소진한다. 따라서 별도
evaluator resource limit을 드러내며, compiler Stage15/N 증거 또는 stage
receipt가 아니다. `bin/clojure-clr`는 그 generation-2 tool 위 focused
`-e`/single-file compatibility facade일 뿐이다; unsupported command profile은
fail closed이고, pinned upstream compiler/runtime이 그 아래 explicit
bootstrap trust root로 남는다.

현재 `clr-meta`의 별도 selfhost compiler family는 Compiler Stage1--15/N,
StageN, scoped compiler self-reproduction/fixed point까지 live gate로 닫혀 있다.
모든 receipt는 `promotion/allowed?=false`를 유지한다. evaluator generation
0--2와 이 compiler stage family를 혼동하지 말 것. 닫히지 않은 것은 general
CLR IL fixed point, broad ClojureCLR compatibility/replacement, standalone
source-free distribution, PNIX common-compiler integration, host promotion이다.

`pnix-clr` 제품 의존은 operational하다: aggregate gate가 `clr-meta`를 먼저
검증하고 generic artifact builder가 정확한 8-namespace plan을 AOT로 만든 뒤
제품이 그 hash-bound artifact만 검증·로드한다. 다만 현재 제품 backend는
`host-clojureclr-aot`이며 selfhost StageN compiler를 제품 컴파일러로 직접
소비하지 않는다. direct compiler acceleration/replacement는 미래 작업이다.
정본은 `clr-meta/STATUS.md`와 `clr-meta/STAGE15_N_ROADMAP.md`다.

failure는 structured로 유지하고 language-error sink로 `Held`를 쓰지 말 것.

host substrate는 pinned upstream `Clojure` NuGet package (1.12.3-alpha8)이며
`clr-bootstrap/`에서 `bin/build-clr`가 publish한다. 그 signed, version-pinned
package가 explicit bootstrap trust root다; upstream compiler source는 여기에
vendor되지 않는다. cloned JVM/domain surface는 의도적으로 active CLR host로
옮기지 않았다: textual rename은 CLR 증거가 아니므로 여기서는 CLR-owned
mechanism만 port한다.

## Dual-axis + host library (혼동하지 말 것)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).  
C# surface 상세: [`csharp/Pnix.Clr/README.md`](csharp/Pnix.Clr/README.md).

| Axis | Entry | 역할 |
|------|-------|------|
| **host-main (C#)** | `pnix-clr-cs` / MSBuild + `Pnix.Clr` | process→CLI Eval API |
| **host-main (CLR)** | `pnix-clr-clr` / `clojure-clr` | focused `-e`/file facade + library env |
| **pnix-main** | `pnix-clr-pnix` / `pnix-clr` | pnix REPL / `.px` eval |
| **library** | `bin/export-pnix-clr-library` → `pnix-clr-library` | guest AOT `*.clj.dll` + managed DLL + props |
| **meta** | `clr-meta` | pnix-agnostic |

Env: `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (+ legacy
`PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`.

```bash
./bin/export-pnix-clr-library
# flake: nix run .#pnix-clr-library   /   nix run .#pnix-clr-refs
```

Guest AOT DLL은 **ClojureCLR-bound**이며 portable multi-host `.px` package가
아니다. evaluator generation으로 compiler Stage15/N을 주장하지 말 것. Rhino
plugin은 **sdk_8**을 pin; 이 host의 AOT/runtime은 **net10** — TFM을 조용히
혼합하지 말 것.  
HM: `~/dot-nix/dev/cs`.
