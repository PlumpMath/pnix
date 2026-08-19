# pnix-cljs 에이전트 경계

`pnix-cljs`는 PNIX의 ClojureScript/JavaScript 호스트 투영이다.

이 트리는 **자기완결**이다: 형제 저장소에 의존하지 않으며, 다른 호스트와 corpus,
gate, `.px` 코어를 공유하지 않는다.

## 정체성

```text
cljs-meta
  = ClojureScript 호스트 메커니즘 및 self-host 평가 substrate

pnix-cljs
  = CLJS로 구현된 PNIX 파서/평가기 및 JavaScript interop 표면
```

## 영구 규칙

- 호스트 값과 명목 outcome 클래스는 네이티브 ClojureScript 값이다.
- 언어 타입은 구조적 데이터이며, 권위 있는 문자열이 아니다.
- 기본 parse/evaluation 오류는 `Failed`이며, 절대 `Held`가 아니다.
- `cljs-meta` proof 또는 재컴파일은 구현을 검증할 수 있으나,
  일상적인 `pnix-cljs` 평가를 gate 할 수 없다.
- 활성 ClojureScript 소스 클로저에 JVM, Java reflection, ASM, 또는
  Clojure 전용 구현 코드를 복사하지 않는다.
- 이 seed는 다른 호스트들과 아직 공유 corpus/all-host gate로 교차검증되지
  않았다. 2026-08-11 성숙도 패스에서 builtin 표면은 상당히 넓어졌다
  (math, bitwise, list/attrset 헬퍼를 참조 호스트 `evaluator.clj`에서 이식).

## 이중 축 + 호스트 라이브러리 (혼동 금지)

정식 monorepo 문서: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| 축 | 진입점 | 역할 |
|------|-------|------|
| **host-main** | `clojurescript` → `pnix-cljs`; `NODE_PATH`가 있는 `node` | `share/pnix-cljs` 로드 |
| **pnix-main** | `pnix-cljs-pnix` | pnix REPL / `.px` 평가 |
| **library** | flake 패키지 `share/pnix-cljs` | 호스트 바인딩 JS 모듈, 이식 가능 `.px` 아님 |
| **meta** | `cljs-meta` / `pnix-cljs-cljs` | fixed-point 호스트 메커니즘 |

호스트 언어 `.px` import: `require('@plumpmath/pnix-cljs')` 또는
`require('pnix-cljs-module.js')` — [`pnix-cljs/docs/IMPLEMENTATION.md`](pnix-cljs/docs/IMPLEMENTATION.md) §3 참고.  
`shadow-cljs`는 **빌드 오케스트레이터**로 남고, 기본 런타임 호스트는 `pnix-cljs`이다.  
HM: `~/dot-nix/dev/cljs` (`pnix-cljs-host`).
