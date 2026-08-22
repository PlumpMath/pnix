# 파운데이션 경로 — pnix-clr

최소 읽기 순서:

1. **`00-foundation`** — `pnix-clr` 로 `.px` 실행.
2. **`01-pure-eval-boundary`** — 게스트 언어 vs 호스트 프로세스.
3. **`02-host-library-import`** — 로컬 dual-TFM 라이브러리 (nuget.org 아님).
4. **`03-outcome-projection`** — 구조화된 production 결과.
5. **`04-csharp-embed-pnix`** — C# host-main.
6. **`05-inprocess-opt-in`** — 선택적 ALC 경로; 기본은 process-spawn.
7. **`06-meta-pair-boundary`** — 제품 절반 vs meta 절반.
8. **`07`–`10`** — builtins · production outcome self-check · artifact 게이트 · multi-ns bootstrap.
9. **`11`–`15`** — 리스트 고차 · with/merge · 패턴 람다 · tryEval · 문자열/버전.
10. **`production-readiness`** — 동일 `.px` library import, PNIX-in-PNIX,
    `clr-meta` evaluator와 Compiler Stage15/N 증거를 한 실행으로 조합.

깊은 self-host stage 사다리의 구현과 정본은 **`pnix-clr/clr-meta/`** 에 있다.
readiness 예제는 그 정체성을 제품에 복사하지 않고 증거만 조합한다.
