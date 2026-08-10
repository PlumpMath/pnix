(ns pnix-clj.trust
  "Trust / provenance receipts: content-addressed artifact trust chain over the compiled + runtime evidence."
  (:require [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :trust-and-provenance-risk-report
   :role :common-mode-risk-and-claim-boundary-evidence
   :product-runtime :allowed
   :semantic-authority :risk-report-only
   :mutation :forbidden
   :admission :default-held-on-uncertainty
   :determinism :report-hash-required
   :allowed-output :trust-risk-report})

(defn common-mode-risk-report
  []
  (let [body {:risk :correlated-common-mode-failure
              :reference :knight-leveson-n-version-programming-risk
              :claim-boundary
              {:cross-lane-voting :reduces-regression-risk
               :not-claimed :independent-formal-proof
               :acceptance-rule :default-held-on-uncertainty}
              :shared-tcb
              [:pnix-parser
               :pnix-evaluator-semantics
               :lowering-policy
               :clj-meta-compiler
               :jvm-runtime
               :host-filesystem-for-explicit-imports]
              :mitigations
              [{:kind :independent-live-oracle
                :artifact :live-oracle
                :role :external-reference-nix-json-comparison}
               {:kind :generated-differential-gate
                :artifact :grammar-fuzzer
                :role :generated-positive-and-error-path-run-source-check}
               {:kind :dynamic-coverage
                :artifact :coverage
                :role :measures-which-semantics-the-corpus-exercises}
               {:kind :determinism-chain
                :artifact :determinism
                :role :repeated-parse-eval-hash-stability}
               {:kind :mirror-error-corpus
                :artifact :mirror-error
                :role :prevents-silent-negative-to-success-flips}]
              :residual-risk
              [:oracle-fixture-starvation
               :shared-parser-bugs
               :shared-host-runtime-assumptions
               :gaps-in-lazy-structure-semantics
               :store-and-string-context-frontier]}]
    (assoc body
           :kind :pnix-common-mode-risk-report
           :schema :pnix-clj.common-mode-risk-report.v0
           :status :ok
           :report-hash (hash/data-hash body))))

(defn report
  []
  (common-mode-risk-report))

(defn -main
  [& _]
  (let [{:keys [status report-hash]} (report)]
    (println (format "pnix-clj trust: common-mode-risk status=%s hash=%s"
                     (name status) report-hash))
    (shutdown-agents)))
