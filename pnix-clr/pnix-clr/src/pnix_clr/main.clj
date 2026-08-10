(ns pnix-clr.main
  (:require [clojure.string :as str]
            [pnix-clr.evaluator :as evaluator]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]
            [pnix-clr.production-outcome :as production-outcome]))

(defn usage! []
  (binding [*out* *err*]
    (println
     (str "usage: pnix-clr -e SOURCE | FILE.px | --repl"
          " | --production-outcome-self-check"
          " | --production-outcome CASES.tsv"))))

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

      (= 1 (count args))
      (let [file (host/canonical-path (first args))
            root (file-root host-root file)]
        (print-result! (evaluator/eval-file root file)))

      :else
      (do
        (usage!)
        (host/exit! 2)))))
