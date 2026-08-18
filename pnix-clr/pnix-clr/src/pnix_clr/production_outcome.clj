(ns pnix-clr.production-outcome
  "ClojureCLR projection of the common production basic-outcome contract.
  `--self-check` verifies this host's own nominal outcome boundary in
  isolation. `production-report` can additionally replay an externally
  supplied case-matrix TSV (case/kind/phase/class/expected-value/source
  columns) against this host's evaluator; it is not wired to any sibling
  corpus tree by default and supplies only the CLR runtime/observer adapter."
  (:require [clojure.string :as str]
            [pnix-clr.evaluator :as evaluator]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]))

(def report-schema "pnix.production-basic-outcome-report.v1")

(defn project-production-outcome
  [result]
  (cond
    (outcome/done? result)
    (sorted-map
     "error_class" nil
     "error_phase" nil
     "outcome_kind" "done"
     "schema" outcome/production-projection-schema
     "value_json" (json/write-json (outcome/value-of result)))

    (outcome/failed? result)
    (let [error (outcome/error-of result)]
      (sorted-map
       "error_class" (:class error)
       "error_phase" (:phase error)
       "outcome_kind" "failed"
       "schema" outcome/production-projection-schema
       "value_json" nil))

    :else
    (throw (ex-info "basic projection accepts Done or Failed" {}))))

(defn- optional-field
  [value]
  (when-not (str/blank? value) value))

(defn- expected-projection
  [kind phase class-name value-json]
  (sorted-map
   "error_class" (optional-field class-name)
   "error_phase" (optional-field phase)
   "outcome_kind" kind
   "schema" outcome/production-projection-schema
   "value_json" (optional-field value-json)))

(defn- read-cases
  [path]
  (mapv #(str/split % #"\t" 6)
        (remove str/blank?
                (str/split-lines
                 (System.IO.File/ReadAllText
                  (System.IO.Path/GetFullPath (str path)))))))

(defn production-report
  [path]
  (let [matrix
        (mapv
         (fn [[case-name kind phase class-name expected-value source]]
           (let [projection
                 (project-production-outcome
                  (evaluator/eval-source source))
                 expected
                 (expected-projection
                  kind phase class-name expected-value)]
             (sorted-map
              "case" case-name
              "matches_expected" (= expected projection)
              "projection" projection)))
         (read-cases path))
        all-ok (every? #(get % "matches_expected") matrix)]
    (sorted-map
     "all_ok" all-ok
     "host" "pnix-clr"
     "host_outcome_schema" outcome/schema
     "matrix" matrix
     "model_schema" outcome/model-schema
     "schema" report-schema
     "status"
     (sorted-map
      "automatic_codegen" false
      "basic_language_errors_are_held" false
      "legacy_error_transport_is_semantic_owner" false
      "production_basic_outcome_convergence_v1" all-ok
      "production_common_machine_replacement" false
      "production_requested_integration" false
      "production_suspension_equivalence" false))))

(defn- usage!
  []
  (binding [*out* *err*]
    (println
     "usage: pnix-clr.production-outcome --self-check | CASES.tsv"))
  (host/exit! 2))

(defn -main
  [& args]
  (cond
    (= ["--self-check"] (vec args))
    (println (json/write-json (outcome/self-check)))

    (= 1 (count args))
    (let [report (production-report (first args))]
      (println (json/write-json report))
      (when-not (get report "all_ok")
        (host/exit! 1)))

    :else
    (usage!)))
