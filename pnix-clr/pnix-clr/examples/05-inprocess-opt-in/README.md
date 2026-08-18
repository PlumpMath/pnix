# 05 — in-process 평가 (opt-in)

## 쉽게 말하면 (비유)
`04-csharp-embed-pnix`의 기본 API는 `.px`를 **별도 프로세스**로 스폰해
평가하고 결과만 받는다 — 마치 자식 프로세스를 fork해서 결과를 파이프로
읽는 것과 같다. in-process 모드는 그 대신 같은 프로세스 **안에서** 직접
평가한다 — 다만 이건 실험적 opt-in 경로다.

## 무엇을
기본 API는 **process-spawn**. net10 실험적 in-process(`SourceInProcess` /
`FileInProcess`)는 substrate + artifact가 있을 때만, 게이트/스모크에서
opt-in으로 확인한다. `pnix-clr-inprocess-eval-gate`가 같은 9개 케이스를
process-spawn과 in-process 양쪽으로 돌려 결과가 일치하는지 비교한다.

## plain .NET의 한계
프로세스 스폰은 격리는 확실하지만 매 호출마다 새 프로세스 시작 비용이
붙는다. in-process 평가로 그 비용을 없애려면 별도 `AssemblyLoadContext`
격리, artifact 검증 등을 직접 설계해야 한다 — .NET이 "격리된 in-process
게스트 평가"를 기본 제공하지 않는다.

## pnix-clr의 방식 (`pnix-clr-inprocess-eval-gate`, 실행 결과)
```
== select-path / typeof / length / hasAttr / list-sum-shape
== fail-div0 / fail-unbound / fail-missing-attr
== file-hello
```
9개 케이스 모두 `process:` 경로와 `inproc:` 경로가 **바이트 단위로 같은
값/실패**를 낸다(예: `fail-div0`는 둘 다
`error:{"class":"division-by-zero",...}` exit=1). substrate/artifact가
없을 때는 `NotSupportedException`으로 닫힌다(`negative substrate`/
`negative artifact` 케이스).
```
== summary: pass=17 fail=0
pnix-clr-inprocess-eval-gate: PASS
```

## 비주장
- isolated ALC를 기본으로 함
- 모든 부하에서 process-spawn과 완전 패리티
- nuget.org 배포

## 어디에 쓰나
프로세스 스폰 비용이 부담되는 고빈도 호출 경로에서, substrate/artifact를
직접 관리할 수 있는 환경에 한해 실험적으로 시도해볼 때.

## 실행
```bash
cd pnix-clr
# substrate + artifact 가 있을 때:
#   ./bin/pnix-clr-inprocess-eval-gate
# HelloPnix:
#   dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
#     --inprocess --file csharp/examples/hello.px
```

문서: `pnix-clr/docs/IN_PROCESS_EVAL.md`.
