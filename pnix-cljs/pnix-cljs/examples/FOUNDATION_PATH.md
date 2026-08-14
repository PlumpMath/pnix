# 파운데이션 경로 — pnix-cljs

일상 사용을 위한 최소 읽기 순서 (연구 사다리 전체가 아님):

1. **`00-foundation`** — Node 모듈로 몇 개 `.px` 폼 평가.
2. **`01-pure-eval-boundary`** — plain `eval` 이 게스트 경계가 아닌 이유.
3. **`02-host-library-import`** — 로컬 라이브러리 export (npm 레지스트리 아님).
4. **`03-outcome-projection`** — 구조화된 결과 모양.
5. **`04-js-embed-pnix`** — host-main 방향.
6. **`05-experimental-honesty`** — 비주장 목록.
7. **`06-meta-pair-boundary`** — 제품 절반 vs meta 절반.
8. **`07`–`10`** — builtins · 파일 평가 · rec/let · JSON 관측 투영.

더 깊은 specialize / machine / oracle 예제는 **pnix-clj** / **pnix-hy** 에 있다.
이 호스트는 표면이 실재할 때만 슬라이스를 추가한다.
