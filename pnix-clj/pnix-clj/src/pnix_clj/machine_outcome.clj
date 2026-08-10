(ns pnix-clj.machine-outcome
  (:require [clojure.data.json :as json]
            [clojure.string :as str]
            [pnix-clj.core :as core]))

(def lane-classification
  {:lane :core
   :scope :host-machine-outcome-adapter
   :role :seal-and-observe-common-machine-boundaries
   :product-runtime :allowed
   :semantic-authority :implements-pnix-meta-outcome-abi
   :mutation :none
   :admission :tri-machine-outcome-gate
   :determinism :required
   :allowed-output :canonical-host-boundary-observation})

(def schema "pnix.machine.host-outcome.v1")
(def production-projection-schema
  "pnix.production-basic-outcome-projection.v1")

(deftype EvalError [phase class-name evidence])
(deftype EffectRequest [effect args])
(deftype Continuation [id])
(deftype Checkpoint [id])
(deftype ResourceReason [class-name divergence-proven])

(defprotocol BoundaryOutcome
  (observe-boundary [outcome]))

(deftype Done [value]
  BoundaryOutcome
  (observe-boundary [_]
    (sorted-map "status" "done"
                "value" value)))

(deftype Failed [error]
  BoundaryOutcome
  (observe-boundary [_]
    (sorted-map
     "error" (sorted-map
              "class" (.-class-name ^EvalError error)
              "evidence" (.-evidence ^EvalError error)
              "phase" (.-phase ^EvalError error))
     "status" "failed")))

(deftype Requested [request continuation]
  BoundaryOutcome
  (observe-boundary [_]
    (sorted-map
     "continuation" (sorted-map "id" (.-id ^Continuation continuation))
     "request" (sorted-map
                "args" (.-args ^EffectRequest request)
                "effect" (.-effect ^EffectRequest request))
     "status" "requested")))

(deftype Suspended [checkpoint reason]
  BoundaryOutcome
  (observe-boundary [_]
    (sorted-map
     "checkpoint" (sorted-map "id" (.-id ^Checkpoint checkpoint))
     "reason" (sorted-map
               "class" (.-class-name ^ResourceReason reason)
               "divergence_proven"
               (.-divergence-proven ^ResourceReason reason))
     "status" "suspended")))

(defn machine-outcome?
  [value]
  (or (instance? Done value)
      (instance? Failed value)
      (instance? Requested value)
      (instance? Suspended value)))

(def ^:private basic-classification-by-signal
  {:syntax-error ["parse" "syntax-error"]
   :unsupported-syntax ["parse" "syntax-error"]
   :parse-stack-overflow ["parse" "unsupported-expression"]
   :unknown-variable ["eval" "unknown-variable"]
   :unbound-var ["eval" "unknown-variable"]
   :attribute-missing ["eval" "attribute-missing"]
   :missing-attr ["eval" "attribute-missing"]
   :not-callable ["eval" "not-callable"]
   :call-target-not-callable ["eval" "not-callable"]
   :non-boolean-condition ["eval" "non-boolean-condition"]
   :non-bool-if-condition ["eval" "non-boolean-condition"]
   :if-condition ["eval" "non-boolean-condition"]
   :if-condition-not-bool ["eval" "non-boolean-condition"]
   :if-condition-not-boolean ["eval" "non-boolean-condition"]
   :type-error ["eval" "type-error"]
   :arithmetic-non-number ["eval" "type-error"]
   :eval-binary-failed ["eval" "type-error"]
   :division-by-zero ["eval" "division-by-zero"]
   :integer-overflow ["eval" "integer-overflow"]
   :cycle-detected ["eval" "cycle-detected"]
   :infinite-recursion ["eval" "cycle-detected"]})

(defn- structured-signals [result]
  ;; Order is significant: a stable error class is more specific than the
  ;; compatibility reason that transported it (for example eval-binary-failed
  ;; carrying division-by-zero). Messages and JVM class names are deliberately
  ;; excluded; neither is part of the semantic ABI.
  (filter keyword?
          [(get-in result [:error :class])
           (:class result)
           (get-in result [:error :reason])
           (:reason result)]))

(defn- classify-legacy-error [result]
  (if-let [[phase class-name]
           (some basic-classification-by-signal
                 (structured-signals result))]
    {:phase phase :class class-name}
    {:phase (if (contains? #{:parse "parse"}
                            (get-in result [:error :phase]))
              "parse"
              "eval")
     :class "unsupported-expression"}))

(defn- failed-outcome [phase class-name]
  (Failed. (EvalError. phase class-name (sorted-map))))

