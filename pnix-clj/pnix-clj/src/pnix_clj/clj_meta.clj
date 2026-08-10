(ns pnix-clj.clj-meta
  (:require [pnix-clj.interop :as interop]
            [pnix-clj.lowering :as lowering]))

(def lane-classification
  {:lane :core
   :scope :clj-meta-host-proof-interop
   :role :meta-circular-host-compile-eval-client
   :product-runtime :allowed
   :semantic-authority :requires-receipt
   :mutation :forbidden
   :admission :gated-by-compile-receipt
   :determinism :required
   :allowed-output :host-eval-result-with-compile-receipt})

;; This namespace is the host compile/eval interop client: it crosses the
;; boundary from a pnix-lowered Clojure form into clj-meta's compiler/evaluator
;; (the host proof lane) and brings back a host value. See clj-meta-separation.md.
(def ^:private host-compile-interop
  (interop/interop-meta {:direction :pnix-lowered-form->clj-meta-compile
                         :effect-class :host-compile
                         :loss-status :lossless}))

(defn- compile-form-star
  []
  (requiring-resolve 'pnix.clj-meta.compiler/compile-form*))

(defn- eval-form
  []
  (requiring-resolve 'pnix.clj-meta.compiler/eval-form))

(defn- form-proof-compile-receipt
  []
  (requiring-resolve 'pnix.clj-meta.form-proof/compile-receipt))

(defn- form-proof-determinism-receipt
  []
  (requiring-resolve 'pnix.clj-meta.form-proof/determinism-receipt))

(defn- kept-class-units-cap-var
  []
  (requiring-resolve 'pnix.clj-meta.compiler/*kept-class-units-cap*))

(defn- clear-kept-classes
  []
  (requiring-resolve 'pnix.clj-meta.compiler/clear-kept-classes!))

(def ^:dynamic ^:private *compile-proof-scope* :full)

(defn- normalized-host-error
  "Extract only stable pnix error identity from a host throwable. Native class,
  message, stack, and arbitrary ex-data never cross the boundary."
  [^Throwable t fallback-phase fallback-class]
  (let [data (ex-data t)
        nested (when (map? (:error data)) (:error data))
        class-candidate (or (:class nested) (:reason data) (:error-class data))
        phase-candidate (or (:phase nested) (:phase data))
        error-class (if (keyword? class-candidate)
                      class-candidate
                      fallback-class)
        phase (if (keyword? phase-candidate)
                phase-candidate
                (if (keyword? (:reason data)) :eval fallback-phase))]
    {:phase phase
     :class error-class
     :evidence (cond-> {}
                 (keyword? (:operator data)) (assoc :operator (:operator data)))}))

(defn- compile-receipt
  [form primary primary-value repeat repeat-value]
  (case *compile-proof-scope*
    :determinism
    ((form-proof-determinism-receipt)
     form primary primary-value repeat repeat-value)

    ((form-proof-compile-receipt)
     form primary primary-value repeat repeat-value lowering/force-normal)))

(defn eval-lowered
  "Evaluate a lowered Clojure form through clj-meta and keep compile diagnostics."
  [form]
  (let [capability (interop/check-capability (:effect-class host-compile-interop)
                                             interop/host-compile-capabilities)
        attach #(interop/attach-witness :clj-meta-host-compile
                                        host-compile-interop
                                        form
                                        %)]
    (if (not= :ok (:status capability))
      (attach {:status :failed
               :interop host-compile-interop
               :capability capability
               :reason (:reason capability)
               :error (:error capability)})
      (try
        (let [eval-form* (eval-form)
              compile-form* (compile-form-star)
              wrapper-form (list 'fn [] form)
              eval-form-value (lowering/force-normal (eval-form* form))
              primary (compile-form* wrapper-form)
              primary-value (lowering/force-normal ((:fn primary)))
              repeat (compile-form* wrapper-form)
              repeat-value (lowering/force-normal ((:fn repeat)))
              receipt (compile-receipt form primary primary-value repeat repeat-value)
              api-values-agree? (= eval-form-value primary-value)]
          (attach
           (if (and (= :ok (get-in receipt [:determinism :status]))
                    (not= false (get-in receipt
                                         [:strict :same-value-as-primary?]))
                    api-values-agree?)
             {:status :ok
              :interop host-compile-interop
              :capability capability
              :value eval-form-value
              :mode (:mode primary)
              :diagnostics (vec (:diagnostics primary))
              :execution-api 'pnix.clj-meta.compiler/eval-form
              :evidence-apis ['pnix.clj-meta.compiler/compile-form*
                              'pnix.clj-meta.form-proof/compile-receipt]
              :api-values-agree? api-values-agree?
              :compile-receipt receipt}
             {:status :failed
              :interop host-compile-interop
              :capability capability
              :reason (cond
                        (not api-values-agree?)
                        :clj-meta-eval-form-value-mismatch

                        (= false (get-in receipt
                                         [:strict :same-value-as-primary?]))
                        :clj-meta-strict-value-mismatch

                        :else
                        :clj-meta-determinism-mismatch)
              :error {:phase :host-compile
                      :class (cond
                               (not api-values-agree?)
                               :clj-meta-eval-form-value-mismatch

                               (= false (get-in receipt
                                                [:strict :same-value-as-primary?]))
                               :clj-meta-strict-value-mismatch

                               :else
                               :clj-meta-determinism-mismatch)
                      :evidence {:api-values-agree? api-values-agree?
                                 :determinism-status (get-in receipt
                                                             [:determinism :status])}}
              :value eval-form-value
              :mode (:mode primary)
              :diagnostics (vec (:diagnostics primary))
              :execution-api 'pnix.clj-meta.compiler/eval-form
              :evidence-apis ['pnix.clj-meta.compiler/compile-form*
                              'pnix.clj-meta.form-proof/compile-receipt]
              :api-values-agree? api-values-agree?
              :compile-receipt receipt})))
        (catch Throwable t
          (let [symbol (:clojure.error/symbol (ex-data t))
                unavailable? (contains? #{'pnix.clj-meta.compiler/eval-form
                                          'pnix.clj-meta.compiler/compile-form*}
                                        symbol)
                error (if unavailable?
                        {:phase :host-compile
                         :class :clj-meta-unavailable
                         :evidence {:symbol symbol}}
                        (normalized-host-error t
                                               :host-compile
                                               :clj-meta-eval-failed))
                reason (:class error)]
            (attach {:status :failed
                     :interop host-compile-interop
                     :capability capability
                     :reason reason
                     :error error})))))))

(defn eval-lowered-bounded
  "Evaluate a data-producing proof form while bounding clj-meta's retained
  generated-class units. This is for finite report batches whose compiled
  functions do not escape; product artifacts use eval-lowered's normal
  lifecycle."
  [form kept-class-units-cap]
  (when-not (and (integer? kept-class-units-cap)
                 (pos? kept-class-units-cap))
    (throw (ex-info "kept class units cap must be a positive integer"
                    {:cap kept-class-units-cap})))
  (try
    (with-bindings {(kept-class-units-cap-var) kept-class-units-cap}
      (eval-lowered form))
    (finally
      ((clear-kept-classes)))))

(defn eval-lowered-determinism-bounded
  "Evaluate a finite proof row with primary/repeat determinism evidence only.
  Full strict, bytecode-artifact, and verified-compile evidence stays in the
  ordinary eval-lowered aggregate path."
  [form kept-class-units-cap]
  (binding [*compile-proof-scope* :determinism]
    (eval-lowered-bounded form kept-class-units-cap)))
