(ns pnix-clj.mirror
  (:require [pnix-clj.px-runtime :as px-runtime]
            [pnix-clj.stage15 :as stage15]))

(def lane-classification
  {:lane :core
   :scope :cross-runtime-mirror-evidence
   :role :build-host-px-and-pnix-mirror-rows
   :product-runtime :allowed
   :semantic-authority :cross-lane-evidence-only
   :mutation :forbidden
   :admission :mirror-verdict-only
   :determinism :required-upstream
   :allowed-output :mirror-row-or-cross-mirror-verdict})

(def default-facets
  [:host/parse
   :host/eval
   :host/lower
   :host/clj-meta
   :inner/stage15-control
   :inner/px-runtime
   :inner/pnix-mirror
   :cross/value-agreement])

(defn clojure-mirror-row
  [{:keys [source-hash ast-hash lowered clj-meta-result stage15-control]}]
  {:kind :clojure-mirror
   :source-hash source-hash
   :ast-hash ast-hash
   :lowered-form (:form lowered)
   :lowered-form-hash (:form-hash lowered)
   :bytecode-hash (get-in clj-meta-result
                          [:compile-receipt
                           :bytecode-artifact
                           :artifact-hash])
   :clj-meta-status (:status clj-meta-result)
   :clj-meta-mode (:mode clj-meta-result)
   :clj-meta-diagnostics (:diagnostics clj-meta-result)
   :clj-meta-compile-receipt (:compile-receipt clj-meta-result)
   :clj-meta-determinism (get-in clj-meta-result
                                 [:compile-receipt :determinism])
   :clj-meta-fallback (get-in clj-meta-result
                              [:compile-receipt :fallback])
   :clj-meta-strict (get-in clj-meta-result
                            [:compile-receipt :strict])
   :clj-meta-verified (get-in clj-meta-result
                              [:compile-receipt :verified-compile])
   :value (:value clj-meta-result)
   :stage15-control stage15-control})

(defn px-runtime-row
  [{:keys [source-hash ast-hash runtime-artifact run-plan source-execution]}]
  (let [status (if source-execution
                 (:status source-execution)
                 :failed)]
    (cond-> {:kind :px-runtime
             :status status
             :reason (if (= :ok status)
                       :px-runtime-source-execution-ok
                       (or (:reason source-execution)
                           (:reason run-plan)))
             :source-hash source-hash
             :ast-hash ast-hash
             :artifact runtime-artifact
             :run-plan run-plan
             :source-execution source-execution
             :run-result (:run-result source-execution)
             :mirror-result (:mirror-result source-execution)}
      (= :ok status)
      (assoc :value (:value source-execution))

      (not= :ok status)
      (assoc :error
             (some-> (:error source-execution)
                     (update :class #(if (= :import-module-cycle %)
                                       :import-cycle
                                       %)))))))

(defn pnix-mirror-row
  [{:keys [source-hash ast-hash px-runtime]}]
  (let [runtime-receipt (get-in px-runtime
                                [:source-execution :pnix-mirror-receipt])]
    (cond
      (and (= :ok (:status px-runtime))
           (= "ok" (get runtime-receipt "status")))
      {:kind :pnix-mirror
       :status :ok
       :reason :pnix-mirror-from-runtime-receipt
       :source-hash source-hash
       :ast-hash ast-hash
       :px-runtime-status (:status px-runtime)
       :runtime-receipt runtime-receipt
       :value (get runtime-receipt "value")}

      (= :ok (:status px-runtime))
      {:kind :pnix-mirror
       :status :failed
       :reason :pnix-mirror-runtime-receipt-missing
       :source-hash source-hash
       :ast-hash ast-hash
       :px-runtime-status (:status px-runtime)
       :runtime-receipt runtime-receipt}

      :else
      {:kind :pnix-mirror
       :status :failed
       :reason (if (:artifact px-runtime)
                 :pnix-mirror-awaits-px-runtime-execution
                 :pnix-mirror-awaits-px-runtime-artifact)
       :source-hash source-hash
       :ast-hash ast-hash
       :px-runtime-status (:status px-runtime)
       :runtime-receipt runtime-receipt})))

