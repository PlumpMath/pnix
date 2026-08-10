(ns pnix-clj.clj-meta-executor
  "Direct pnix-lowered-form execution through the Clojure host mechanism.

  This namespace performs the requested computation. It does not own service
  policy, proof admission, or semantic meaning, and execution is never
  conditioned on a compile/proof receipt. Shared PNIX meaning remains owned by
  pnix-meta .px modules."
  (:require [pnix-clj.interop :as interop]
            [pnix-clj.lowering :as lowering]))

(def lane-classification
  {:lane :core
   :scope :clj-meta-host-execution
   :role :host-mechanism-executor
   :product-runtime :allowed
   :semantic-authority :pnix-meta-px
   :service-policy-owner false
   :proof-receipt-required false
   :allowed-output :runtime-result})

(def ^:private host-execution-interop
  (interop/interop-meta {:direction :pnix-lowered-form->clj-meta-execution
                         :effect-class :host-compile
                         :loss-status :lossless}))

(defn- eval-form
  []
  (requiring-resolve 'pnix.clj-meta.compiler/eval-form))

(defn- normalized-host-error
  [^Throwable t fallback-phase fallback-class]
  (let [data (ex-data t)
        nested (when (map? (:error data)) (:error data))
        class-candidate (or (:class nested) (:reason data) (:error-class data))
        phase-candidate (or (:phase nested) (:phase data))]
    {:phase (if (keyword? phase-candidate) phase-candidate fallback-phase)
     :class (if (keyword? class-candidate) class-candidate fallback-class)
     :evidence (cond-> {}
                 (keyword? (:operator data)) (assoc :operator (:operator data)))}))

(defn eval-lowered
  "Execute one lowered Clojure form directly. Proof APIs are a separate lane."
  [form]
  (let [attach #(interop/attach-witness :clj-meta-host-execution
                                        host-execution-interop
                                        form
                                        %)]
    (try
      (attach {:status :ok
               :interop host-execution-interop
               :value (lowering/force-normal ((eval-form) form))
               :mode :host-execution-direct
               :diagnostics []
               :execution-api 'pnix.clj-meta.compiler/eval-form})
      (catch Throwable t
        (let [symbol (:clojure.error/symbol (ex-data t))
              unavailable? (= 'pnix.clj-meta.compiler/eval-form symbol)
              error (if unavailable?
                      {:phase :host-compile
                       :class :clj-meta-unavailable
                       :evidence {:symbol symbol}}
                      (normalized-host-error t
                                             :host-compile
                                             :clj-meta-eval-failed))]
          (attach {:status :failed
                   :interop host-execution-interop
                   :reason (:class error)
                   :error error}))))))
