# examples — pnix-clr (ClojureCLR / .NET)

> **성숙도:** experimental seed. 여기서는 **인정된 표면**만 다룬다
> (pnix-main CLI, 로컬 NuGet/라이브러리 export, C# host-main, 선택적 in-process).  
> Stage15/N, nuget.org 공개 배포, 다섯 호스트 의미 패리티를 **주장하지 않는다**.

## 형제 호스트와 규모

| 호스트 | 카탈로그 크기 (대략) |
|--------|----------------------|
| clj / hy | 연구용 dense 카탈로그 |
| rs | 중간 pillar 카탈로그 |
| **clr** | **코어 00–17** (이 트리) — 실표면이 있을 때만 확장 |

공유 테마 표: 모노레포 [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md).

## 패턴

번호 슬라이스마다 보통:

- `README.md` — 무엇 / 왜 / 실행 방법
- `.px` 및/또는 `pnix-clr/csharp/examples/` C# 프로젝트 포인터

## 카탈로그

| 디렉터리 | 테마 |
|----------|------|
| [`00-foundation`](00-foundation/) | pnix-main seed + production outcome self-check 포인터 |
| [`01-pure-eval-boundary`](01-pure-eval-boundary/) | plain .NET eval vs 게스트 경계 |
| [`02-host-library-import`](02-host-library-import/) | 로컬 Pnix.Clr export (nuget.org 아님) |
| [`03-outcome-projection`](03-outcome-projection/) | production outcome / fail-closed 모양 |
| [`04-csharp-embed-pnix`](04-csharp-embed-pnix/) | host-main HelloPnix |
| [`05-inprocess-opt-in`](05-inprocess-opt-in/) | experimental in-process (net10); 기본은 process-spawn |
| [`06-meta-pair-boundary`](06-meta-pair-boundary/) | pnix-clr vs clr-meta 역할 |
| [`07-builtins-surface`](07-builtins-surface/) | typeOf · getAttrFromPath · lib.sum |
| [`08-production-outcome-self-check`](08-production-outcome-self-check/) | CLI 자신의 outcome 경계 계약 self-check (로컬) |
| [`09-artifact-gate`](09-artifact-gate/) | AOT artifact fail-closed |
| [`10-clojure-clr-multi-ns`](10-clojure-clr-multi-ns/) | bootstrap 다중 ns (호스트 Clojure) |
| [`11-list-higher-order`](11-list-higher-order/) | map · filter · genList · concatLists |
| [`12-with-assert-merge`](12-with-assert-merge/) | with · assert · ++ · // |
| [`13-pattern-lambda`](13-pattern-lambda/) | attrset formal · 기본값 · 커리 |
| [`14-tryEval`](14-tryEval/) | tryEval 성공/throw |
| [`15-string-and-version`](15-string-and-version/) | substring · concatStringsSep · splitVersion |
| [`16-closures`](16-closures/) | 커링 클로저의 바깥 바인딩 캡처 + 다회 재호출 |
| [`17-repl-session`](17-repl-session/) | `--repl` 대화형 pnix REPL 세션 |

## 실행 (대표)

```bash
cd pnix-clr
./bin/build-pnix-clr-artifact   # artifact 없을 때
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
./bin/pnix-clr-library-smoke    # 로컬 피드만
```

모노레포 `examples/host-import/clr/`도 참고. 호스트 Clojure 멀티파일
bootstrap 템플릿은 `pnix-clr/examples/clojure-clr-project/`(이 저장소
루트 기준, `pnix-clr/pnix-clr/`가 아님).