(defn cross-mirror-verdict-row
  [{:keys [source-hash ast-hash eval-result clj-meta-result clojure-mirror
           px-runtime pnix-mirror]}]
  (let [eval-value (:value eval-result)
        clj-meta-value (:value clj-meta-result)
        px-value (:value px-runtime)
        pnix-value (:value pnix-mirror)
        host-agrees? (and (= :ok (:status eval-result))
                          (= :ok (:status clj-meta-result))
                          (= eval-value clj-meta-value))]
    (cond
      (not host-agrees?)
      {:kind :cross-mirror-verdict
       :status :rejected
       :reason :host-mirror-disagrees-before-px-runtime
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :disagree
       :eval-status (:status eval-result)
       :clj-meta-status (:status clj-meta-result)
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)}

      (not= :ok (:status px-runtime))
      {:kind :cross-mirror-verdict
       :status :failed
       :reason (:reason px-runtime)
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :failed
       :host-mirror-agrees? true
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)}

      (not= eval-value px-value)
      {:kind :cross-mirror-verdict
       :status :rejected
       :reason :px-runtime-value-mismatch
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :disagree
       :host-mirror-agrees? true
       :eval-value eval-value
       :px-runtime-value px-value
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)}

      (not= :ok (:status pnix-mirror))
      {:kind :cross-mirror-verdict
       :status :failed
       :reason (:reason pnix-mirror)
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :failed
       :host-mirror-agrees? true
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)}

      (not= eval-value pnix-value)
      {:kind :cross-mirror-verdict
       :status :rejected
       :reason :pnix-mirror-value-mismatch
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :disagree
       :host-mirror-agrees? true
       :eval-value eval-value
       :pnix-mirror-value pnix-value
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)}

      :else
      {:kind :cross-mirror-verdict
       :status :ok
       :reason :mirrors-agree
       :source-hash source-hash
       :ast-hash ast-hash
       :equivalence :agree
       :host-mirror-agrees? true
       :clojure-mirror-status (:clj-meta-status clojure-mirror)
       :px-runtime-status (:status px-runtime)
       :pnix-mirror-status (:status pnix-mirror)})))

(defn run-mirror
  "Single pnix runtime mirror entrypoint. Callers provide the already-computed
  host parse/eval/lowering lanes; this function owns the mirror-side facets and
  returns every mirror row from one place."
  [{:keys [source source-hash ast-hash eval-result lowering-result
           clj-meta-result facets import-modules]}]
  (let [facets (or facets default-facets)
        stage15-control (stage15/control-plan)
        clojure-mirror (clojure-mirror-row
                        {:source-hash source-hash
                         :ast-hash ast-hash
                         :lowered lowering-result
                         :clj-meta-result clj-meta-result
                         :stage15-control stage15-control})
        runtime-artifact (px-runtime/select-runtime-artifact)
        runtime-plan (px-runtime/runtime-run-plan runtime-artifact)
        runtime-execution (px-runtime/runtime-source-execution source import-modules)
        px-runtime-result (px-runtime-row {:source-hash source-hash
                                           :ast-hash ast-hash
                                           :runtime-artifact runtime-artifact
                                           :run-plan runtime-plan
                                           :source-execution runtime-execution})
        pnix-mirror (pnix-mirror-row {:source-hash source-hash
                                      :ast-hash ast-hash
                                      :px-runtime px-runtime-result})
        cross-mirror-verdict (cross-mirror-verdict-row
                              {:source-hash source-hash
                               :ast-hash ast-hash
                               :eval-result eval-result
                               :clj-meta-result clj-meta-result
                               :clojure-mirror clojure-mirror
                               :px-runtime px-runtime-result
                               :pnix-mirror pnix-mirror})]
    {:kind :pnix-clj.mirror/run-mirror
     :facets facets
     :source-hash source-hash
     :ast-hash ast-hash
     :stage15-control stage15-control
     :runtime-artifact runtime-artifact
     :runtime-plan runtime-plan
     :runtime-execution runtime-execution
     :clojure-mirror clojure-mirror
     :px-runtime px-runtime-result
     :pnix-mirror pnix-mirror
     :cross-mirror-verdict cross-mirror-verdict
     :facet-statuses {:host/eval (:status eval-result)
                      :host/clj-meta (:status clj-meta-result)
                      :inner/stage15-control (:status stage15-control)
                      :inner/px-runtime (:status px-runtime-result)
                      :inner/pnix-mirror (:status pnix-mirror)
                      :cross/value-agreement (:status cross-mirror-verdict)}}))
