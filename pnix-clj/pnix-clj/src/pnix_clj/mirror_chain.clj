(ns pnix-clj.mirror-chain
  "§6.6-6.7 — mirror DRIFT events + repeated-run chain-convergence stability
  (spine BUILD 7th; see docs/SPINE_ROADMAP.md).

  The per-run cross-mirror verdict (mirror.clj / run-source) checks that the
  four substrates AGREE on one value in a single run. This adds the TEMPORAL
  axis: run the SAME source repeatedly and require every result hash to match
  the first (self-evaluation stability). A divergence is recorded as a §5
  :mirror/chain-drift event pinned to the first divergent run -- the same
  first-divergent-anchor discipline as §9, feeding the §15 witness.

  Convergence here is over REPEATED runs (is the mirror stable in time?),
  complementing the CROSS-LANE collapse (do the substrates agree?)."
  (:require [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :core
   :scope :temporal-mirror-chain-evidence
   :role :repeated-run-convergence-and-drift-events
   :product-runtime :allowed
   :semantic-authority :evidence-only
   :mutation :append-only-mirror-events
   :admission :fail-closed-on-drift
   :determinism :chain-convergence-required
   :allowed-output :mirror-chain-report-or-drift-event})

(defn run-result-hash
  "Run `source` through the canonical entrypoint and hash its collapsed value
  (nil-status → a stable :held marker so held sources chain too)."
  [source]
  (let [run-source (requiring-resolve 'pnix-clj.core/run-source)
        row (run-source source)
        cross (:cross-mirror-verdict row)]
    {:status (:status row)
     :cross (:equivalence cross)
     :value-hash (when (= :accepted (:status row))
                   (hash/data-hash (get-in row [:eval-result :value])))}))

(defn converge?
  "True iff two runs of `source` produced the same result."
  [source]
  (= (run-result-hash source) (run-result-hash source)))

(defn mirror-chain!
  "Run `source` `n` times; every result must equal the first (chain
  convergence). Records a :mirror/run event per run and, on divergence, a
  :mirror/chain-drift event pinned to the first divergent run. Returns
  {:status :ok|:failed :chain-converged? :runs :result}."
  ([source] (mirror-chain! source {}))
  ([source {:keys [runs store] :or {runs 3}}]
   (let [results (mapv (fn [_] (run-result-hash source)) (range runs))
         first-r (first results)
         divergent-idx (first (keep-indexed
                               (fn [i r] (when (not= r first-r) i)) results))
         source-hash (hash/sha256 source)]
     (when store
       (store/append! store :mirror/run
                      {:source-hash source-hash :result first-r :runs runs}))
     (if (nil? divergent-idx)
       {:status :ok :chain-converged? true :runs runs :result first-r}
       (do (when store
             (store/append! store :mirror/chain-drift
                            {:source-hash source-hash
                             :first-divergent-run divergent-idx
                             :results results}))
           (err/failed :evidence
                       :mirror-chain-drift
                       {:chain-converged? false
                        :first-divergent-run divergent-idx}))))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [log (store/open-store)
        c1 (mirror-chain! "let x = 40; in x + 2" {:runs 5 :store log})
        c2 (mirror-chain! "({ a }@args: a + args.a) { a = 21; }" {:runs 4 :store log})
        c3 (mirror-chain! "(builtins.tryEval (builtins.throw \"x\")).success"
                          {:runs 3 :store log})
        runs (store/events-of log :mirror/run)
        drifts (store/events-of log :mirror/chain-drift)
        rows [{:id :arith-chain-converges :ok? (:chain-converged? c1)}
              {:id :pattern-lambda-chain-converges :ok? (:chain-converged? c2)}
              {:id :tryeval-select-chain-converges :ok? (:chain-converged? c3)}
              {:id :runs-recorded-as-events :ok? (= 3 (count runs))}
              {:id :no-drift-on-deterministic :ok? (zero? (count drifts))}
              {:id :converge-is-reflexive
               :ok? (converge? "1 + 2 * 3")}
              {:id :chain-log-intact
               :ok? (= :intact (:status (store/verify-chain log)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-mirror-chain-report
              :schema :pnix-clj.mirror-chain-report.v0
              :policy :repeated-run-chain-convergence-drift-as-events
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash (:rows body)))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj mirror-chain: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
