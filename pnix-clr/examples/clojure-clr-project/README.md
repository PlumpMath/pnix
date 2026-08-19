# ClojureCLR 멀티-ns 프로젝트 예제 (bootstrap 프로파일)

**목적:** `clojure-clr` focused facade(`-e` / 단일 폼)를 넘는 P3.2 step 3
템플릿 — 업스트림 bootstrap + `CLOJURE_LOAD_PATH`로 멀티 네임스페이스 로드.

**TFM:** net10.0만. Rhino net8과 혼동 금지 — `pnix-clr/docs/IMPLEMENTATION.md` §7.

## 레이아웃

```text
src/user/core.clj      ; user.core — demo.math 사용
src/demo/math.clj      ; demo.math — answer 42
```

## 실행

```bash
# monorepo 체크아웃에서 (pnix-clr 루트)
./examples/clojure-clr-project/run.sh
# 또는 프로파일 스모크의 일부:
./bin/clojure-clr-profiles-smoke
```

기대 출력: `42`.

`CLOJURE_LOAD_PATH`가 `src`를 가리키고 bootstrap `Clojure.Main`이
`(require 'user.core)` 후 `(user.core/-main)`를 평가한다.

## 이것이 / 아닌 것

| 이것 | 아닌 것 |
|------|---------|
| bootstrap + load-path 멀티-ns 샘플 | `deps.edn` / tools.deps 전체 패리티 |
| `clojure-clr-bootstrap` 프로파일 증거 | Compiler Stage15/N 또는 self-host 주장 |
| 로컬 스모크/게이트 친화 | nuget.org 또는 공개 배포 스토리 |
