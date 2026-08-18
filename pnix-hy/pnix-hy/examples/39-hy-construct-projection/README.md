# 39. Hy 언어 구성체 프로젝션 — defmacro/import/quasiquote/reader-macro/매크로 전개

## 무엇을
`07-hy-macro-over-pnix`가 "Hy 매크로를 pnix 코드 위에 적용"한다면, 이건 반대
방향이다 — **Hy 자신의 언어 구성체**(매크로 정의, import, quasiquote 템플릿,
reader macro 정의, 매크로 전개 단계)를 pnix 쪽에서 다룰 수 있는 **구조화된
값**으로 프로젝션한다. 다섯 개의 독립 함수를 한 예제로 묶는다:

- `hy_defmacro_projection` — `defmacro` 정의를 이름/파라미터/본문 reader-form으로
- `hy_import_projection` — `import`/`require`를 run-time(모듈 바인딩) vs
  compile-time(매크로 스코프) 구분과 함께
- `hy_macro_step_trace` — 매크로 전개를 **고정점까지 단계별로** 추적
  (`macroexpand_1`을 반복)
- `hy_quasiquote_projection` — quasiquote/unquote 템플릿을 code-as-data로
  (splice 구멍까지)
- `hy_reader_macro_projection` — `defreader`(read-time 매크로) 정의 + 등록 여부

## 왜
Python엔 매크로가 없고, `ast` 모듈은 컴파일 후 트리만 준다 — "이 매크로가
정의될 때 무슨 파라미터/본문을 받았나", "전개가 몇 단계 걸렸나", "이
quasiquote 템플릿의 구멍이 어디인가"를 구조화된 값으로 조회할 방법이 없다.

## 무엇을 게이트하나
다섯 함수 모두 `pnix-hy.hy-*-projection.v0`류 스키마를 갖는 결정적 값을
반환하고, reader-form(Hy AST)까지 노출한다 — CLI `--defmacro`/`--import`/
`--macro-steps`/`--quasiquote`/`--reader-macro`가 이 함수들의 얇은 wrapper다.

## 한 줄
> Hy가 매크로/import/reader-macro로 무엇을 하는지, 사람이 코드를 안 읽고도
> 구조화된 값 하나로 조회한다.

## 경계
- 7-`hy-macro-over-pnix`/8-`hy-reader-embed-pnix`(Hy 매크로를 pnix에 적용)와
  방향이 반대다 — 여긴 Hy 구성체 자체를 pnix 쪽 값으로 뽑아낸다.
