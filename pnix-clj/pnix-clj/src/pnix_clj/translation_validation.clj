(ns pnix-clj.translation-validation
  "Per-compilation translation validation: for each source-form -> clj-meta bytecode lowering, verify semantic preservation for THAT run (Necula/Pnueli style), not whole-compiler correctness."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :translation-validation
   :product-runtime :forbidden
   :codegen-product-lane :forbidden
   :whole-compiler-correctness-claim :forbidden
   :mutation :forbidden
   :admission :forbidden
   :default-on-uncertainty :held
   :allowed-output :equivalence-verdict})

(def validator-catalog
  [{:id :parse-source
    :validate-form "Validate(source, parse-result)"
    :candidate :ast
    :acceptance :parse-result-ok
    :default-on-uncertainty :held
    :residual-tcb [:parser :readerless-tokenizer]}
   {:id :evaluator-oracle
    :validate-form "Validate(source, evaluator-result, oracle-result)"
    :candidate :evaluator-value
    :acceptance :oracle-value-equality
    :default-on-uncertainty :held
    :failure-status :rejected
    :residual-tcb [:oracle-fixture :evaluator]}
   {:id :lowering-clj-meta
    :validate-form "Validate(ast, lowered-form, clj-meta-result)"
    :candidate :lowered-clojure-form
    :acceptance :evaluator-clj-meta-value-equality
    :default-on-uncertainty :held
    :failure-status :rejected
    :residual-tcb [:lowering-policy :clj-meta-compiler :jvm]}
   {:id :compile-receipt
    :validate-form "Validate(lowered-form, clj-meta-compile-receipt)"
    :candidate :compiled-classfiles
    :acceptance :determinism-and-strict-evidence
    :default-on-uncertainty :held
    :residual-tcb [:clj-meta-form-proof :bytecode-artifact-api]}
   {:id :px-runtime
    :validate-form "Validate(source, px-runtime-result)"
    :candidate :px-runtime-execution
    :acceptance :evaluator-px-value-equality
    :default-on-uncertainty :held
    :failure-status :rejected
    :residual-tcb [:px-runtime-artifact :runtime-json-bridge]}
   {:id :pnix-mirror
    :validate-form "Validate(px-runtime-result, pnix-mirror-receipt)"
    :candidate :pnix-mirror-row
    :acceptance :evaluator-pnix-mirror-value-equality
    :default-on-uncertainty :held
    :failure-status :rejected
    :residual-tcb [:pnix-mirror-runtime-receipt]}
   {:id :cross-mirror
    :validate-form "Validate(clojure-mirror, px-runtime, pnix-mirror)"
    :candidate :cross-mirror-verdict
    :acceptance :host-and-pnix-mirror-agree
    :default-on-uncertainty :held
    :failure-status :rejected
    :residual-tcb [:shared-parser :shared-value-normalization]}
   {:id :stage15-execution
    :validate-form "Validate(stage15-command-plan, stage15-execution-report)"
    :candidate :clj-meta-stage15-commands
    :acceptance :selected-commands-exit-zero
    :default-on-uncertainty :held
    :residual-tcb [:clj-meta-deps :subprocess-environment]}
   {:id :external-live-oracle
    :validate-form "Validate(source, pnix-result, external-nix-json)"
    :candidate :external-reference-result
    :acceptance :external-json-value-equality
    :default-on-uncertainty :skipped
    :failure-status :rejected
    :residual-tcb [:nix-instantiate :json-normalization]}])

(defn- status
  [ok? failed?]
  (cond
    ok? :ok
    failed? :failed
    :else :rejected))

(defn- sample-rows
  [receipt]
  [{:validator :parse-source
    :status (if (= :ok (get-in receipt [:receipts 0 :status])) :ok :failed)
    :evidence {:parse-status (get-in receipt [:receipts 0 :status])
               :ast-hash (:ast-hash receipt)}}
   {:validator :evaluator-oracle
    :status (status (= (get-in receipt [:eval-result :value])
                       (get-in receipt [:oracle-result :value]))
                    (not= :ok (get-in receipt [:oracle-result :status])))
    :evidence {:evaluator-status (get-in receipt [:eval-result :status])
               :oracle-status (get-in receipt [:oracle-result :status])}}
   {:validator :lowering-clj-meta
    :status (status (= (get-in receipt [:eval-result :value])
                       (get-in receipt [:clj-meta-result :value]))
                    (not= :ok (get-in receipt [:clj-meta-result :status])))
    :evidence {:lowering-status (get-in receipt [:receipts 2 :status])
               :clj-meta-status (get-in receipt [:clj-meta-result :status])}}
   {:validator :compile-receipt
    :status (if (and (= :ok (get-in receipt [:clj-meta-result
                                             :compile-receipt
                                             :determinism
                                             :status]))
                     (= :ok (get-in receipt [:clj-meta-result
                                             :compile-receipt
                                             :bytecode-artifact
                                             :status])))
              :ok
              :held)
    :evidence {:determinism-status (get-in receipt [:clj-meta-result
                                                   :compile-receipt
                                                   :determinism
                                                   :status])
               :bytecode-status (get-in receipt [:clj-meta-result
                                                :compile-receipt
                                                :bytecode-artifact
                                                :status])}}
   {:validator :px-runtime
    :status (status (= (get-in receipt [:eval-result :value])
                       (get-in receipt [:px-runtime :value]))
                    (not= :ok (get-in receipt [:px-runtime :status])))
    :evidence {:px-runtime-status (get-in receipt [:px-runtime :status])}}
   {:validator :pnix-mirror
    :status (status (= (get-in receipt [:eval-result :value])
                       (get-in receipt [:pnix-mirror :value]))
                    (not= :ok (get-in receipt [:pnix-mirror :status])))
    :evidence {:pnix-mirror-status (get-in receipt [:pnix-mirror :status])}}
   {:validator :cross-mirror
    :status (if (and (= :ok (get-in receipt [:cross-mirror-verdict :status]))
                     (= :agree (get-in receipt [:cross-mirror-verdict
                                               :equivalence])))
              :ok
              :held)
    :evidence (select-keys (:cross-mirror-verdict receipt)
                           [:status :reason :equivalence])}])

(defn report
  []
  (let [receipt (pnix/verify-source {:source-id :translation-validation/literal-42
                                  :source "42"
                                  :oracle-result {:status :ok
                                                  :authority :inline-tv-smoke
                                                  :value 42}})
        rows (sample-rows receipt)
        held-or-rejected (remove #(= :ok (:status %)) rows)
        canonical {:validators validator-catalog
                   :sample-rows rows}]
    {:kind :pnix-translation-validation-report
     :schema :pnix-clj.translation-validation-report.v0
     :status (if (seq held-or-rejected) :failed :ok)
     :reason (if (seq held-or-rejected)
               :translation-validator-failed
               :translation-validators-framed)
     :policy :validate-source-candidate-default-held
     :validator-count (count validator-catalog)
     :validators validator-catalog
     :sample-source "42"
     :sample-receipt-status (:status receipt)
     :sample-rows rows
     :first-held-or-rejected (first held-or-rejected)
     :receipt-hash (hash/data-hash canonical)}))

(defn -main
  [& _]
  (let [{:keys [status validator-count receipt-hash]} (report)]
    (println (format "pnix-clj translation validation: status=%s validators=%d hash=%s"
                     (name status) validator-count receipt-hash))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