(defn legacy-result->outcome [result]
  (if (= :ok (:status result))
    (Done. (:value result))
    (let [{:keys [phase class]} (classify-legacy-error result)]
      (failed-outcome phase class))))

(defn eval-source-outcome
  "Canonical unbounded basic entry. Adds neither hidden fuel nor effects."
  [source]
  (legacy-result->outcome (core/eval-source source)))

(defn- canonical-data [value]
  (cond
    (map? value)
    (into (sorted-map)
          (map (fn [[key item]]
                 [(if (keyword? key) (name key) (str key))
                  (canonical-data item)]))
          value)
    (vector? value) (mapv canonical-data value)
    (sequential? value) (mapv canonical-data value)
    :else value))

(defn- value-json [value]
  (json/write-str (canonical-data value) :escape-slash false))

(defn project-production-outcome [outcome]
  (cond
    (instance? Done outcome)
    (sorted-map
     "error_class" nil "error_phase" nil "outcome_kind" "done"
     "schema" production-projection-schema
     "value_json" (value-json (.-value ^Done outcome)))
    (instance? Failed outcome)
    (let [error (.-error ^Failed outcome)]
      (sorted-map
       "error_class" (.-class-name ^EvalError error)
       "error_phase" (.-phase ^EvalError error)
       "outcome_kind" "failed" "schema" production-projection-schema
       "value_json" nil))
    :else (throw (ex-info "basic projection accepts Done or Failed" {}))))

(defn- expected-projection [kind phase class-name value]
  (sorted-map
   "error_class" (not-empty class-name)
   "error_phase" (not-empty phase)
   "outcome_kind" kind "schema" production-projection-schema
   "value_json" (not-empty value)))

(defn- read-cases [path]
  (mapv #(str/split % #"\t" 6)
        (remove str/blank? (str/split-lines (slurp path)))))

(defn production-report [path]
  (let [matrix
        (mapv
         (fn [[case-name kind phase class-name expected-value source]]
           (let [projection (project-production-outcome
                             (eval-source-outcome source))
                 expected (expected-projection
                           kind phase class-name expected-value)]
             (sorted-map "case" case-name
                         "matches_expected" (= expected projection)
                         "projection" projection)))
         (read-cases path))]
    (sorted-map
     "host" "pnix-clj" "host_outcome_schema" schema "matrix" matrix
     "model_schema" "pnix.machine.eval-outcome-model.v1"
     "schema" "pnix.production-basic-outcome-report.v1"
     "status"
     (sorted-map
      "automatic_codegen" false
      "basic_language_errors_are_held" false
      "legacy_error_transport_is_semantic_owner" false
      "production_basic_outcome_convergence_v1" true
      "production_common_machine_replacement" false
      "production_requested_integration" false
      "production_suspension_equivalence" false))))

(defn self-check
  []
  (let [done (Done. "value")
        failed (Failed. (EvalError. "eval" "not-callable" (sorted-map)))
        requested (Requested.
                   (EffectRequest. "open" (sorted-map))
                   (Continuation. 1))
        suspended (Suspended.
                   (Checkpoint. 2)
                   (ResourceReason.
                    "resource-budget-exhausted"
                    false))
        guest-shape {"outcome_kind" "done"}
        done-observed (observe-boundary done)
        failed-observed (observe-boundary failed)
        requested-observed (observe-boundary requested)
        suspended-observed (observe-boundary suspended)]
    (assert (= "done" (get done-observed "status")))
    (assert (= "eval" (get-in failed-observed ["error" "phase"])))
    (assert (= "not-callable" (get-in failed-observed ["error" "class"])))
    (assert (= "open" (get-in requested-observed ["request" "effect"])))
    (assert (false?
             (get-in suspended-observed
                     ["reason" "divergence_proven"])))
    (assert (not (machine-outcome? guest-shape)))
    (sorted-map
     "all_ok" true
     "done" (get done-observed "status")
     "failed_class" (get-in failed-observed ["error" "class"])
     "failed_phase" (get-in failed-observed ["error" "phase"])
     "guest_shape_is_outcome" (machine-outcome? guest-shape)
     "requested" (get requested-observed "status")
     "requested_effect" (get-in requested-observed ["request" "effect"])
     "schema" schema
     "suspended" (get suspended-observed "status")
     "suspended_divergence_proven"
     (get-in suspended-observed ["reason" "divergence_proven"]))))

(defn -main
  [& args]
  (if (= "--production" (first args))
    (println (json/write-str (production-report (second args))))
    (println (json/write-str (self-check)))))
