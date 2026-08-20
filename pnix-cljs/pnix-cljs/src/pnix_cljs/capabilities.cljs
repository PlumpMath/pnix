(ns pnix-cljs.capabilities
  "Generated capability index (docs/CAPABILITIES.md), derived ONLY from code
  -- no hand-written text, no timestamps. The builtin presence list comes
  straight from `evaluator/builtins-value`'s key set (the same table
  `invoke-builtin` dispatches against, per IMPLEMENTATION.md §1) via
  `builtin-public-names`, so adding/removing a builtin there changes this
  doc's output the next time it is regenerated -- no separate list to keep
  in sync by hand. Mirrors pnix-rs's `capabilities_doc()` /
  `cmd_capabilities_check()` pattern (see pnix-rs/src/main.rs) sized down to
  this host's narrower scope (no big self-hosting-tower namespace surface to
  reflect over). Resolves the 'CAPABILITIES.md 자동 생성기 없음' backlog item
  in docs/PLANS.md."
  (:require [clojure.string :as string]
            [pnix-cljs.evaluator :as evaluator]))

(def ^:private node-path (js/require "path"))
(def ^:private fs (js/require "fs"))

(defn builtin-public-names
  "The presence inventory: every key registered in `builtins-value`
  (callables, value constants like true/false/null/langVersion/nixVersion/
  storeDir, and the recursive `builtins` self field), sorted. Introspection
  of the live table, not a hand-typed list."
  []
  (->> (:fields (evaluator/builtins-value))
       keys
       sort))

(defn capabilities-doc
  "Build the docs/CAPABILITIES.md markdown string from code. Pure function of
  the current build (no filesystem access) so it can be compared byte-for-
  byte against the committed file by `capabilities-check`."
  []
  (let [names (builtin-public-names)]
    (str
     "# pnix-cljs CAPABILITIES — 능력 인덱스 (중복개발 방지 조회)\n"
     "\n"
     "> 생성: `node pnix-cljs/dist/pnix-cljs.js capabilities > pnix-cljs/docs/CAPABILITIES.md` — 손 편집 금지.\n"
     "> drift 게이트: `node pnix-cljs/dist/pnix-cljs.js capabilities-check`.\n"
     "\n"
     "## CLI 명령\n"
     "\n"
     "| 명령 | 목적 |\n"
     "|---|---|\n"
     "| `-e SOURCE` / `--eval SOURCE` | 인라인 소스 평가 → canonical JSON 프로젝션 |\n"
     "| `FILE` | `.px` 파일 평가 → canonical JSON 프로젝션 |\n"
     "| `--repl` | 대화형 REPL (Node `readline`, `core/projection` 그대로 사용) |\n"
     "| `capabilities` | 이 문서 생성 (stdout) |\n"
     "| `capabilities-check` | drift 게이트 — 재생성 결과 vs 커밋된 `docs/CAPABILITIES.md` |\n"
     "\n"
     "## 모듈 (`src/pnix_cljs/`)\n"
     "\n"
     "| 파일 | 역할 |\n"
     "|---|---|\n"
     "| `tokenizer.cljs` | 렉서 |\n"
     "| `parser.cljs` | 재귀 하강 파서 → AST |\n"
     "| `evaluator.cljs` | 값 표현 + 평가기 + 빌트인 dispatch (가장 큼) |\n"
     "| `core.cljs` | `eval-source`/`projection`/`canonical-json` 진입점 |\n"
     "| `module.cljs` / `node_loader.cljs` | Node용 import 소스 로딩 어댑터 |\n"
     "| `main.cljs` | CLI 진입점 (`dist/pnix-cljs.js`) |\n"
     "| `capabilities.cljs` | 이 문서의 생성기 |\n"
     "\n"
     "## 호스트 라이브러리 진입점 (Node / CommonJS)\n"
     "\n"
     "| 대상 | 빌드 결과물 | shadow-cljs main |\n"
     "|---|---|---|\n"
     "| CLI | `dist/pnix-cljs.js` (`package.json` `bin.pnix-cljs`) | `pnix-cljs.main` |\n"
     "| require 라이브러리 | `dist/pnix-cljs-module.js` (`package.json` `main`) | `pnix-cljs.module` |\n"
     "| self-test | `dist/pnix-cljs-self-test.js` | `pnix-cljs.self-test` |\n"
     "\n"
     "require API 상세(`evalSource`/`evalFile`/... 전체 표면): [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) §3.\n"
     "\n"
     "## 빌트인 표면 (presence inventory)\n"
     "\n"
     "등록된 `builtins-value` 키 " (str (count names)) "개(콜러블 + 값 상수 `true`/`false`/`null`/`langVersion`/`nixVersion`/`storeDir` + 재귀 `builtins` 필드):\n"
     "\n"
     (string/join " " names) "\n"
     "\n"
     "presence는 호출 parity 주장이 아니다 — 실제 실행 의미는 `evaluator/invoke-builtin`을 참고.\n"
     "5개 호스트 비교표는 [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) §4.\n"
     "\n"
     "## 관련 문서\n"
     "\n"
     "narrative 상세(아키텍처, 스코프, known 차이점, 역사)는 [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) 전체 참고 — 이 문서는 코드에서 파생된 terse 인덱스일 뿐이다.\n")))

(defn- capabilities-md-path
  "docs/CAPABILITIES.md, resolved relative to this compiled bundle's own
  location (dist/) rather than the invoking process's cwd, so the check
  works the same whether invoked from the repo root, this package root, or
  anywhere else `bin/pnix-cljs-gate` happens to `cd` from."
  []
  (.resolve node-path js/__dirname ".." "docs" "CAPABILITIES.md"))

(defn write-capabilities!
  "Regenerate the committed docs/CAPABILITIES.md from code."
  []
  (.writeFileSync fs (capabilities-md-path) (capabilities-doc)))

(defn capabilities-check
  "Drift gate: regenerate in memory and diff against the committed file.
  Returns {:pass bool :message string}."
  []
  (let [path (capabilities-md-path)]
    (if (.existsSync fs path)
      (let [on-disk (.readFileSync fs path "utf8")
            generated (capabilities-doc)]
        (if (= on-disk generated)
          {:pass true
           :message (str "  ok   " path " matches the generated index\n  => PASS (1 passed, 0 failed)")}
          {:pass false
           :message (str "  FAIL drift — run `node dist/pnix-cljs.js capabilities > " path "`\n  => FAIL (0 passed, 1 failed)")}))
      {:pass false
       :message (str "  FAIL missing " path " — run `node dist/pnix-cljs.js capabilities > " path "`\n  => FAIL (0 passed, 1 failed)")})))
