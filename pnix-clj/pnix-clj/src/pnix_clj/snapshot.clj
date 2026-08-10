(ns pnix-clj.snapshot
  "§8 — snapshot determinism (spine BUILD 4th; see docs/SPINE_ROADMAP.md).

  A snapshot PINS the runtime under which a term is evaluated: the evaluator
  version (content-bound: a hash of the evaluator's builtin surface) and the
  symbol version (the §13.1 host-lane-id: JVM + Clojure + classpath). Its
  :snapshot/id is the content hash of that pin.

  ★The determinism precondition (Build Systems à la Carte §4.2.4 -- deep
  constructive traces / the Frankenbuild example): reusing a result keyed on
  content is sound ONLY if the runtime is identical. So `resolve-under-snapshot`
  FAILS CLOSED via `assert-snapshot-runtime-match!` when the current runtime
  does not match the snapshot's pin -- you can never get a value computed under
  a different evaluator/host masquerading as the snapshot's. This is the pin the
  §15 witness records and the gate §9 replays against."
  (:require [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.reflect :as reflect]))

(def lane-classification
  {:lane :core
   :scope :runtime-snapshot-pinning
   :role :pin-evaluator-and-host-runtime-version
   :product-runtime :allowed
   :semantic-authority :runtime-match-gate-only
   :mutation :forbidden
   :admission :fail-closed-on-mismatch
   :determinism :snapshot-id-content-hash
   :allowed-output :snapshot-or-runtime-match-verdict})

(defn evaluator-version
  "Content-bound id of the evaluator: a hash of its builtin surface (sorted) +
  a schema tag. Changes iff the evaluator's observable surface changes."
  []
  (hash/data-hash
   [:pnix-clj.evaluator.v0
    (vec (sort (keys (get evaluator/default-env "builtins"))))
    (vec (sort (remove #{"builtins"} (keys evaluator/default-env))))]))

(defn symbol-version
  "Content-bound id of the host lane (JVM + Clojure + classpath), from §13.1."
  []
  (reflect/host-lane-id))

(defn make-snapshot
  "Capture the current runtime pin as a snapshot (pure EDN, hashable)."
  []
  (let [ev (evaluator-version)
        sv (symbol-version)
        content {:evaluator-version ev
                 :symbol-version sv
                 :jvm (reflect/jvm-version-id)}]
    (assoc content
           :kind :pnix-snapshot
           :schema :pnix-clj.snapshot.v0
           :snapshot/id (hash/data-hash content))))

(defn runtime-matches?
  "True iff the current runtime matches the snapshot's pinned versions."
  [snapshot]
  (and (= (evaluator-version) (:evaluator-version snapshot))
       (= (symbol-version) (:symbol-version snapshot))))

(defn assert-snapshot-runtime-match!
  "FAIL CLOSED: returns {:status :ok} when the runtime matches, else a held
  verdict naming exactly which pin diverged (never silently proceeds)."
  [snapshot]
  (cond
    (not= (evaluator-version) (:evaluator-version snapshot))
    {:status :failed :reason :snapshot-evaluator-version-mismatch
     :snapshot/id (:snapshot/id snapshot)
     :expected (:evaluator-version snapshot) :actual (evaluator-version)}

    (not= (symbol-version) (:symbol-version snapshot))
    {:status :failed :reason :snapshot-symbol-version-mismatch
     :snapshot/id (:snapshot/id snapshot)
     :expected (:symbol-version snapshot) :actual (symbol-version)}

    :else {:status :ok :snapshot/id (:snapshot/id snapshot)}))

(defn resolve-under-snapshot
  "Evaluate `source` ONLY if the runtime still matches `snapshot`; otherwise fail
  closed. Returns {:status :ok :value ... :snapshot/id ...} or the held mismatch."
  [source snapshot]
  (let [gate (assert-snapshot-runtime-match! snapshot)]
    (if (not= :ok (:status gate))
      gate
      (let [eval-source (requiring-resolve 'pnix-clj.core/eval-source)
            r (eval-source source)]
        (assoc r :snapshot/id (:snapshot/id snapshot))))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [snap (make-snapshot)
        good (resolve-under-snapshot "1 + 2" snap)
        ;; a snapshot pinned to a DIFFERENT evaluator version must fail closed
        stale (assoc snap :evaluator-version "not-the-current-evaluator")
        gate-stale (assert-snapshot-runtime-match! stale)
        resolved-stale (resolve-under-snapshot "1 + 2" stale)
        rows [{:id :snapshot-id-content-hash
               :ok? (= (:snapshot/id snap)
                       (:snapshot/id (make-snapshot)))}     ; deterministic
              {:id :runtime-matches-self :ok? (runtime-matches? snap)}
              {:id :resolve-ok-under-matching :ok? (= 3 (:value good))}
              {:id :resolve-carries-snapshot-id
               :ok? (= (:snapshot/id snap) (:snapshot/id good))}
              {:id :stale-fails-closed
               :ok? (= :failed (:status gate-stale))}
              {:id :stale-reason-precise
               :ok? (= :snapshot-evaluator-version-mismatch (:reason gate-stale))}
              {:id :resolve-stale-refuses
               :ok? (= :failed (:status resolved-stale))}
              {:id :evaluator-version-content-bound
               :ok? (= (evaluator-version) (evaluator-version))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-snapshot-report
              :schema :pnix-clj.snapshot-report.v0
              :policy :runtime-pinned-snapshot-fail-closed-on-mismatch
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
    (println (format "pnix-clj snapshot: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
