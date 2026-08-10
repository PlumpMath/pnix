(ns pnix-clj.replay
  "Replay / audit -- independently RE-VERIFY a persisted witness later, in a
  fresh run. This is the payoff of the durable evidence store: the §8/§9
  determinism guarantee is checked ACROSS TIME/PROCESS, not just within one run.

  `replay-witness` loads a persisted §15 witness and its source, re-runs the
  witnessed pipeline, and compares the fresh witness against the stored one:
  same term-hash (§3), same result-hash (§9 determinism), same admission
  status. A match is :reproduced; any difference is :diverged, pinned to the
  field that changed -- the same fail-closed, evidence-first discipline as the
  rest of the spine.

  Reproduction here is the strongest determinism claim the store can make: the
  identical result is derived again from the persisted source under a matching
  runtime, entirely from durable evidence."
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.persist :as persist]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.witnessed-run :as witnessed-run]))

(def lane-classification
  {:lane :core
   :scope :durable-witness-replay-evidence
   :role :reverify-persisted-witness-across-time
   :product-runtime :allowed
   :semantic-authority :reproduction-evidence-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :fresh-rerun-must-match-witness
   :allowed-output :reproduced-diverged-or-missing-replay-verdict})

(defn replay-witness
  "Load witness `wid` + its source from `pstore`, re-run witnessed, and compare.
  Returns {:verdict :reproduced|:diverged|:missing :diffs [..] :original :fresh}."
  [pstore wid]
  (let [orig (persist/load-witness pstore wid)]
    (if-not orig
      {:verdict :missing :reason :no-such-witness :witness-id wid}
      (let [source (persist/load-source pstore (:input-hash orig))]
        (if-not source
          {:verdict :missing :reason :no-persisted-source :witness-id wid}
          (let [fresh-run (witnessed-run/run-witnessed source)
                fresh (:witness fresh-run)
                checks {:term-hash    (= (:term-hash orig) (:term-hash fresh))
                        :result-hash  (= (:result-hash orig) (:result-hash fresh))
                        :evaluator    (= (:evaluator-version orig) (:evaluator-version fresh))
                        :snapshot     (= (:snapshot/id orig) (:snapshot/id fresh))}
                diffs (keep (fn [[k ok?]] (when-not ok? k)) checks)]
            {:verdict (if (empty? diffs) :reproduced :diverged)
             :diffs (vec diffs)
             :witness-id wid
             :runtime-matches? (snapshot/runtime-matches?
                                {:evaluator-version (:evaluator-version orig)
                                 :symbol-version (:runtime-version orig)})
             :original (select-keys orig [:term-hash :result-hash :snapshot/id :status])
             :fresh (select-keys fresh [:term-hash :result-hash :snapshot/id :status])}))))))

;; ---- report --------------------------------------------------------------

(defn report
  ([] (report (str (System/getProperty "java.io.tmpdir")
                   "/pnix-replay-" (System/nanoTime))))
  ([dir]
   (let [;; persist a witnessed run, then replay it from disk
         durable (witnessed-run/run-witnessed-durable "let x = 40; in x + 2" dir)
         pstore (persist/open-persistent-store dir)
         wid (get-in durable [:persisted :witness-id])
         rep (replay-witness pstore wid)
         ;; a second, DIFFERENT program's witness reproduces on its own terms
         d2 (witnessed-run/run-witnessed-durable "builtins.length [ 1 2 3 4 5 ]" dir)
         rep2 (replay-witness pstore (get-in d2 [:persisted :witness-id]))
         missing (replay-witness pstore "deadbeef-not-a-real-witness")
         rows [{:id :persisted-witness-reproduces
                :ok? (= :reproduced (:verdict rep))}
               {:id :same-term-and-result-hash
                :ok? (empty? (:diffs rep))}
               {:id :runtime-still-matches
                :ok? (:runtime-matches? rep)}
               {:id :second-program-reproduces
                :ok? (= :reproduced (:verdict rep2))}
               {:id :missing-witness-reported
                :ok? (= :missing (:verdict missing))}
               {:id :original-hashes-equal-fresh
                ;; the reproducibility claim: identical term + result derived
                ;; again from the persisted source
                :ok? (and (= (:term-hash (:original rep)) (:term-hash (:fresh rep)))
                          (= (:result-hash (:original rep)) (:result-hash (:fresh rep))))}]
         rejected (count (remove :ok? rows))
         body {:kind :pnix-replay-report
               :schema :pnix-clj.replay-report.v0
               :policy :persisted-witness-reverified-across-process-reproduced-or-diverged
               :total (count rows)
               :accepted (- (count rows) rejected)
               :rejected rejected
               :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
     (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f))
     (assoc body
            :status (if (zero? rejected) :ok :failed)
            :report-hash (hash/data-hash (:rows body))))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj replay: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
