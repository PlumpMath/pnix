(ns pnix-clr.main
  (:require [clojure.string :as str]
            [pnix-clr.evaluator :as evaluator]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]
            [pnix-clr.production-outcome :as production-outcome]))

(def cli-commands
  "The complete pnix-clr CLI surface: [argv-form description]. Single source
  of truth for `usage!` and `capabilities-doc` so the two documents of the
  same surface cannot silently drift apart -- add a new -main dispatch clause
  here first, then wire it below."
  [["-e SOURCE" "인라인 SOURCE 평가, CLI JSON projection 출력"]
   ["FILE.px" "파일 평가, CLI JSON projection 출력"]
   ["--repl" "대화형 pnix REPL (개발자 진입점, evaluation authority 아님)"]
   ["--production-outcome-self-check" "내장 production-outcome self-check 실행"]
   ["--production-outcome CASES.tsv" "TSV 파일의 production-outcome 케이스 실행"]
   ["capabilities" "생성된 능력 인덱스를 stdout에 출력 (docs/CAPABILITIES.md 소스)"]
   ["capabilities-check" "커밋된 docs/CAPABILITIES.md를 방금 렌더링한 것과 비교하는 drift 게이트"]])

(defn usage! []
  (binding [*out* *err*]
    (println (str "usage: pnix-clr " (str/join " | " (map first cli-commands))))))

(defn- render
  "Render a PNIX value in Nix surface notation, the same shape the other hosts'
  pnix REPLs print, so a value reads identically whichever host evaluated it.
  Non-transparent values fall back to an opaque marker rather than exposing a
  host representation."
  [v]
  (cond
    (nil? v) "null"
    ;; CLR's Boolean.ToString is "True"/"False"; PNIX surface syntax is lower
    ;; case, so the literal is written out rather than stringified.
    (instance? System.Boolean v) (if v "true" "false")
    (string? v) (pr-str v)
    (number? v) (str v)
    (map? v) (str "{ "
                  (->> (sort-by key v)
                       (map (fn [[k x]] (str k " = " (render x) ";")))
                       (str/join " "))
                  " }")
    (sequential? v) (str "[ " (str/join " " (map render v)) " ]")
    :else "«opaque»"))

(defn projection
  [result]
  (cond-> {"schema" "pnix-clr.cli-result.v1"
           "host" "pnix-clr"
           "outcome_kind" (name (outcome/kind result))}
    (outcome/done? result) (assoc "value" (outcome/value-of result))
    (outcome/failed? result) (assoc "error" (outcome/error-of result))))

(defn- path-inside?
  [root path]
  (let [relative (-> (System.IO.Path/GetRelativePath
                      (host/canonical-path root)
                      (host/canonical-path path))
                     (str/replace "\\" "/"))]
    (not (or (= relative "..")
             (str/starts-with? relative "../")
             (System.IO.Path/IsPathRooted relative)))))

(defn- file-root
  [base-root file]
  (if (path-inside? base-root file)
    base-root
    (System.IO.Path/GetDirectoryName (host/canonical-path file))))

(defn- print-result!
  [result]
  (println (json/write-json (projection result)))
  (when (outcome/failed? result)
    (host/exit! 1)))

(defn- repl-eval-print!
  "Evaluate one REPL line. A structured failure is reported and the loop
  continues; only the process exit status is reserved for the batch modes."
  [root source]
  (let [result (evaluator/eval-source
                source
                {:root root
                 :file (host/combine root "pnix-clr-repl.px")})]
    (if (outcome/done? result)
      (println (render (outcome/value-of result)))
      (println "error:" (json/write-json (outcome/error-of result))))))

