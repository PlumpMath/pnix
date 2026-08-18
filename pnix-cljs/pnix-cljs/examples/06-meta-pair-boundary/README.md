# 06 — meta 쌍 경계

## 쉽게 말하면 (비유)
모노레포의 다섯 호스트는 전부 "제품 런타임 절반 + 메타 증명 절반"의 쌍이다.
이 예제는 그 쌍의 **어느 쪽을 이 카탈로그가 다루는지**를 명시적으로 긋는다 —
헷갈리기 쉬운 두 정체성을 표로 분리한다.

## 무엇을
모노레포 이중 축:

| 절반 | 역할 |
|------|------|
| **pnix-cljs** (이 패키지) | pnix 런타임: parse/eval, Node export |
| **cljs-meta** (`pnix-cljs/cljs-meta/`) | 호스트 언어 meta / fixed-point 메커니즘 |

이 `examples/` 카탈로그(00~17)는 **product 쪽만** 보여준다. meta 게이트는
`cljs-meta` 자체 README/bin을 본다.

## 왜 분리하나
`pnix-cljs`는 pnix 소스를 파싱/평가하고 Node로 export하는 **제품 런타임**이고,
`cljs-meta`는 ClojureScript 자기 자신의 self-host/fixed-point 증명을 다루는
**pnix-agnostic** 레인이다 — 서로 다른 질문("pnix가 뭘 하나" vs "cljs-meta가
ClojureScript를 스스로 컴파일할 수 있나")에 답한다. 섞어서 다루면 어느 쪽
주장인지 불분명해진다.

## pnix-cljs의 방식
- 이 카탈로그의 모든 예제는 `dist/pnix-cljs-module.js`(product 절반)만
  `require`한다.
- `16-closures`처럼 cljs-meta 쪽 세부사항이 관련될 때도, 그 경계를 README에
  명시하고 넘어가지 않는다(Stage15/N을 여기서 승격하지 않음).

## 어디에 쓰나
"이 예제가 제품 주장인가 메타 증명 주장인가"를 헷갈릴 때 참고 기준점.

## 실행
읽기 전용 문서다 — 코드 실행은 없다.

## 관련
- 모노레포 `README.md` — 다섯 호스트 쌍 표
- `pnix-cljs/cljs-meta/README.md`
