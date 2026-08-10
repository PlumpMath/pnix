(ns pnix-clj.purity
  "§9 — purity / determinism as EVENTS (spine BUILD 5th; see docs/SPINE_ROADMAP.md).

  ★Research-verified stance (Nakajima 2026; Build Systems à la Carte): do NOT
  assume determinism -- WITNESS it by ACTUAL re-run and diff. A deterministic
  fold over a log is not automatically byte-identical, and content-addressing a
  nondeterministic effect does not make replay deterministic. So `purity-check!`
  RE-RUNS the evaluation and compares result hashes; a divergence is caught as
  a VIOLATION event pinned to the FIRST divergent run -- which is the §15
  witness anchor -- and the check FAILS CLOSED.

  Runs are recorded as §5 events (:purity/run / :purity/violation), pinned to
  the §8 snapshot, so determinism is durable evidence, not a transient assert."
  (:require [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :core
   :scope :purity-determinism-event-spine
   :role :rerun-based-determinism-witness
   :product-runtime :allowed
   :semantic-authority :evidence-only
   :mutation :append-only-store-events
   :admission :fail-closed
   :determinism :witnessed-by-rerun
   :allowed-output :purity-event-or-violation-report})

(defn- result-of
  [source]
  (let [eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        r (eval-source source)]
    {:status (:status r)
     :result-hash (when (= :ok (:status r)) (hash/data-hash (:value r)))}))

(defn purity-check!
  "Re-run `source` `runs` times and WITNESS determinism by comparing result
  hashes. On agreement: append a :purity/run event and return {:status :ok
  :result-hash}. On divergence: append a :purity/violation event pinned to the
  FIRST divergent run and return {:status :failed :reason :nondeterministic}
  (fail closed). `store` (a §5 log) and `snapshot` (§8 pin) are optional."
  ([source] (purity-check! source {}))
  ([source {:keys [runs store snapshot] :or {runs 3}}]
   (let [snap (or snapshot (snapshot/make-snapshot))
         results (mapv (fn [_] (result-of source)) (range runs))
         first-r (first results)
         divergent-idx (first (keep-indexed
                               (fn [i r] (when (not= r first-r) i))
                               results))
         source-hash (hash/sha256 source)]
     (if (nil? divergent-idx)
       (let [payload {:source-hash source-hash
                      :result-hash (:result-hash first-r)
                      :runs runs
                      :snapshot/id (:snapshot/id snap)}]
         (when store (store/append! store :purity/run payload))
         {:status :ok :result-hash (:result-hash first-r)
          :runs runs :snapshot/id (:snapshot/id snap)})
       ;; nondeterminism: pin the first divergent run as the violation anchor
       (let [payload {:source-hash source-hash
                      :first-divergent-run divergent-idx
                      :result-hashes (mapv :result-hash results)
                      :snapshot/id (:snapshot/id snap)}]
         (when store (store/append! store :purity/violation payload))
         (err/failed :evidence
                     :nondeterministic
                     {:first-divergent-run divergent-idx
                      :result-hashes (mapv :result-hash results)}))))))

(defn mutation-isolation!
  "A result captured under a snapshot must NOT change after later commits to
  unrelated state (referential transparency). Re-evaluates before and after an
  intervening store mutation; returns {:status :ok|:held}."
  [source snapshot]
  (let [before (snapshot/resolve-under-snapshot source snapshot)
        scratch (store/open-store)]
    (dotimes [i 5] (store/append! scratch :noise {:i i}))   ; later commits
    (let [after (snapshot/resolve-under-snapshot source snapshot)]
      (if (= (:value before) (:value after))
        {:status :ok :value (:value before)}
        (err/failed :evidence
                    :mutation-leaked
                    {:before (:value before) :after (:value after)})))))

(defn threaded-stress
  "Evaluate `source` concurrently from `n` threads; every result must agree (no
  shared-state race). Returns {:status :ok|:held}."
  [source n]
  (let [results (->> (range n)
                     (mapv (fn [_] (future (:result-hash (result-of source)))))
                     (mapv deref))]
    (if (apply = results)
      {:status :ok :threads n :result-hash (first results)}
      (err/failed :evidence :threaded-nondeterminism {:results results}))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [log (store/open-store)
        snap (snapshot/make-snapshot)
        pc (purity-check! "let x = 40; in x + 2" {:runs 5 :store log :snapshot snap})
        mi (mutation-isolation! "1 + 2 * 3" snap)
        ts (threaded-stress "builtins.foldl' (a: b: a + b) 0 [ 1 2 3 4 5 ]" 8)
        events (store/events-of log :purity/run)
        rows [{:id :repeated-eval-deterministic :ok? (= :ok (:status pc))}
              {:id :purity-run-recorded-as-event :ok? (= 1 (count events))}
              {:id :run-pinned-to-snapshot
               :ok? (= (:snapshot/id snap) (get-in (first events) [:payload :snapshot/id]))}
              {:id :mutation-isolation :ok? (= :ok (:status mi))}
              {:id :threaded-determinism :ok? (= :ok (:status ts))}
              {:id :log-chain-intact :ok? (= :intact (:status (store/verify-chain log)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-purity-report
              :schema :pnix-clj.purity-report.v0
              :policy :determinism-witnessed-by-rerun-recorded-as-events-fail-closed
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
    (println (format "pnix-clj purity: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