(defn- repl!
  "Interactive pnix REPL. Reads one expression per line; `:q` or EOF ends it.
  This is a developer entry point: it never becomes an evaluation authority,
  and it routes every expression through the same `evaluator/eval-source` the
  batch modes use."
  [root]
  (println "pnix-clr — pnix REPL. :q to quit.")
  (loop []
    (print "pnix> ")
    (flush)
    (when-let [line (read-line)]
      (let [trimmed (str/trim line)]
        (when-not (contains? #{":q" ":quit" ":exit"} trimmed)
          (when (seq trimmed)
            (try
              (repl-eval-print! root trimmed)
              (catch System.Exception error
                (println "!" (.Message error)))))
          (recur))))))

(defn- wrap-names
  "Word-wrap a sorted name list into fixed-width lines for markdown output."
  [names per-line]
  (->> (partition-all per-line names)
       (map #(str/join " " %))
       (map #(str "  " %))
       (str/join "\n")))

(defn capabilities-doc
  "The full text of docs/CAPABILITIES.md, derived only from code: the
  `cli-commands` table above (also `usage!`'s single source of truth) and the
  live `builtins-entries` registration table in pnix-clr.evaluator, reached
  through `evaluator/builtin-names`. Nothing here is a hand-typed snapshot --
  a builtin added to or removed from `builtins-entries`, or a CLI form added
  to `cli-commands`, changes this string the next time it is rendered."
  []
  (let [names (evaluator/builtin-names)]
    (str
     "# pnix-clr CAPABILITIES — 생성물 (손 편집 금지 / GENERATED — do not hand-edit)\n"
     "\n"
     "> 재생성: `bin/pnix-clr capabilities > pnix-clr/docs/CAPABILITIES.md`"
     " (저장소 루트 `pnix-clr/`에서 실행).\n"
     "> drift 게이트: `bin/pnix-clr capabilities-check`"
     " (`bin/pnix-clr-gate`에 연결됨).\n"
     "\n"
     "코드에서만 파생된 인덱스 -- 손으로 쓴 텍스트나 타임스탬프 없음. 서술형 설명"
     "(아키텍처, 5개 호스트 비교, 범위 경계)은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md)"
     " 참고 -- 이 문서는 조회용 인덱스일 뿐이다.\n"
     "\n"
     "## CLI 명령 (`bin/pnix-clr`)\n"
     "\n"
     "| 형태 | 동작 |\n"
     "|---|---|\n"
     (str/join "\n" (map (fn [[form description]]
                           (str "| `" form "` | " description " |"))
                         cli-commands))
     "\n"
     "\n"
     "## 빌트인 presence (" (count names) "종)\n"
     "\n"
     "`pnix-clr.evaluator/builtin-names`가 root `builtins-entries` 등록 테이블"
     "(이름 -> 빌트인/상수)에서 직접 뽑은 목록 -- 손으로 옮겨 적지 않았으므로 빌트인이"
     " 추가/삭제되면 재생성 시 자동으로 따라온다. presence는 등록된 이름과 arity만 볼"
     " 뿐 호출 시 실제 semantics/parity를 주장하지 않는다; 5개 호스트 비교표는"
     " [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §2 참고.\n"
     "\n"
     (wrap-names names 8)
     "\n"
     "\n"
     "`import`/`scopedImport`는 이 호스트에서 예약 키워드(파서 전용 문법)로 구현돼 있어"
     " `builtins-entries` 등록 패턴에 안 잡힌다 -- 실제로는 둘 다 있다"
     "(`IMPLEMENTATION.md` §1). `builtins`(재귀 자기참조)는 `builtins-entries`가"
     " 아니라 `make-builtins`가 별도로 붙이므로 위 목록에는 없다.\n")))

(defn- capabilities-doc-path
  [root]
  (host/combine root "pnix-clr" "docs" "CAPABILITIES.md"))

(defn- capabilities-check!
  "Drift gate: regenerate capabilities-doc in memory and diff it byte-for-byte
  against the committed docs/CAPABILITIES.md. Fails loudly (non-zero exit) on
  any mismatch, including a missing file."
  [root]
  (let [path (capabilities-doc-path root)
        expected (capabilities-doc)]
    (if-not (host/file-exists? path)
      (do (binding [*out* *err*]
            (println
             (str "capabilities-check: FAIL -- " path " missing; run"
                  " `bin/pnix-clr capabilities > pnix-clr/docs/CAPABILITIES.md`")))
          (host/exit! 1))
      (let [actual (host/read-file-text path)]
        (if (= expected actual)
          (println (str "capabilities-check: PASS -- " path
                        " matches the generated index"))
          (do (binding [*out* *err*]
                (println
                 (str "capabilities-check: FAIL -- drift between " path
                      " and the generated index; run"
                      " `bin/pnix-clr capabilities > pnix-clr/docs/CAPABILITIES.md`")))
              (host/exit! 1)))))))

(defn -main
  [& args]
  (let [args (vec args)
        host-root (host/default-root)]
    (cond
      (and (= 2 (count args)) (contains? #{"-e" "--eval"} (first args)))
      (print-result!
       (evaluator/eval-source
        (second args)
        {:root host-root
         :file (host/combine host-root "pnix-clr-inline.px")}))

      (= ["--repl"] args)
      (repl! host-root)

      (= ["--production-outcome-self-check"] args)
      (production-outcome/-main "--self-check")

      (and (= 2 (count args))
           (= "--production-outcome" (first args)))
      (production-outcome/-main (second args))

      (= ["capabilities"] args)
      (do (print (capabilities-doc)) (flush))

      (= ["capabilities-check"] args)
      (capabilities-check! host-root)

      (= 1 (count args))
      (let [file (host/canonical-path (first args))
            root (file-root host-root file)]
        (print-result! (evaluator/eval-file root file)))

      :else
      (do
        (usage!)
        (host/exit! 2)))))
