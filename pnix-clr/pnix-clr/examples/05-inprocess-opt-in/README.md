# 05 — in-process 평가 (opt-in)

## 무엇을

기본 API 는 **process-spawn**. net10 실험적 in-process (`SourceInProcess` /
`FileInProcess`) 는 substrate + artifact 가 있을 때만, 게이트/스모크에서 opt-in.

## 비주장

- isolated ALC 를 기본으로 함
- 모든 부하에서 process-spawn 과 완전 패리티
- nuget.org 배포

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
