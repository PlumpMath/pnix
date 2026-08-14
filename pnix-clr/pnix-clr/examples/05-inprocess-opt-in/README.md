# 05 — in-process eval (opt-in)

## 무엇을

기본 API는 **process-spawn**. net10 실험적 in-process (`SourceInProcess` /
`FileInProcess`)는 substrate + artifact 가 있을 때만, 게이트/스모크에서 opt-in.

## 비주장

- isolated ALC as default
- full parity with process-spawn under all loads
- nuget.org distribution

## 실행

```bash
cd pnix-clr
# when substrate + artifact present:
#   ./bin/pnix-clr-inprocess-eval-gate
# HelloPnix:
#   dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
#     --inprocess --file csharp/examples/hello.px
```

Docs: `pnix-clr/docs/IN_PROCESS_EVAL.md`.
